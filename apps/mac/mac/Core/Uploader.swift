import Foundation

class Uploader {
    private let queue: Queue
    private let config: Config
    private var timer: DispatchSourceTimer?

    // Thread-safe state management
    private let stateQueue = DispatchQueue(label: "com.ariata.uploader.state")
    private var _retryDelay: TimeInterval = 60 // Start with 1 minute
    private let maxRetryDelay: TimeInterval = 960 // Max 16 minutes
    private var _consecutive401Errors = 0
    private let max401Errors = 3 // Pause after 3 consecutive 401s (~15 minutes)
    private var _isAuthPaused = false
    private var _authPauseUntil: Date?
    private var _authPauseDuration: TimeInterval = 3600 // Start with 1 hour

    private var retryDelay: TimeInterval {
        get { stateQueue.sync { _retryDelay } }
        set { stateQueue.sync(flags: .barrier) { _retryDelay = newValue } }
    }

    private var consecutive401Errors: Int {
        get { stateQueue.sync { _consecutive401Errors } }
        set { stateQueue.sync(flags: .barrier) { _consecutive401Errors = newValue } }
    }

    private var isAuthPaused: Bool {
        get { stateQueue.sync { _isAuthPaused } }
        set { stateQueue.sync(flags: .barrier) { _isAuthPaused = newValue } }
    }

    private var authPauseUntil: Date? {
        get { stateQueue.sync { _authPauseUntil } }
        set { stateQueue.sync(flags: .barrier) { _authPauseUntil = newValue } }
    }

    private var authPauseDuration: TimeInterval {
        get { stateQueue.sync { _authPauseDuration } }
        set { stateQueue.sync(flags: .barrier) { _authPauseDuration = newValue } }
    }

    /// Callback invoked after successful upload
    var onUploadComplete: (() -> Void)?

    /// Callback invoked when auth fails repeatedly
    var onAuthFailure: (() -> Void)?

    init(queue: Queue, config: Config) {
        self.queue = queue
        self.config = config
    }
    
    func start() {
        print("Starting uploader (5-minute intervals)...")

        // Upload immediately on start
        Task {
            await upload()
        }

        // Create dispatch timer for reliable firing in menu bar app
        // This works even when menus are open, unlike NSTimer/RunLoop
        let newTimer = DispatchSource.makeTimerSource(queue: .main)
        newTimer.schedule(deadline: .now() + 300, repeating: 300) // 5 minutes
        newTimer.setEventHandler { [weak self] in
            guard let self = self else { return }
            Task {
                await self.upload()
            }
        }
        newTimer.resume()
        self.timer = newTimer
    }
    
    func stop() {
        timer?.cancel()
        timer = nil
        print("Uploader stopped")
    }
    
    func uploadNow() async -> (uploaded: Int, failed: Int) {
        return await upload()
    }
    
    @discardableResult
    private func upload() async -> (uploaded: Int, failed: Int) {
        // Check if uploads are paused due to auth failures
        if isAuthPaused {
            if let pauseUntil = authPauseUntil, Date() < pauseUntil {
                let timeRemaining = Int(pauseUntil.timeIntervalSinceNow / 60)
                print("⏸️ Uploads paused due to auth failure (resuming in \(timeRemaining) minutes)")
                return (0, 0)
            } else {
                // Pause expired, try again
                print("🔄 Auth pause expired, resuming uploads...")
                isAuthPaused = false
                authPauseUntil = nil
                consecutive401Errors = 0
            }
        }

        var totalUploaded = 0
        var totalFailed = 0

        // Upload events
        let eventsResult = await uploadEvents()
        totalUploaded += eventsResult.uploaded
        totalFailed += eventsResult.failed

        // Upload messages
        let messagesResult = await uploadMessages()
        totalUploaded += messagesResult.uploaded
        totalFailed += messagesResult.failed

        // Log summary
        if totalUploaded > 0 || totalFailed > 0 {
            print("📤 Upload summary: \(totalUploaded) successful, \(totalFailed) failed")
        }

        // Call completion callback if any records were uploaded
        if totalUploaded > 0 {
            onUploadComplete?()
        }

        return (totalUploaded, totalFailed)
    }
    
    private func uploadEvents() async -> (uploaded: Int, failed: Int) {
        do {
            // Get pending events with their IDs
            let eventsWithIds = try queue.getPendingEvents()
            
            if eventsWithIds.isEmpty {
                return (0, 0)
            }
            
            print("Uploading \(eventsWithIds.count) events...")
            
            // Extract just the events for the payload
            let events = eventsWithIds.map { $0.event }
            
            // Prepare payload
            let payload: [String: Any] = [
                "source": "mac",
                "stream": "apps",
                "device_id": config.deviceId,
                "records": events.map { $0.toDictionary },
                "timestamp": ISO8601DateFormatter().string(from: Date())
            ]
            
            // Create request
            guard let url = URL(string: "\(config.apiEndpoint)/ingest") else {
                print("Invalid API endpoint")
                return (0, events.count)
            }
            
            var request = URLRequest(url: url)
            request.httpMethod = "POST"
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.setValue(config.deviceToken, forHTTPHeaderField: "X-Device-Token")
            request.httpBody = try JSONSerialization.data(withJSONObject: payload)
            
            // Send request
            let (data, response) = try await URLSession.shared.data(for: request)
            
            if let httpResponse = response as? HTTPURLResponse {
                if httpResponse.statusCode == 200 {
                    // Success - mark events as uploaded
                    let eventIds = eventsWithIds.map { $0.id }
                    try queue.markEventsAsUploaded(eventIds)

                    // Clean up old events
                    try queue.cleanupOldEvents()

                    print("✓ Uploaded \(events.count) events successfully")
                    retryDelay = 60 // Reset retry delay on success
                    consecutive401Errors = 0 // Reset auth error counter

                    // Reset pause state on successful upload
                    if isAuthPaused {
                        print("🔄 Auth successful - resuming normal uploads")
                        isAuthPaused = false
                        authPauseUntil = nil
                        authPauseDuration = 3600 // Reset to 1 hour
                    }

                    return (events.count, 0)
                } else if httpResponse.statusCode == 401 {
                    print("❌ Upload failed: Authentication error (401)")
                    if let body = String(data: data, encoding: .utf8) {
                        print("   Response: \(body)")
                    }

                    consecutive401Errors += 1
                    print("   Consecutive 401 errors: \(consecutive401Errors)/\(max401Errors)")

                    if consecutive401Errors >= max401Errors {
                        // Pause uploads with exponential backoff instead of stopping completely
                        let pauseMinutes = Int(authPauseDuration / 60)
                        print("❌ CRITICAL: Auth failed \(consecutive401Errors) times - pausing uploads for \(pauseMinutes) minutes")
                        print("   Your device may have been unpaired or the source deleted")
                        print("   Uploads will automatically resume after pause expires")
                        print("   Or re-pair this device to resume immediately")

                        authPauseUntil = Date().addingTimeInterval(authPauseDuration)
                        isAuthPaused = true

                        // Double pause duration for next time (exponential backoff)
                        authPauseDuration = min(authPauseDuration * 2, 24 * 3600) // Max 24 hours

                        // Notify callback
                        onAuthFailure?()
                    }

                    return (0, events.count)
                } else {
                    print("Upload failed with status: \(httpResponse.statusCode)")
                    if let body = String(data: data, encoding: .utf8) {
                        print("Response: \(body)")
                    }

                    // Reset 401 counter for non-auth errors
                    consecutive401Errors = 0

                    // Exponential backoff
                    retryDelay = min(retryDelay * 2, maxRetryDelay)
                    return (0, events.count)
                }
            }
            
            return (0, events.count)
            
        } catch {
            print("Upload events error: \(error)")
            retryDelay = min(retryDelay * 2, maxRetryDelay)
            return (0, 0)
        }
    }
    
    private func uploadMessages() async -> (uploaded: Int, failed: Int) {
        do {
            // Get pending messages with their IDs
            let messagesWithIds = try queue.getPendingMessages()
            
            if messagesWithIds.isEmpty {
                return (0, 0)
            }
            
            print("Uploading \(messagesWithIds.count) messages...")
            
            // Extract just the messages for the payload
            let messages = messagesWithIds.map { $0.message }
            
            // Prepare payload
            let payload: [String: Any] = [
                "source": "mac",
                "stream": "imessage",
                "device_id": config.deviceId,
                "records": messages.map { $0.toDictionary },
                "timestamp": ISO8601DateFormatter().string(from: Date())
            ]
            
            // Create request
            guard let url = URL(string: "\(config.apiEndpoint)/ingest") else {
                print("Invalid API endpoint")
                return (0, messages.count)
            }
            
            var request = URLRequest(url: url)
            request.httpMethod = "POST"
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.setValue(config.deviceToken, forHTTPHeaderField: "X-Device-Token")
            request.httpBody = try JSONSerialization.data(withJSONObject: payload)
            
            // Send request
            let (data, response) = try await URLSession.shared.data(for: request)
            
            if let httpResponse = response as? HTTPURLResponse {
                if httpResponse.statusCode == 200 {
                    // Success - mark messages as uploaded
                    let messageIds = messagesWithIds.map { $0.id }
                    try queue.markMessagesAsUploaded(messageIds)

                    // Clean up old messages
                    try queue.cleanupOldMessages()

                    print("✓ Uploaded \(messages.count) messages successfully")
                    retryDelay = 60 // Reset retry delay on success
                    consecutive401Errors = 0 // Reset auth error counter

                    // Reset pause state on successful upload
                    if isAuthPaused {
                        print("🔄 Auth successful - resuming normal uploads")
                        isAuthPaused = false
                        authPauseUntil = nil
                        authPauseDuration = 3600 // Reset to 1 hour
                    }

                    return (messages.count, 0)
                } else if httpResponse.statusCode == 401 {
                    print("❌ Upload messages failed: Authentication error (401)")
                    if let body = String(data: data, encoding: .utf8) {
                        print("   Response: \(body)")
                    }

                    consecutive401Errors += 1
                    print("   Consecutive 401 errors: \(consecutive401Errors)/\(max401Errors)")

                    if consecutive401Errors >= max401Errors {
                        // Pause uploads with exponential backoff instead of stopping completely
                        let pauseMinutes = Int(authPauseDuration / 60)
                        print("❌ CRITICAL: Auth failed \(consecutive401Errors) times - pausing uploads for \(pauseMinutes) minutes")
                        print("   Your device may have been unpaired or the source deleted")
                        print("   Uploads will automatically resume after pause expires")
                        print("   Or re-pair this device to resume immediately")

                        authPauseUntil = Date().addingTimeInterval(authPauseDuration)
                        isAuthPaused = true

                        // Double pause duration for next time (exponential backoff)
                        authPauseDuration = min(authPauseDuration * 2, 24 * 3600) // Max 24 hours

                        // Notify callback
                        onAuthFailure?()
                    }

                    return (0, messages.count)
                } else {
                    print("Upload messages failed with status: \(httpResponse.statusCode)")
                    if let body = String(data: data, encoding: .utf8) {
                        print("Response: \(body)")
                    }

                    // Reset 401 counter for non-auth errors
                    consecutive401Errors = 0

                    // Exponential backoff
                    retryDelay = min(retryDelay * 2, maxRetryDelay)
                    return (0, messages.count)
                }
            }
            
            return (0, messages.count)
            
        } catch {
            print("Upload messages error: \(error)")
            retryDelay = min(retryDelay * 2, maxRetryDelay)
            return (0, 0)
        }
    }
}