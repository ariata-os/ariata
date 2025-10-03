import ArgumentParser
import Foundation

struct InitCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "init",
        abstract: "Initialize with a device token from the web UI"
    )
    
    @Argument(help: "Device token from the Ariata web UI")
    var token: String
    
    func run() throws {
        print("Validating device token...")
        
        // Create a class to hold mutable state safely
        class ResultHolder {
            var config: Config?
            var error: Error?
        }
        
        let holder = ResultHolder()
        let group = DispatchGroup()
        
        group.enter()
        Task {
            do {
                // Validate token and get endpoint
                let (endpoint, deviceName) = try await Config.validateToken(token.uppercased())
                
                print("✓ Token validated")
                print("✓ Connected to: \(endpoint)")
                print("✓ Device name: \(deviceName)")
                
                // Generate device ID
                let deviceId = UUID().uuidString
                
                // Create and save config
                let config = Config(
                    deviceToken: token.uppercased(),
                    deviceId: deviceId,
                    apiEndpoint: endpoint,
                    createdAt: Date()
                )
                
                try config.save()
                
                print("✓ Configuration saved to ~/.ariata/config.json")
                print("\nReady to start monitoring!")
                print("Run 'ariata-mac start' to begin monitoring")
                print("Run 'ariata-mac daemon' to install as background service")
                
                holder.config = config
            } catch {
                holder.error = error
            }
            group.leave()
        }
        
        group.wait()
        
        if let error = holder.error {
            throw error
        } else if holder.config != nil {
            return
        } else {
            throw ConfigError.networkError("Unknown error")
        }
    }
}