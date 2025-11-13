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

    // MARK: - Dependencies
    private let configProvider: ConfigurationProvider
    private let storageProvider: StorageProvider
    private let networkManager: NetworkManager

    private var uploadTimer: ReliableTimer?
    private let uploadInterval: TimeInterval = 300 // 5 minutes
    private let backgroundTaskIdentifier = "com.ariata.ios.sync"

    private var statsUpdateTimer: ReliableTimer?

    private let lastUploadKey = "com.ariata.lastUploadDate"
    private let lastSuccessfulSyncKey = "com.ariata.lastSuccessfulSyncDate"

    /// Initialize with dependency injection
    init(configProvider: ConfigurationProvider,
         storageProvider: StorageProvider,
         networkManager: NetworkManager) {
        self.configProvider = configProvider
        self.storageProvider = storageProvider
        self.networkManager = networkManager

        loadLastUploadDate()
        loadLastSuccessfulSyncDate()
        registerBackgroundTasks()
        updateUploadStats()
        setupLowPowerModeObserver()
    }

    /// Legacy singleton initializer - uses default dependencies
    private convenience init() {
        self.init(
            configProvider: DeviceManager.shared,
            storageProvider: SQLiteManager.shared,
            networkManager: NetworkManager.shared
        )
    }
    
    // MARK: - Timer Management
    
    func startPeriodicUploads() {
        stopPeriodicUploads()

        // Schedule timer for 5-minute intervals
        uploadTimer = ReliableTimer.builder()
            .interval(uploadInterval)
            .qos(.userInitiated)
            .handler { [weak self] in
                Task {
                    await self?.performUpload()
                }
            }
            .build()

        // Schedule stats update timer for every 2 seconds
        statsUpdateTimer = ReliableTimer.builder()
            .interval(2.0)
            .qos(.utility)
            .handler { [weak self] in
                self?.updateUploadStats()
            }
            .build()

        // Perform initial upload
        Task {
            await performUpload()
        }

        // Schedule background task
        scheduleBackgroundUpload()
    }

    func stopPeriodicUploads() {
        uploadTimer?.cancel()
        uploadTimer = nil
        statsUpdateTimer?.cancel()
        statsUpdateTimer = nil
    }
    
    // MARK: - Upload Logic
    
    func performUpload() async {
        #if DEBUG
        print("🚀 Starting upload process...")
        #endif

        // Check if Low Power Mode is enabled - pause uploads to save battery
        if ProcessInfo.processInfo.isLowPowerModeEnabled {
            #if DEBUG
            print("⚡️ Low Power Mode enabled - skipping upload to save battery")
            #endif
            return
        }

        // Small delay to ensure any pending saves complete
        try? await Task.sleep(nanoseconds: 500_000_000) // 0.5 seconds


        // Clean up any bad events first
        _ = storageProvider.cleanupBadEvents()

        guard configProvider.isConfigured else {
            return
        }

        guard let ingestURL = configProvider.ingestURL else {
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
        let pendingEvents = storageProvider.dequeueNext(limit: 1000) // Get more events for batching
        
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
        _ = storageProvider.cleanupOldEvents()
    }
    
    private func uploadBatchedEvents(streamName: String, events: [UploadEvent], to url: URL) async -> Bool {
        // Get the appropriate processor for this stream type
        guard let processor = StreamProcessorFactory.processor(for: streamName) else {
            print("⚠️ No processor found for stream: \(streamName)")
            for event in events {
                storageProvider.incrementRetry(id: event.id)
            }
            return false
        }

        // Use type-erased processing based on stream type
        switch streamName {
        case "ios_healthkit":
            let healthKitProcessor = processor as! HealthKitStreamProcessor
            return await uploadWithProcessor(processor: healthKitProcessor, events: events, to: url)

        case "ios_location":
            let locationProcessor = processor as! LocationStreamProcessor
            return await uploadWithProcessor(processor: locationProcessor, events: events, to: url)

        case "ios_mic":
            let audioProcessor = processor as! AudioStreamProcessor
            return await uploadWithProcessor(processor: audioProcessor, events: events, to: url)

        default:
            for event in events {
                storageProvider.incrementRetry(id: event.id)
            }
            return false
        }
    }

    /// Generic upload method that works with any stream processor
    private func uploadWithProcessor<P: StreamDataProcessor>(processor: P, events: [UploadEvent], to url: URL) async -> Bool {
        do {
            var allItems: [P.DataType] = []

            // Decode each event and collect all items
            for event in events {
                do {
                    let items = try processor.decode(event.dataBlob)
                    allItems.append(contentsOf: items)
                } catch {
                    print("⚠️ Failed to decode event \(event.id): \(error)")
                    storageProvider.incrementRetry(id: event.id)
                }
            }

            // If we have items, combine and upload
            guard !allItems.isEmpty else {
                return false
            }

            let combinedData = processor.combine(allItems, deviceId: configProvider.deviceId)

            // Upload combined data
            let response = try await networkManager.uploadData(
                combinedData,
                deviceToken: configProvider.deviceToken,
                endpoint: url
            )

            // Mark all events as complete
            for event in events {
                handleSuccessfulUpload(event: event, response: response)
            }

            return true

        } catch {
            for event in events {
                handleFailedUpload(event: event, error: error)
            }
            return false
        }
    }

    private func handleSuccessfulUpload(event: UploadEvent, response: UploadResponse) {
        // Mark as complete in SQLite
        storageProvider.markAsComplete(id: event.id)
    }

    private func handleFailedUpload(event: UploadEvent, error: Error) {
        // Increment retry count
        storageProvider.incrementRetry(id: event.id)
        
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
        let stats = storageProvider.getQueueStats()
        let counts = storageProvider.getStreamCounts()

        Task { @MainActor in
            self.uploadStats = (stats.pending, stats.failed, stats.total)
            self.streamCounts = counts
        }
    }

    func getQueueSizeString() -> String {
        let stats = storageProvider.getQueueStats()
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
    
    // MARK: - Low Power Mode Monitoring

    private func setupLowPowerModeObserver() {
        // Observe Low Power Mode changes
        NotificationCenter.default.addObserver(
            forName: Notification.Name.NSProcessInfoPowerStateDidChange,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            let isLowPowerModeEnabled = ProcessInfo.processInfo.isLowPowerModeEnabled

            #if DEBUG
            print("⚡️ Low Power Mode changed: \(isLowPowerModeEnabled ? "ENABLED" : "DISABLED")")
            #endif

            // When Low Power Mode is disabled, trigger immediate upload
            if !isLowPowerModeEnabled, let self = self {
                #if DEBUG
                print("⚡️ Low Power Mode disabled - triggering queued uploads")
                #endif

                Task {
                    await self.performUpload()
                }
            }
        }
    }

    // MARK: - Network Monitoring

    func handleNetworkChange(isConnected: Bool) {
        if isConnected && configProvider.isConfigured {
            // Network restored, trigger upload
            Task {
                await performUpload()
            }
        }
    }
}