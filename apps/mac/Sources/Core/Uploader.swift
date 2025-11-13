import Foundation

class Uploader {
    private let queue: Queue
    private let config: Config
    private var timer: Timer?
    private var retryDelay: TimeInterval = 60 // Start with 1 minute
    private let maxRetryDelay: TimeInterval = 960 // Max 16 minutes
    
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
        
        // Then every 5 minutes
        timer = Timer.scheduledTimer(withTimeInterval: 300, repeats: true) { [weak self] _ in
            guard let self = self else { return }
            Task {
                await self.upload()
            }
        }
    }
    
    func stop() {
        timer?.invalidate()
        timer = nil
        print("Uploader stopped")
    }
    
    func uploadNow() async -> (uploaded: Int, failed: Int) {
        return await upload()
    }
    
    @discardableResult
    private func upload() async -> (uploaded: Int, failed: Int) {
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
                "stream_name": "mac_apps",
                "device_id": config.deviceId,
                "data": events.map { $0.toDictionary },
                "batch_metadata": [
                    "total_records": events.count
                ]
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
                    return (events.count, 0)
                } else {
                    print("Upload failed with status: \(httpResponse.statusCode)")
                    if let body = String(data: data, encoding: .utf8) {
                        print("Response: \(body)")
                    }
                    
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
                "stream_name": "mac_messages",
                "device_id": config.deviceId,
                "data": messages.map { $0.toDictionary },
                "batch_metadata": [
                    "total_records": messages.count
                ]
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
                    return (messages.count, 0)
                } else {
                    print("Upload messages failed with status: \(httpResponse.statusCode)")
                    if let body = String(data: data, encoding: .utf8) {
                        print("Response: \(body)")
                    }
                    
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