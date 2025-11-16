import Foundation

/// API client for communicating with the Ariata backend
class APIClient {

    static let shared = APIClient()

    private init() {}

    // MARK: - Data Models

    struct StreamInfo: Codable {
        let name: String
        let isEnabled: Bool
        let lastSyncAt: Date?

        enum CodingKeys: String, CodingKey {
            case name
            case isEnabled = "is_enabled"
            case lastSyncAt = "last_sync_at"
        }
    }

    struct StreamsResponse: Codable {
        let success: Bool
        let streams: [StreamInfo]
    }

    struct SourceInfo: Codable {
        let id: String
        let instanceName: String
        let sourceType: String

        enum CodingKeys: String, CodingKey {
            case id
            case instanceName = "instance_name"
            case sourceType = "source_type"
        }
    }

    struct JobInfo: Codable {
        let id: String
        let status: String
        let createdAt: Date
        let completedAt: Date?
        let recordsProcessed: Int?

        enum CodingKeys: String, CodingKey {
            case id
            case status
            case createdAt = "created_at"
            case completedAt = "completed_at"
            case recordsProcessed = "records_processed"
        }
    }

    // MARK: - API Methods

    /// Fetch all streams for the current device
    func fetchStreams() async throws -> [StreamInfo] {
        guard let config = Config.load() else {
            throw APIError.notConfigured
        }

        // Get device ID to construct the streams endpoint
        let deviceId = config.deviceId

        guard let url = URL(string: "\(config.apiEndpoint)/api/sources/\(deviceId)/streams") else {
            throw APIError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("Bearer \(config.deviceToken)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let (data, response) = try await URLSession.shared.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw APIError.invalidResponse
        }

        guard httpResponse.statusCode == 200 else {
            if httpResponse.statusCode == 401 {
                throw APIError.unauthorized
            }
            throw APIError.httpError(statusCode: httpResponse.statusCode)
        }

        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601

        let streamsResponse = try decoder.decode(StreamsResponse.self, from: data)
        return streamsResponse.streams
    }

    /// Fetch recent jobs for a specific stream
    func fetchRecentJobs(sourceId: String, streamName: String, limit: Int = 10) async throws -> [JobInfo] {
        guard let config = Config.load() else {
            throw APIError.notConfigured
        }

        guard let url = URL(string: "\(config.apiEndpoint)/api/sources/\(sourceId)/streams/\(streamName)/jobs?limit=\(limit)") else {
            throw APIError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("Bearer \(config.deviceToken)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let (data, response) = try await URLSession.shared.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw APIError.invalidResponse
        }

        guard httpResponse.statusCode == 200 else {
            if httpResponse.statusCode == 401 {
                throw APIError.unauthorized
            }
            throw APIError.httpError(statusCode: httpResponse.statusCode)
        }

        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601

        let jobs = try decoder.decode([JobInfo].self, from: data)
        return jobs
    }

    /// Check backend connectivity
    func checkConnectivity() async -> Bool {
        guard let config = Config.load() else {
            return false
        }

        guard let url = URL(string: "\(config.apiEndpoint)/api/health") else {
            return false
        }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.timeoutInterval = 5.0  // Quick timeout for connectivity check

        do {
            let (_, response) = try await URLSession.shared.data(for: request)
            if let httpResponse = response as? HTTPURLResponse {
                return httpResponse.statusCode == 200
            }
            return false
        } catch {
            return false
        }
    }
}

// MARK: - Errors

enum APIError: LocalizedError {
    case notConfigured
    case invalidURL
    case invalidResponse
    case unauthorized
    case httpError(statusCode: Int)
    case networkError(Error)

    var errorDescription: String? {
        switch self {
        case .notConfigured:
            return "Device not configured. Please pair this Mac first."
        case .invalidURL:
            return "Invalid API URL"
        case .invalidResponse:
            return "Invalid response from server"
        case .unauthorized:
            return "Unauthorized. Please re-pair this Mac."
        case .httpError(let statusCode):
            return "HTTP error: \(statusCode)"
        case .networkError(let error):
            return "Network error: \(error.localizedDescription)"
        }
    }
}
