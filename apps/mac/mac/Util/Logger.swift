import Foundation

/// Shared logger for Mac app - writes to both console and file
class Logger {
    private static var logFile: FileHandle?
    private static var isSetup = false

    /// Setup logging to file
    static func setup() {
        guard !isSetup else { return }

        let fileManager = FileManager.default
        let logDir = fileManager.homeDirectoryForCurrentUser.appendingPathComponent(".ariata/logs")

        try? fileManager.createDirectory(at: logDir, withIntermediateDirectories: true)

        let logPath = logDir.appendingPathComponent("mac-app.log")

        if !fileManager.fileExists(atPath: logPath.path) {
            fileManager.createFile(atPath: logPath.path, contents: nil)
        }

        logFile = try? FileHandle(forWritingTo: logPath)
        logFile?.seekToEndOfFile()

        isSetup = true

        log(String(repeating: "=", count: 50))
        log("Logger initialized at \(Date())")
    }

    /// Log a message to both console and file
    static func log(_ message: String, level: LogLevel = .info) {
        let timestamp = ISO8601DateFormatter().string(from: Date())
        let levelStr = level.emoji
        let logMessage = "[\(timestamp)] \(levelStr) \(message)\n"

        // Print to console
        print("\(levelStr) \(message)")

        // Write to file
        if let data = logMessage.data(using: .utf8) {
            logFile?.write(data)
        }
    }

    /// Close log file
    static func close() {
        try? logFile?.close()
        logFile = nil
        isSetup = false
    }

    enum LogLevel {
        case debug
        case info
        case warning
        case error
        case success

        var emoji: String {
            switch self {
            case .debug: return "🔍"
            case .info: return "ℹ️"
            case .warning: return "⚠️"
            case .error: return "❌"
            case .success: return "✅"
            }
        }
    }
}
