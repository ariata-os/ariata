import ArgumentParser
import Foundation

struct StopCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "stop",
        abstract: "Stop the background daemon"
    )
    
    func run() throws {
        print("Stopping daemon...")
        
        let plistPath = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents/com.ariata.mac.plist")
        
        if !FileManager.default.fileExists(atPath: plistPath.path) {
            print("LaunchAgent not installed")
            return
        }
        
        // Unload the agent
        let result = shell("launchctl bootout gui/$(id -u) \(plistPath.path) 2>&1")
        
        if result.contains("error") && !result.contains("Could not find") {
            print("Warning: \(result)")
        } else {
            print("✓ Daemon stopped")
        }
        
        // Verify it's stopped
        Thread.sleep(forTimeInterval: 0.5)
        let listResult = shell("launchctl list | grep com.ariata.mac")
        if listResult.isEmpty {
            print("✓ Service is no longer running")
        } else {
            print("⚠️  Service may still be running")
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
            return "Error: \(error)"
        }
    }
}