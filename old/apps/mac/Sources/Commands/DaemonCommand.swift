import ArgumentParser
import Foundation

struct DaemonCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "daemon",
        abstract: "Install/update LaunchAgent for background monitoring"
    )
    
    func run() throws {
        // Check config first
        guard Config.load() != nil else {
            throw ConfigError.notConfigured
        }
        
        print("Installing LaunchAgent...")
        
        // Get paths
        let launchAgentsDir = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents")
        let plistPath = launchAgentsDir.appendingPathComponent("com.ariata.mac.plist")
        
        // Create LaunchAgents directory if needed
        try FileManager.default.createDirectory(
            at: launchAgentsDir,
            withIntermediateDirectories: true
        )
        
        // Get current executable path
        let executablePath = ProcessInfo.processInfo.arguments[0]
        let resolvedPath = URL(fileURLWithPath: executablePath).standardized.path
        
        // Create plist content
        let plistContent = """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>Label</key>
            <string>com.ariata.mac</string>
            
            <key>ProgramArguments</key>
            <array>
                <string>\(resolvedPath)</string>
                <string>start</string>
            </array>
            
            <key>RunAtLoad</key>
            <true/>
            
            <key>KeepAlive</key>
            <dict>
                <key>SuccessfulExit</key>
                <false/>
                <key>Crashed</key>
                <true/>
            </dict>
            
            <key>ProcessType</key>
            <string>Background</string>
            
            <key>StandardOutPath</key>
            <string>\(Config.configDir.path)/ariata-mac.log</string>
            
            <key>StandardErrorPath</key>
            <string>\(Config.configDir.path)/ariata-mac.error.log</string>
            
            <key>WorkingDirectory</key>
            <string>\(FileManager.default.homeDirectoryForCurrentUser.path)</string>
            
            <key>ThrottleInterval</key>
            <integer>10</integer>
        </dict>
        </plist>
        """
        
        // Unload if already loaded
        let _ = shell("launchctl bootout gui/$(id -u) \(plistPath.path) 2>/dev/null")
        Thread.sleep(forTimeInterval: 0.5)
        
        // Write plist
        try plistContent.write(to: plistPath, atomically: true, encoding: .utf8)
        print("✓ Created LaunchAgent plist")
        
        // Load the agent
        let loadResult = shell("launchctl bootstrap gui/$(id -u) \(plistPath.path) 2>&1")
        if loadResult.contains("error") && !loadResult.contains("already exists") {
            print("⚠️  Warning: \(loadResult)")
        } else {
            print("✓ LaunchAgent loaded")
        }
        
        // Verify it's running
        Thread.sleep(forTimeInterval: 1)
        let listResult = shell("launchctl list | grep com.ariata.mac")
        if !listResult.isEmpty {
            print("✓ Service is running")
            print("\nMonitoring started in background!")
            print("Logs: ~/.ariata/ariata-mac.log")
            print("To stop: ariata-mac stop")
        } else {
            print("⚠️  Service may not have started. Check logs at ~/.ariata/ariata-mac.error.log")
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