//
//  BatchUploadCoordinator.swift
//  Ariata
//
//  Coordinates batch uploads with retry logic and background handling
//

import Foundation
import UIKit
import BackgroundTasks
import Combine

class BatchUploadCoordinator: ObservableObject {
    static let shared = BatchUploadCoordinator()
    
    @Published var isUploading = false
    @Published var lastUploadDate: Date?
    @Published var lastSuccessfulSyncDate: Date?
    @Published var uploadStats: (pending: Int, failed: Int, total: Int) = (0, 0, 0)
    @Published var streamCounts: (healthkit: Int, location: Int, audio: Int) = (0, 0, 0)
    
    private var uploadTimer: Timer?
    private let uploadInterval: TimeInterval = 300 // 5 minutes
    private let backgroundTaskIdentifier = "com.ariata.ios.sync"
    
    private var statsUpdateTimer: Timer?
    
    private let lastUploadKey = "com.ariata.lastUploadDate"
    private let lastSuccessfulSyncKey = "com.ariata.lastSuccessfulSyncDate"
    private let networkManager = NetworkManager.shared
    private let deviceManager = DeviceManager.shared
    private let sqliteManager = SQLiteManager.shared
    
    private init() {
        loadLastUploadDate()
        loadLastSuccessfulSyncDate()
        registerBackgroundTasks()
        updateUploadStats()
    }
    
    // MARK: - Timer Management
    
    func startPeriodicUploads() {
        stopPeriodicUploads()
        
        // Schedule timer for 5-minute intervals
        uploadTimer = Timer.scheduledTimer(withTimeInterval: uploadInterval, repeats: true) { [weak self] _ in
            Task {
                await self?.performUpload()
            }
        }
        
        // Schedule stats update timer for every 2 seconds
        statsUpdateTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            self?.updateUploadStats()
        }
        
        // Perform initial upload
        Task {
            await performUpload()
        }
        
        // Schedule background task
        scheduleBackgroundUpload()
    }
    
    func stopPeriodicUploads() {
        uploadTimer?.invalidate()
        uploadTimer = nil
        statsUpdateTimer?.invalidate()
        statsUpdateTimer = nil
    }
    
    // MARK: - Upload Logic
    
    func performUpload() async {
        #if DEBUG
        print("🚀 Starting upload process...")
        #endif
        
        // Small delay to ensure any pending saves complete
        try? await Task.sleep(nanoseconds: 500_000_000) // 0.5 seconds
        
        
        // Clean up any bad events first
        _ = sqliteManager.cleanupBadEvents()
        
        guard deviceManager.isConfigured else {
            return
        }
        
        guard let ingestURL = deviceManager.configuration.ingestURL else {
            return
        }
        
        await MainActor.run {
            self.isUploading = true
        }
        
        defer {
            Task { @MainActor in
                self.isUploading = false
                self.updateUploadStats()
            }
        }
        
        // Get ALL pending uploads from SQLite queue
        let pendingEvents = sqliteManager.dequeueNext(limit: 1000) // Get more events for batching
        
        if pendingEvents.isEmpty {
            return
        }
        
        // Group events by stream name
        let groupedEvents = Dictionary(grouping: pendingEvents) { $0.streamName }
        
        // Track if any uploads succeeded
        var anyUploadSucceeded = false
        
        // Process each stream group as a batch
        for (streamName, events) in groupedEvents {
            let success = await uploadBatchedEvents(streamName: streamName, events: events, to: ingestURL)
            if success {
                anyUploadSucceeded = true
            }
        }
        
        // Update last upload date (attempt)
        saveLastUploadDate(Date())
        
        // Update last successful sync date if any uploads succeeded
        if anyUploadSucceeded {
            saveLastSuccessfulSyncDate(Date())
        }
        
        // Cleanup old events
        _ = sqliteManager.cleanupOldEvents()
    }
    
    private func uploadBatchedEvents(streamName: String, events: [UploadEvent], to url: URL) async -> Bool {
        
        do {
            let decoder = JSONDecoder()
            
            // Decode and combine data based on stream type
            switch streamName {
            case "ios_healthkit":
                var allMetrics: [HealthKitMetric] = []
                
                // Decode each event and collect all metrics
                for event in events {
                    do {
                        let streamData = try decoder.decode(HealthKitStreamData.self, from: event.dataBlob)
                        allMetrics.append(contentsOf: streamData.data)
                    } catch {
                        sqliteManager.incrementRetry(id: event.id)
                    }
                }
                
                // Create combined stream data
                if !allMetrics.isEmpty {
                    let combinedData = HealthKitStreamData(
                        deviceId: DeviceManager.shared.configuration.deviceId,
                        metrics: allMetrics
                    )
                    
                    // Upload combined data
                    let response = try await networkManager.uploadData(
                        combinedData,
                        deviceToken: deviceManager.configuration.deviceToken,
                        endpoint: url
                    )
                    
                    // Mark all events as complete
                    for event in events {
                        handleSuccessfulUpload(event: event, response: response)
                    }
                    
                    return true
                } else {
                    return false
                }
                
            case "ios_location":
                var allLocations: [LocationData] = []
                
                // Decode each event and collect all locations
                for event in events {
                    do {
                        let streamData = try decoder.decode(CoreLocationStreamData.self, from: event.dataBlob)
                        allLocations.append(contentsOf: streamData.data)
                    } catch {
                        sqliteManager.incrementRetry(id: event.id)
                    }
                }
                
                // Create combined stream data
                if !allLocations.isEmpty {
                    let combinedData = CoreLocationStreamData(
                        deviceId: DeviceManager.shared.configuration.deviceId,
                        locations: allLocations
                    )
                    
                    // Upload combined data
                    let response = try await networkManager.uploadData(
                        combinedData,
                        deviceToken: deviceManager.configuration.deviceToken,
                        endpoint: url
                    )
                    
                    // Mark all events as complete
                    for event in events {
                        handleSuccessfulUpload(event: event, response: response)
                    }
                    
                    return true
                } else {
                    return false
                }
                
            case "ios_mic":
                var allChunks: [AudioChunk] = []
                
                // Decode each event and collect all chunks
                for event in events {
                    do {
                        let streamData = try decoder.decode(AudioStreamData.self, from: event.dataBlob)
                        allChunks.append(contentsOf: streamData.data)
                    } catch {
                        sqliteManager.incrementRetry(id: event.id)
                    }
                }
                
                // Create combined stream data
                if !allChunks.isEmpty {
                    let combinedData = AudioStreamData(
                        deviceId: DeviceManager.shared.configuration.deviceId,
                        chunks: allChunks
                    )
                    
                    // Upload combined data
                    let response = try await networkManager.uploadData(
                        combinedData,
                        deviceToken: deviceManager.configuration.deviceToken,
                        endpoint: url
                    )
                    
                    // Mark all events as complete
                    for event in events {
                        handleSuccessfulUpload(event: event, response: response)
                    }
                    
                    return true
                } else {
                    return false
                }
                
            default:
                for event in events {
                    sqliteManager.incrementRetry(id: event.id)
                }
                return false
            }
            
        } catch {
            for event in events {
                handleFailedUpload(event: event, error: error)
            }
            return false
        }
    }
    
    
    private func handleSuccessfulUpload(event: UploadEvent, response: UploadResponse) {
        // Mark as complete in SQLite
        sqliteManager.markAsComplete(id: event.id)
    }
    
    private func handleFailedUpload(event: UploadEvent, error: Error) {
        // Increment retry count
        sqliteManager.incrementRetry(id: event.id)
        
        // Log specific error types
        if let networkError = error as? NetworkError {
            switch networkError {
            case .timeout:
                break
            case .noConnection:
                break
            case .invalidToken:
                stopPeriodicUploads()
            default:
                break
            }
        }
    }
    
    // MARK: - Background Tasks
    
    private func registerBackgroundTasks() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: backgroundTaskIdentifier, using: nil) { task in
            self.handleBackgroundTask(task: task as! BGProcessingTask)
        }
    }
    
    private func scheduleBackgroundUpload() {
        let request = BGProcessingTaskRequest(identifier: backgroundTaskIdentifier)
        request.requiresNetworkConnectivity = true
        request.requiresExternalPower = false
        request.earliestBeginDate = Date(timeIntervalSinceNow: uploadInterval)
        
        do {
            try BGTaskScheduler.shared.submit(request)
            print("Background upload task scheduled")
        } catch {
            print("Failed to schedule background task: \(error)")
        }
    }
    
    private func handleBackgroundTask(task: BGProcessingTask) {
        // Schedule next background task
        scheduleBackgroundUpload()
        
        // Create a task to perform upload
        let uploadTask = Task {
            await performUpload()
        }
        
        // Set expiration handler
        task.expirationHandler = {
            uploadTask.cancel()
        }
        
        // Notify completion when done
        Task {
            _ = await uploadTask.result
            task.setTaskCompleted(success: true)
        }
    }
    
    // MARK: - Manual Upload
    
    func triggerManualUpload() async {
        await performUpload()
    }
    
    // MARK: - Statistics
    
    func updateUploadStats() {
        let stats = sqliteManager.getQueueStats()
        let counts = sqliteManager.getStreamCounts()
        
        Task { @MainActor in
            self.uploadStats = (stats.pending, stats.failed, stats.total)
            self.streamCounts = counts
        }
    }
    
    func getQueueSizeString() -> String {
        let stats = sqliteManager.getQueueStats()
        let bytes = stats.totalSize
        
        if bytes < 1024 {
            return "\(bytes) B"
        } else if bytes < 1024 * 1024 {
            return String(format: "%.1f KB", Double(bytes) / 1024.0)
        } else {
            return String(format: "%.1f MB", Double(bytes) / (1024.0 * 1024.0))
        }
    }
    
    // MARK: - Persistence
    
    private func loadLastUploadDate() {
        if let timestamp = UserDefaults.standard.object(forKey: lastUploadKey) as? TimeInterval {
            let date = Date(timeIntervalSince1970: timestamp)
            Task { @MainActor in
                self.lastUploadDate = date
            }
        }
    }
    
    private func loadLastSuccessfulSyncDate() {
        if let timestamp = UserDefaults.standard.object(forKey: lastSuccessfulSyncKey) as? TimeInterval {
            let date = Date(timeIntervalSince1970: timestamp)
            Task { @MainActor in
                self.lastSuccessfulSyncDate = date
            }
        }
    }
    
    private func saveLastUploadDate(_ date: Date) {
        Task { @MainActor in
            lastUploadDate = date
        }
        UserDefaults.standard.set(date.timeIntervalSince1970, forKey: lastUploadKey)
    }
    
    private func saveLastSuccessfulSyncDate(_ date: Date) {
        Task { @MainActor in
            lastSuccessfulSyncDate = date
        }
        UserDefaults.standard.set(date.timeIntervalSince1970, forKey: lastSuccessfulSyncKey)
    }
    
    // MARK: - Network Monitoring
    
    func handleNetworkChange(isConnected: Bool) {
        if isConnected && deviceManager.isConfigured {
            // Network restored, trigger upload
            Task {
                await performUpload()
            }
        }
    }
}