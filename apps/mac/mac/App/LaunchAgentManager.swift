import Foundation

/// Manages LaunchAgent installation for auto-start on login
class LaunchAgentManager {

    private let plistName = "com.ariata.daemon.plist"

    private var launchAgentPath: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents")
            .appendingPathComponent(plistName)
    }

    /// Check if LaunchAgent is installed
    func isInstalled() -> Bool {
        return FileManager.default.fileExists(atPath: launchAgentPath.path)
    }

    /// Install LaunchAgent for auto-start
    func install() throws {
        // Ensure LaunchAgents directory exists
        let launchAgentsDir = launchAgentPath.deletingLastPathComponent()
        try FileManager.default.createDirectory(
            at: launchAgentsDir,
            withIntermediateDirectories: true
        )

        // Get path to Ariata.app
        let appPath = Bundle.main.bundlePath
        let executablePath = "\(appPath)/Contents/MacOS/Ariata"

        // Create plist content
        let plistContent = """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>Label</key>
            <string>com.ariata.daemon</string>

            <key>ProgramArguments</key>
            <array>
                <string>\(executablePath)</string>
                <string>--daemon</string>
            </array>

            <key>RunAtLoad</key>
            <true/>

            <key>KeepAlive</key>
            <true/>

            <key>StandardOutPath</key>
            <string>/tmp/ariata.log</string>

            <key>StandardErrorPath</key>
            <string>/tmp/ariata.err</string>
        </dict>
        </plist>
        """

        // Write plist file
        try plistContent.write(to: launchAgentPath, atomically: true, encoding: .utf8)

        // Load with launchctl
        try loadLaunchAgent()

        print("LaunchAgent installed successfully at \(launchAgentPath.path)")
    }

    /// Uninstall LaunchAgent
    func uninstall() throws {
        // Unload first
        try unloadLaunchAgent()

        // Remove plist file
        if FileManager.default.fileExists(atPath: launchAgentPath.path) {
            try FileManager.default.removeItem(at: launchAgentPath)
        }

        print("LaunchAgent uninstalled successfully")
    }

    /// Load LaunchAgent with launchctl
    private func loadLaunchAgent() throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/launchctl")
        process.arguments = ["load", launchAgentPath.path]

        try process.run()
        process.waitUntilExit()

        if process.terminationStatus != 0 {
            throw LaunchAgentError.loadFailed
        }
    }

    /// Unload LaunchAgent with launchctl
    private func unloadLaunchAgent() throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/launchctl")
        process.arguments = ["unload", launchAgentPath.path]

        try process.run()
        process.waitUntilExit()

        // Ignore error if already unloaded
    }

    enum LaunchAgentError: Error {
        case loadFailed
        case unloadFailed
    }
}
