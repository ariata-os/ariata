//
//  NetworkManager.swift
//  Ariata
//
//  Handles all network communication with retry logic
//

import Foundation
import Combine

enum NetworkError: LocalizedError {
    case invalidURL
    case invalidToken
    case serverError(Int)
    case timeout
    case noConnection
    case decodingError
    case unknown(Error)
    
    var errorDescription: String? {
        switch self {
        case .invalidURL:
            return "Invalid API endpoint URL"
        case .invalidToken:
            return "Invalid device token (E002)"
        case .serverError(let code):
            return "Server error: \(code) (E003)"
        case .timeout:
            return "Network timeout (E001)"
        case .noConnection:
            return "No internet connection"
        case .decodingError:
            return "Failed to decode response"
        case .unknown(let error):
            return "Unknown error: \(error.localizedDescription)"
        }
    }
    
    var errorCode: String {
        switch self {
        case .timeout: return "E001"
        case .invalidToken: return "E002"
        case .serverError: return "E003"
        default: return "E000"
        }
    }
}

class NetworkManager: ObservableObject {
    static let shared = NetworkManager()
    
    private let session: URLSession
    private let timeout: TimeInterval = 30.0
    private var cancellables = Set<AnyCancellable>()
    
    @Published var isConnected: Bool = true
    @Published var lastError: NetworkError?
    
    private init() {
        let configuration = URLSessionConfiguration.default
        configuration.timeoutIntervalForRequest = timeout
        configuration.timeoutIntervalForResource = timeout
        configuration.waitsForConnectivity = true
        configuration.allowsCellularAccess = true
        
        self.session = URLSession(configuration: configuration)
    }
    
    // MARK: - Data Upload
    
    func uploadData<T: Encodable>(_ data: T, deviceToken: String, endpoint: URL) async throws -> UploadResponse {
        var request = URLRequest(url: endpoint)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue(deviceToken, forHTTPHeaderField: "X-Device-Token")
        
        // Encode data
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        request.httpBody = try encoder.encode(data)
        
        do {
            let (data, response) = try await session.data(for: request)
            
            guard let httpResponse = response as? HTTPURLResponse else {
                throw NetworkError.unknown(NSError(domain: "Invalid response", code: 0))
            }
            
            switch httpResponse.statusCode {
            case 200...299:
                return try JSONDecoder().decode(UploadResponse.self, from: data)
            case 401:
                throw NetworkError.invalidToken
            case 500...599:
                throw NetworkError.serverError(httpResponse.statusCode)
            default:
                throw NetworkError.unknown(NSError(domain: "HTTP \(httpResponse.statusCode)", code: httpResponse.statusCode))
            }
        } catch {
            if let urlError = error as? URLError {
                switch urlError.code {
                case .timedOut:
                    throw NetworkError.timeout
                case .notConnectedToInternet, .networkConnectionLost:
                    throw NetworkError.noConnection
                default:
                    throw NetworkError.unknown(error)
                }
            }
            
            if error is NetworkError {
                throw error
            }
            
            throw NetworkError.unknown(error)
        }
    }
    
    // MARK: - Device Verification
    
    struct StreamConfiguration {
        let enabled: Bool
        let initialSyncDays: Int
        let displayName: String
    }
    
    struct VerificationResponse {
        let success: Bool
        let configurationComplete: Bool
        let message: String?
        let configuredStreamCount: Int
        let streamConfigurations: [String: StreamConfiguration]
    }
    
    func verifyDeviceToken(endpoint: URL, deviceToken: String, deviceInfo: [String: Any]) async -> VerificationResponse {
        var request = URLRequest(url: endpoint)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue(deviceToken, forHTTPHeaderField: "X-Device-Token")
        request.timeoutInterval = 10.0
        
        do {
            // Encode device info
            request.httpBody = try JSONSerialization.data(withJSONObject: deviceInfo)
            
            let (data, response) = try await session.data(for: request)
            
            guard let httpResponse = response as? HTTPURLResponse else {
                lastError = .unknown(NSError(domain: "Invalid response", code: 0))
                return VerificationResponse(success: false, configurationComplete: false, message: nil, configuredStreamCount: 0, streamConfigurations: [:])
            }
            
            switch httpResponse.statusCode {
            case 200...299:
                // Parse response to get success and configuration status
                if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let success = json["success"] as? Bool {
                    let configurationComplete = json["configurationComplete"] as? Bool ?? false
                    let message = json["message"] as? String
                    let configuredStreamCount = json["configuredStreamCount"] as? Int ?? 0
                    
                    // Parse stream configurations
                    var streamConfigs: [String: StreamConfiguration] = [:]
                    if let configuration = json["configuration"] as? [String: Any],
                       let streams = configuration["streams"] as? [String: Any] {
                        for (streamKey, streamData) in streams {
                            if let stream = streamData as? [String: Any],
                               let enabled = stream["enabled"] as? Bool,
                               let initialSyncDays = stream["initialSyncDays"] as? Int,
                               let displayName = stream["displayName"] as? String {
                                streamConfigs[streamKey] = StreamConfiguration(
                                    enabled: enabled,
                                    initialSyncDays: initialSyncDays,
                                    displayName: displayName
                                )
                            }
                        }
                    }
                    
                    print("✅ Verification response:")
                    print("   Success: \(success)")
                    print("   Configuration complete: \(configurationComplete)")
                    print("   Configured streams: \(configuredStreamCount)")
                    print("   Stream configs: \(streamConfigs.keys.joined(separator: ", "))")
                    for (key, config) in streamConfigs {
                        print("     - \(key): enabled=\(config.enabled), initialSyncDays=\(config.initialSyncDays)")
                    }
                    print("   Message: \(message ?? "none")")
                    
                    return VerificationResponse(
                        success: success,
                        configurationComplete: configurationComplete,
                        message: message,
                        configuredStreamCount: configuredStreamCount,
                        streamConfigurations: streamConfigs
                    )
                }
                return VerificationResponse(success: true, configurationComplete: false, message: nil, configuredStreamCount: 0, streamConfigurations: [:])
            case 401:
                lastError = .invalidToken
                return VerificationResponse(success: false, configurationComplete: false, message: "Invalid token", configuredStreamCount: 0, streamConfigurations: [:])
            case 404:
                // Token not found in database
                lastError = .unknown(NSError(domain: "Device token not found. Please generate a new token in the web app.", code: 404))
                return VerificationResponse(success: false, configurationComplete: false, message: "Token not found", configuredStreamCount: 0, streamConfigurations: [:])
            default:
                lastError = .serverError(httpResponse.statusCode)
                return VerificationResponse(success: false, configurationComplete: false, message: "Server error", configuredStreamCount: 0, streamConfigurations: [:])
            }
        } catch {
            print("❌ Network request failed: \(error)")
            print("   URL: \(endpoint)")
            
            if let urlError = error as? URLError {
                switch urlError.code {
                case .timedOut:
                    lastError = .timeout
                case .notConnectedToInternet, .networkConnectionLost:
                    lastError = .noConnection
                case .cannotConnectToHost:
                    lastError = .unknown(NSError(domain: "Cannot connect to \(endpoint.absoluteString). Please check the URL and ensure the server is running.", code: urlError.code.rawValue))
                case .cannotFindHost:
                    lastError = .unknown(NSError(domain: "Cannot find host \(endpoint.host ?? ""). Please check the URL.", code: urlError.code.rawValue))
                default:
                    lastError = .unknown(error)
                }
            } else {
                lastError = .unknown(error)
            }
            return VerificationResponse(success: false, configurationComplete: false, message: "Network error", configuredStreamCount: 0, streamConfigurations: [:])
        }
    }
    
    // MARK: - Connection Test
    
    func testConnection(endpoint: String) async -> Bool {
        guard let url = URL(string: endpoint) else { return false }
        
        var request = URLRequest(url: url)
        request.httpMethod = "HEAD"
        request.timeoutInterval = 5.0
        
        do {
            let (_, response) = try await session.data(for: request)
            if let httpResponse = response as? HTTPURLResponse {
                return (200...499).contains(httpResponse.statusCode)
            }
        } catch {
            // Log error but don't throw
            print("Connection test failed: \(error)")
        }
        
        return false
    }
}

// MARK: - Request/Response Models

struct UploadResponse: Codable {
    let success: Bool
    let taskId: String?  // Optional since Celery processing may be disabled
    let pipelineActivityId: String
    let dataSizeBytes: Int
    let dataSize: String
    let source: String
    let message: String
    let streamKey: String
    
    private enum CodingKeys: String, CodingKey {
        case success
        case taskId = "task_id"
        case pipelineActivityId = "pipeline_activity_id"
        case dataSizeBytes = "data_size_bytes"
        case dataSize = "data_size"
        case source
        case message
        case streamKey = "stream_key"
    }
}

struct ErrorResponse: Codable {
    let error: String
    let details: String?
}