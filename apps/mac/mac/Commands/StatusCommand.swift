import ArgumentParser
import Foundation

struct StatusCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "status",
        abstract: "Show current status and queue statistics"
    )
    
    func run() throws {
        print("Ariata Mac Monitor Status")
        print("=" * 30)
        
        // Check config
        if let config = Config.load() {
            print("✓ Configured")
            print("  Device ID: \(config.deviceId)")
            print("  API: \(config.apiEndpoint)")
            print("  Created: \(config.createdAt)")
        } else {
            print("✗ Not configured")
            print("  Run 'ariata-mac init <token>' to configure")
            return
        }
        
        print("")
        
        // Check monitoring capabilities
        print("Monitoring Status:")
        print("  App Monitoring: ✓ Enabled")
        
        // Check iMessage monitoring capability
        let messagesDbPath = NSString(string: "~/Library/Messages/chat.db").expandingTildeInPath
        let canReadMessages = FileManager.default.isReadableFile(atPath: messagesDbPath)
        
        if canReadMessages {
            print("  iMessage Monitoring: ✓ Enabled")
        } else {
            print("  iMessage Monitoring: ✗ Disabled")
            print("    → Grant Full Disk Access to enable")
            print("    → System Settings → Privacy & Security → Full Disk Access")
        }
        
        print("")
        
        // Check queue stats
        do {
            let queue = try Queue()
            let stats = try queue.getStats()
            
            print("Queue Statistics:")
            print("  Pending upload: \(stats.pending) events")
            print("  Already uploaded: \(stats.uploaded) events")
            print("  Total in database: \(stats.total) events")
            
            if stats.pending > 100 {
                print("\n⚠️  High number of pending events. Check network connection.")
            }
        } catch {
            print("Error reading queue: \(error)")
        }
        
        print("")
        
        // Check if daemon is installed
        let launchAgentPath = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents/com.ariata.mac.plist")
        
        if FileManager.default.fileExists(atPath: launchAgentPath.path) {
            print("LaunchAgent: ✓ Installed")
            
            // Check if running
            let result = shell("launchctl list | grep com.ariata.mac")
            if !result.isEmpty {
                print("  Status: Running")
            } else {
                print("  Status: Not running")
                print("  Run 'ariata-mac daemon' to restart")
            }
        } else {
            print("LaunchAgent: ✗ Not installed")
            print("  Run 'ariata-mac daemon' to install")
        }
    }
    
    private func shell(_ command: String) -> String {
        let task = Process()
        let pipe = Pipe()
        
        task.standardOutput = pipe
        task.standardError = pipe
        task.arguments = ["-c", command]
        task.launchPath = "/bin/bash"
        
        do {
            try task.run()
            task.waitUntilExit()
            
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            return String(data: data, encoding: .utf8) ?? ""
        } catch {
            return ""
        }
    }
}

// Helper for string repetition
extension String {
    static func *(lhs: String, rhs: Int) -> String {
        return String(repeating: lhs, count: rhs)
    }
}