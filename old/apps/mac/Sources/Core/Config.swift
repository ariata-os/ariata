import Foundation

struct Config: Codable {
    let deviceToken: String
    let deviceId: String
    let apiEndpoint: String
    let createdAt: Date
    
    static let configDir = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".ariata")
    static let configFile = configDir.appendingPathComponent("config.json")
    
    static func load() -> Config? {
        guard FileManager.default.fileExists(atPath: configFile.path) else {
            return nil
        }
        
        do {
            let data = try Data(contentsOf: configFile)
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            return try decoder.decode(Config.self, from: data)
        } catch {
            print("Error loading config: \(error)")
            return nil
        }
    }
    
    func save() throws {
        // Create directory if needed
        try FileManager.default.createDirectory(
            at: Config.configDir,
            withIntermediateDirectories: true
        )
        
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = .prettyPrinted
        let data = try encoder.encode(self)
        try data.write(to: Config.configFile)
    }
    
    static func delete() throws {
        if FileManager.default.fileExists(atPath: configFile.path) {
            try FileManager.default.removeItem(at: configFile)
        }
    }
    
    static func validateToken(_ token: String) async throws -> (endpoint: String, deviceName: String) {
        // Check for API URL from environment variable first, then fall back to localhost
        // This allows the installer to pass the endpoint via ARIATA_API_URL
        let baseURL = ProcessInfo.processInfo.environment["ARIATA_API_URL"] ?? "http://localhost:3000"
        
        guard let url = URL(string: "\(baseURL)/api/sources/device-token?token=\(token)") else {
            throw ConfigError.invalidToken
        }
        
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        
        let (data, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse,
              httpResponse.statusCode == 200 else {
            throw ConfigError.invalidToken
        }
        
        // Parse response to check if token exists
        if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let success = json["success"] as? Bool,
           let exists = json["exists"] as? Bool,
           success && exists,
           let source = json["source"] as? [String: Any],
           let instanceName = source["instanceName"] as? String {
            return (endpoint: baseURL, deviceName: instanceName)
        }
        
        throw ConfigError.invalidToken
    }
}

enum ConfigError: LocalizedError {
    case notConfigured
    case invalidToken
    case networkError(String)
    
    var errorDescription: String? {
        switch self {
        case .notConfigured:
            return "Not configured. Run 'ariata-mac init <token>' first."
        case .invalidToken:
            return "Invalid device token. Please check your token and try again."
        case .networkError(let message):
            return "Network error: \(message)"
        }
    }
}