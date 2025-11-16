import Foundation

/// Wrapper for the daemon components (Monitor, MessageMonitor, Queue, Uploader)
/// Provides control interface for menu bar app
class DaemonController {

    private var monitor: Monitor?
    private var messageMonitor: MessageMonitor?
    private var uploader: Uploader?
    private var queue: Queue?
    private var config: Config?
    private var isPaused = false
    private var lastSyncTime: Date?

    init() {
        // Load config
        config = Config.load()

        // Initialize queue
        do {
            queue = try Queue()
        } catch {
            print("Error initializing queue: \(error)")
        }
    }

    /// Start monitoring and uploading
    func start() {
        guard !isPaused, let queue = queue, let config = config else {
            if config == nil {
                print("⚠️ Config not found. Run 'ariata-mac init <token>' first.")
            }
            return
        }

        print("🚀 Starting daemon...")
        print("   API Endpoint: \(config.apiEndpoint)")
        print("   Device ID: \(config.deviceId)")

        // Initialize and start monitors
        monitor = Monitor(queue: queue)
        monitor?.start()

        messageMonitor = MessageMonitor(queue: queue)
        messageMonitor?.start()

        // Initialize and start uploader with config
        uploader = Uploader(queue: queue, config: config)
        uploader?.start()

        // Set up upload callback to track last sync time
        uploader?.onUploadComplete = { [weak self] in
            self?.lastSyncTime = Date()
        }

        print("✅ Daemon started successfully - monitoring app activity and iMessage")
        print("   Data will be uploaded every 5 minutes to \(config.apiEndpoint)/ingest")
    }

    /// Stop monitoring and uploading
    func stop() {
        print("Stopping daemon...")

        monitor?.stop()
        monitor = nil

        messageMonitor?.stop()
        messageMonitor = nil

        uploader?.stop()
        uploader = nil

        print("Daemon stopped")
    }

    /// Toggle pause state
    func togglePause() {
        isPaused.toggle()

        if isPaused {
            print("Pausing monitoring...")
            stop()
        } else {
            print("Resuming monitoring...")
            start()
        }
    }

    /// Get current daemon statistics
    func getStats() -> DaemonStats {
        let queuedRecords = (try? queue?.count()) ?? 0

        return DaemonStats(
            lastSyncTime: lastSyncTime,
            queuedRecords: queuedRecords,
            isPaused: isPaused
        )
    }
}

/// Statistics about daemon state
struct DaemonStats {
    let lastSyncTime: Date?
    let queuedRecords: Int
    let isPaused: Bool
}
