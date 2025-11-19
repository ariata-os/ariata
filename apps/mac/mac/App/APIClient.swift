import Foundation

/// API client for communicating with the Ariata backend
class APIClient {

    static let shared = APIClient()

    private init() {}

    // MARK: - Data Models

    struct StreamInfo: Decodable {
        let streamName: String
        let displayName: String
        let description: String
        let tableName: String
        let isEnabled: Bool
        let cronSchedule: String?
        let config: [String: Any]?
        let lastSyncAt: Date?
        let supportsIncremental: Bool
        let supportsFullRefresh: Bool
        let configSchema: [String: Any]?
        let configExample: [String: Any]?
        let defaultCronSchedule: String?

        enum CodingKeys: String, CodingKey {
            case streamName = "stream_name"
            case displayName = "display_name"
            case description
            case tableName = "table_name"
            case isEnabled = "is_enabled"
            case cronSchedule = "cron_schedule"
            case config
            case lastSyncAt = "last_sync_at"
            case supportsIncremental = "supports_incremental"
            case supportsFullRefresh = "supports_full_refresh"
            case configSchema = "config_schema"
            case configExample = "config_example"
            case defaultCronSchedule = "default_cron_schedule"
        }

        // Custom decoding for JSON values
        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            streamName = try container.decode(String.self, forKey: .streamName)
            displayName = try container.decode(String.self, forKey: .displayName)
            description = try container.decode(String.self, forKey: .description)
            tableName = try container.decode(String.self, forKey: .tableName)
            isEnabled = try container.decode(Bool.self, forKey: .isEnabled)
            cronSchedule = try container.decodeIfPresent(String.self, forKey: .cronSchedule)
            lastSyncAt = try container.decodeIfPresent(Date.self, forKey: .lastSyncAt)
            supportsIncremental = try container.decode(Bool.self, forKey: .supportsIncremental)
            supportsFullRefresh = try container.decode(Bool.self, forKey: .supportsFullRefresh)
            defaultCronSchedule = try container.decodeIfPresent(String.self, forKey: .defaultCronSchedule)

            // Decode JSON fields as generic dictionaries
            if let configData = try? container.decode(Data.self, forKey: .config),
               let configJSON = try? JSONSerialization.jsonObject(with: configData) as? [String: Any] {
                config = configJSON
            } else {
                config = nil
            }

            if let schemaData = try? container.decode(Data.self, forKey: .configSchema),
               let schemaJSON = try? JSONSerialization.jsonObject(with: schemaData) as? [String: Any] {
                configSchema = schemaJSON
            } else {
                configSchema = nil
            }

            if let exampleData = try? container.decode(Data.self, forKey: .configExample),
               let exampleJSON = try? JSONSerialization.jsonObject(with: exampleData) as? [String: Any] {
                configExample = exampleJSON
            } else {
                configExample = nil
            }
        }
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

        // Backend returns an array directly, not a wrapped response
        let streams = try decoder.decode([StreamInfo].self, from: data)
        return streams
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
