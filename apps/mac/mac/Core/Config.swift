import Foundation
import Security

struct Config: Codable {
    let deviceToken: String
    let deviceId: String
    let apiEndpoint: String
    let createdAt: Date

    static let configDir = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".ariata")
    static let configFile = configDir.appendingPathComponent("config.json")

    // Keychain constants
    private static let keychainService = "com.ariata.mac"
    private static let keychainAccount = "device-token"

    // Private struct for JSON storage (without token)
    private struct ConfigFile: Codable {
        let deviceId: String
        let apiEndpoint: String
        let createdAt: Date
    }
    
    static func load() -> Config? {
        guard FileManager.default.fileExists(atPath: configFile.path) else {
            return nil
        }

        do {
            // Load config file (without token)
            let data = try Data(contentsOf: configFile)
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            let configFile = try decoder.decode(ConfigFile.self, from: data)

            // Load token from Keychain
            guard let deviceToken = loadTokenFromKeychain() else {
                print("⚠️ Config file exists but token not found in Keychain")
                return nil
            }

            return Config(
                deviceToken: deviceToken,
                deviceId: configFile.deviceId,
                apiEndpoint: configFile.apiEndpoint,
                createdAt: configFile.createdAt
            )
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

        // Save token to Keychain
        try Self.saveTokenToKeychain(deviceToken)

        // Save config file (without token)
        let configFile = ConfigFile(
            deviceId: deviceId,
            apiEndpoint: apiEndpoint,
            createdAt: createdAt
        )

        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = .prettyPrinted
        let data = try encoder.encode(configFile)
        try data.write(to: Config.configFile)

        print("✅ Config saved (token stored securely in Keychain)")
    }
    
    static func delete() throws {
        // Delete token from Keychain
        deleteTokenFromKeychain()

        // Delete config file
        if FileManager.default.fileExists(atPath: configFile.path) {
            try FileManager.default.removeItem(at: configFile)
        }
    }

    // MARK: - Keychain Helpers

    private static func saveTokenToKeychain(_ token: String) throws {
        guard let tokenData = token.data(using: .utf8) else {
            throw ConfigError.networkError("Failed to encode token")
        }

        // First, try to update existing keychain item
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: keychainAccount
        ]

        let attributes: [String: Any] = [
            kSecValueData as String: tokenData
        ]

        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)

        if updateStatus == errSecItemNotFound {
            // Item doesn't exist, create new one
            var newItem = query
            newItem[kSecValueData as String] = tokenData

            let addStatus = SecItemAdd(newItem as CFDictionary, nil)

            guard addStatus == errSecSuccess else {
                throw ConfigError.networkError("Failed to save token to Keychain: \(addStatus)")
            }
        } else if updateStatus != errSecSuccess {
            throw ConfigError.networkError("Failed to update token in Keychain: \(updateStatus)")
        }
    }

    private static func loadTokenFromKeychain() -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: keychainAccount,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess,
              let data = result as? Data,
              let token = String(data: data, encoding: .utf8) else {
            return nil
        }

        return token
    }

    private static func deleteTokenFromKeychain() {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: keychainAccount
        ]

        let status = SecItemDelete(query as CFDictionary)

        if status == errSecSuccess {
            print("✅ Token deleted from Keychain")
        } else if status == errSecItemNotFound {
            print("ℹ️ No token found in Keychain to delete")
        } else {
            print("⚠️ Failed to delete token from Keychain: \(status)")
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