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
    private var lastActivityTime: Date?
    private var authValidationFailed = false

    init() {
        Logger.log("🔧 Initializing DaemonController...", level: .debug)

        // Load config
        config = Config.load()
        if let config = config {
            Logger.log("✅ Config loaded successfully", level: .success)
            Logger.log("   Device ID: \(config.deviceId)", level: .debug)
            Logger.log("   API Endpoint: \(config.apiEndpoint)", level: .debug)
        } else {
            Logger.log("❌ Failed to load config from ~/.ariata/config.json", level: .error)
        }

        // Initialize queue
        do {
            queue = try Queue()
            Logger.log("✅ Queue initialized successfully", level: .success)

            // Log queue stats
            if let count = try? queue?.count() {
                Logger.log("   Queue has \(count) pending records", level: .debug)
            }
        } catch {
            Logger.log("❌ Error initializing queue: \(error)", level: .error)
            Logger.log("   This will prevent daemon from starting!", level: .error)

            // Log more specific error details
            if let nsError = error as NSError? {
                Logger.log("   Error domain: \(nsError.domain)", level: .debug)
                Logger.log("   Error code: \(nsError.code)", level: .debug)
                Logger.log("   Error description: \(nsError.localizedDescription)", level: .debug)
            }
        }

        Logger.log("🔧 DaemonController initialization complete", level: .debug)
    }

    /// Validate device authentication with backend
    /// Returns true if auth is valid, false otherwise
    private func validateAuth(config: Config) -> Bool {
        Logger.log("🔐 Validating device authentication...", level: .debug)

        let url = URL(string: "\(config.apiEndpoint)/api/devices/health")!
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue(config.deviceToken, forHTTPHeaderField: "X-Device-Token")
        request.timeoutInterval = 5.0 // 5 second timeout

        // Create semaphore for synchronous request
        let semaphore = DispatchSemaphore(value: 0)
        var isValid = false

        let task = URLSession.shared.dataTask(with: request) { data, response, error in
            defer { semaphore.signal() }

            if let error = error {
                Logger.log("❌ Auth validation failed: \(error.localizedDescription)", level: .error)
                return
            }

            guard let httpResponse = response as? HTTPURLResponse else {
                Logger.log("❌ Auth validation failed: invalid response", level: .error)
                return
            }

            if httpResponse.statusCode == 200 {
                Logger.log("✅ Auth validation successful", level: .success)
                isValid = true
            } else if httpResponse.statusCode == 401 {
                Logger.log("❌ Auth validation failed: invalid or revoked token", level: .error)
                Logger.log("   Your device may have been unpaired or the source deleted", level: .error)
                Logger.log("   Please re-pair this device", level: .error)
            } else {
                Logger.log("❌ Auth validation failed with status \(httpResponse.statusCode)", level: .error)
            }
        }

        task.resume()
        _ = semaphore.wait(timeout: .now() + 6.0) // Wait up to 6 seconds

        return isValid
    }

    /// Start monitoring and uploading
    func start() {
        Logger.log("🚀 Attempting to start daemon...", level: .info)

        // Check guard conditions with detailed logging
        Logger.log("   Checking start conditions:", level: .debug)
        Logger.log("   - isPaused: \(isPaused)", level: .debug)
        Logger.log("   - queue exists: \(queue != nil)", level: .debug)
        Logger.log("   - config exists: \(config != nil)", level: .debug)

        guard !isPaused else {
            Logger.log("❌ Cannot start: daemon is paused", level: .error)
            return
        }

        guard let queue = queue else {
            Logger.log("❌ Cannot start: queue is nil (initialization failed)", level: .error)
            Logger.log("   Check earlier logs for queue initialization errors", level: .error)
            return
        }

        // Reload config (may have been created after init if we just paired)
        config = Config.load()

        guard let config = config else {
            Logger.log("❌ Cannot start: config is nil", level: .error)
            Logger.log("   Config file not found at ~/.ariata/config.json", level: .error)
            Logger.log("   Run pairing flow to create config", level: .error)
            return
        }

        Logger.log("✅ All start conditions met", level: .success)
        Logger.log("   API Endpoint: \(config.apiEndpoint)", level: .info)
        Logger.log("   Device ID: \(config.deviceId)", level: .info)

        // Validate authentication before starting schedulers
        if !validateAuth(config: config) {
            Logger.log("❌ Cannot start: authentication validation failed", level: .error)
            Logger.log("   Schedulers will not start until auth is valid", level: .error)
            Logger.log("   Check that your device is still paired in the web app", level: .error)
            authValidationFailed = true
            return
        }

        authValidationFailed = false

        // Initialize and start monitors
        Logger.log("   Starting app activity monitor...", level: .debug)
        monitor = Monitor(queue: queue)
        monitor?.start()
        Logger.log("   ✅ App monitor started", level: .success)

        Logger.log("   Starting message monitor...", level: .debug)
        messageMonitor = MessageMonitor(queue: queue)
        messageMonitor?.start()
        Logger.log("   ✅ Message monitor started", level: .success)

        // Initialize and start uploader with config
        Logger.log("   Starting uploader...", level: .debug)
        uploader = Uploader(queue: queue, config: config)
        uploader?.start()
        Logger.log("   ✅ Uploader started", level: .success)

        // Set up upload callback to track last sync time
        uploader?.onUploadComplete = { [weak self] in
            self?.lastSyncTime = Date()
            self?.lastActivityTime = Date()
            Logger.log("📤 Upload completed successfully", level: .success)
        }

        // Update activity timestamp
        lastActivityTime = Date()

        Logger.log("✅ Daemon started successfully!", level: .success)
        Logger.log("   Monitoring: app activity + iMessage", level: .info)
        Logger.log("   Upload interval: every 5 minutes to \(config.apiEndpoint)/ingest", level: .info)
    }

    /// Stop monitoring and uploading
    func stop() {
        Logger.log("🛑 Stopping daemon...", level: .info)

        monitor?.stop()
        monitor = nil
        Logger.log("   Stopped app monitor", level: .debug)

        messageMonitor?.stop()
        messageMonitor = nil
        Logger.log("   Stopped message monitor", level: .debug)

        uploader?.stop()
        uploader = nil
        Logger.log("   Stopped uploader", level: .debug)

        lastActivityTime = nil

        Logger.log("✅ Daemon stopped", level: .success)
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

    /// Check if daemon is currently running
    /// Returns true if components are initialized AND have shown recent activity
    func isRunning() -> Bool {
        let hasComponents = monitor != nil && messageMonitor != nil && uploader != nil

        // If components exist, verify they're active (had activity in last 10 minutes)
        if hasComponents {
            if let lastActivity = lastActivityTime {
                let timeSinceActivity = Date().timeIntervalSince(lastActivity)
                let isActive = timeSinceActivity < 600 // 10 minutes

                if !isActive {
                    Logger.log("⚠️ Daemon components exist but no activity in \(Int(timeSinceActivity))s", level: .warning)
                }

                return isActive
            } else {
                // Just started, components exist but no activity yet - consider it running
                return true
            }
        }

        return false
    }

    /// Check if auth validation failed
    func hasAuthFailed() -> Bool {
        return authValidationFailed
    }

    /// Get current daemon statistics
    func getStats() -> DaemonStats {
        let queuedRecords = (try? queue?.count()) ?? 0

        return DaemonStats(
            lastSyncTime: lastSyncTime,
            queuedRecords: queuedRecords,
            isPaused: isPaused,
            authFailed: authValidationFailed
        )
    }

    /// Trigger immediate sync/upload of queued records
    /// Returns tuple of (uploaded count, failed count)
    func syncNow() async -> (uploaded: Int, failed: Int) {
        guard let uploader = uploader else {
            Logger.log("⚠️ Cannot sync: uploader not initialized", level: .warning)
            return (0, 0)
        }

        Logger.log("🔄 Manual sync triggered", level: .info)
        let result = await uploader.uploadNow()
        Logger.log("✅ Manual sync complete: \(result.uploaded) uploaded, \(result.failed) failed", level: .success)

        // Update last sync time on successful upload
        if result.uploaded > 0 {
            lastSyncTime = Date()
            lastActivityTime = Date()
        }

        return result
    }
}

/// Statistics about daemon state
struct DaemonStats {
    let lastSyncTime: Date?
    let queuedRecords: Int
    let isPaused: Bool
    let authFailed: Bool
}
