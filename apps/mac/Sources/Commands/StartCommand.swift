import ArgumentParser
import Foundation

// Global references for signal handlers
private var globalMonitor: Monitor?
private var globalUploader: Uploader?

struct StartCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "start",
        abstract: "Start monitoring in foreground"
    )
    
    @Flag(name: .shortAndLong, help: "Show debug output")
    var verbose = false
    
    func run() throws {
        // Load config
        guard let config = Config.load() else {
            throw ConfigError.notConfigured
        }
        
        print("Starting Ariata Mac Monitor...")
        print("Device ID: \(config.deviceId)")
        print("API Endpoint: \(config.apiEndpoint)")
        print("Press Ctrl+C to stop\n")
        
        // Initialize components
        let queue = try Queue()
        let monitor = Monitor(queue: queue)
        let uploader = Uploader(queue: queue, config: config)
        
        // Store globally for signal handlers
        globalMonitor = monitor
        globalUploader = uploader
        
        // Start monitoring and uploading
        monitor.start()
        uploader.start()
        
        // Set up signal handlers for graceful shutdown
        signal(SIGINT) { _ in
            print("\nShutting down...")
            globalMonitor?.stop()
            globalUploader?.stop()
            Foundation.exit(0)
        }
        
        signal(SIGTERM) { _ in
            print("\nShutting down...")
            globalMonitor?.stop()
            globalUploader?.stop()
            Foundation.exit(0)
        }
        
        // Run forever
        RunLoop.main.run()
    }
}