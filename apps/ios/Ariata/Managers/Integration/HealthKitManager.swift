//
//  HealthKitManager.swift
//  Ariata
//
//  Manages HealthKit authorization and data collection
//

import Foundation
import HealthKit
import Combine

class HealthKitManager: ObservableObject {
    static let shared = HealthKitManager()
    
    private let healthStore = HKHealthStore()
    
    @Published var isAuthorized = false
    @Published var authorizationStatus: [String: Bool] = [:]
    @Published var lastSyncDate: Date?
    @Published var isSyncing = false
    
    private let lastSyncKey = "com.ariata.healthkit.lastSync"
    private var healthTimer: Timer?
    
    // Anchors for incremental sync
    var anchors: [String: HKQueryAnchor] = [:]
    private let anchorKeyPrefix = "com.ariata.healthkit.anchor."
    
    // Define all HealthKit types we need
    private let healthKitTypes: Set<HKSampleType> = [
        HKQuantityType.quantityType(forIdentifier: .heartRate)!,
        HKQuantityType.quantityType(forIdentifier: .stepCount)!,
        HKQuantityType.quantityType(forIdentifier: .activeEnergyBurned)!,
        HKQuantityType.quantityType(forIdentifier: .heartRateVariabilitySDNN)!,
        HKQuantityType.quantityType(forIdentifier: .distanceWalkingRunning)!,
        HKQuantityType.quantityType(forIdentifier: .restingHeartRate)!,
        HKCategoryType.categoryType(forIdentifier: .sleepAnalysis)!
    ]
    
    private init() {
        loadLastSyncDate()
        loadAnchors()
        checkAuthorizationStatus()
    }
    
    // MARK: - Monitoring Control
    
    func startMonitoring() {
        print("🏥 startMonitoring called, isAuthorized: \(isAuthorized)")
        
        guard isAuthorized else {
            print("❌ HealthKit not authorized, cannot start monitoring")
            return
        }
        
        // Check if HealthKit is enabled in configuration
        let isEnabled = DeviceManager.shared.configuration.isStreamEnabled("healthkit")
        guard isEnabled else {
            print("⏸️ HealthKit stream disabled in web app configuration")
            return
        }
        
        // Stop any existing timer
        stopMonitoring()
        
        // Start the 5-minute timer (aligned with sync interval)
        print("⏱️ Creating HealthKit timer with 5-minute interval")
        healthTimer = Timer.scheduledTimer(withTimeInterval: 300.0, repeats: true) { [weak self] _ in
            print("⏰ HealthKit timer fired - collecting data...")
            Task {
                await self?.collectNewData()
            }
        }
        
        // Ensure timer runs in common modes (including background)
        if let timer = healthTimer {
            RunLoop.current.add(timer, forMode: .common)
            print("✅ HealthKit timer added to RunLoop.common")
        }
        
        // Fire immediately to start collecting
        print("🚀 Triggering immediate HealthKit data collection")
        Task {
            await collectNewData()
        }
        
        print("🏥 Started HealthKit monitoring with 5-minute intervals")
    }
    
    func stopMonitoring() {
        if healthTimer != nil {
            print("🛑 Invalidating HealthKit timer")
            healthTimer?.invalidate()
            healthTimer = nil
        }
        
        print("🛑 Stopped HealthKit monitoring")
    }
    
    // MARK: - Authorization
    
    func requestAuthorization() async -> Bool {
        guard HKHealthStore.isHealthDataAvailable() else {
            print("HealthKit is not available on this device")
            return false
        }
        
        do {
            try await healthStore.requestAuthorization(toShare: [], read: healthKitTypes)
            
            // After requesting, test if we actually have access
            let hasAccess = await testHealthKitAccess()
            
            await MainActor.run {
                self.isAuthorized = hasAccess
            }
            
            return hasAccess
        } catch {
            print("HealthKit authorization request failed: \(error)")
            return false
        }
    }
    
    func checkAuthorizationStatus() {
        // Instead of relying on authorization status (which is intentionally vague),
        // we'll try to query recent data to see if we actually have access
        Task {
            print("🏥 Checking HealthKit authorization status...")
            let hasAccess = await testHealthKitAccess()
            print("🏥 HealthKit access test result: \(hasAccess)")
            await MainActor.run {
                self.isAuthorized = hasAccess
            }
        }
    }
    
    func hasAllPermissions() -> Bool {
        return isAuthorized
    }
    
    // Test if we can actually read HealthKit data
    private func testHealthKitAccess() async -> Bool {
        // Try to query a small amount of recent data from each type
        let endDate = Date()
        let startDate = Calendar.current.date(byAdding: .hour, value: -1, to: endDate)!
        let predicate = HKQuery.predicateForSamples(withStart: startDate, end: endDate, options: .strictStartDate)
        
        // We'll test with step count as it's commonly available
        guard let stepType = HKQuantityType.quantityType(forIdentifier: .stepCount) else { return false }
        
        return await withCheckedContinuation { continuation in
            let query = HKSampleQuery(sampleType: stepType, predicate: predicate, limit: 1, sortDescriptors: nil) { _, samples, error in
                if error != nil {
                    // Error might mean no permission
                    continuation.resume(returning: false)
                } else {
                    // No error means we have permission (even if no samples returned)
                    continuation.resume(returning: true)
                }
            }
            
            healthStore.execute(query)
        }
    }
    
    // MARK: - Initial Sync
    
    func performInitialSync(progressHandler: @escaping (Double) -> Void) async -> Bool {
        guard isAuthorized else {
            print("❌ HealthKit not authorized for initial sync")
            return false
        }
        
        await MainActor.run {
            self.isSyncing = true
        }
        
        defer {
            Task { @MainActor in
                self.isSyncing = false
            }
        }
        
        // Get initial sync days from configuration (default to 90 if not configured)
        let initialSyncDays = DeviceManager.shared.configuration.getInitialSyncDays(for: "healthkit")
        
        let endDate = Date()
        let startDate = Calendar.current.date(byAdding: .day, value: -initialSyncDays, to: endDate)!
        
        print("🏁 Starting HealthKit initial sync for \(initialSyncDays) days from \(startDate) to \(endDate)")
        
        var allMetrics: [HealthKitMetric] = []
        let totalTypes = healthKitTypes.count
        var processedTypes = 0
        
        // Collect data for each type
        for type in healthKitTypes {
            if let metrics = await collectData(for: type, from: startDate, to: endDate) {
                allMetrics.append(contentsOf: metrics)
                print("✅ Collected \(metrics.count) metrics for \(getTypeString(for: type))")
            } else {
                print("⚠️ No data for \(getTypeString(for: type))")
            }
            
            processedTypes += 1
            let collectionProgress = Double(processedTypes) / Double(totalTypes) * 0.5 // First 50% for collection
            await MainActor.run {
                progressHandler(collectionProgress)
            }
        }
        
        print("📦 Total metrics collected: \(allMetrics.count)")
        
        // Save to upload queue in batches
        if !allMetrics.isEmpty {
            let batchSize = 1000
            let totalBatches = (allMetrics.count + batchSize - 1) / batchSize
            var savedBatches = 0
            var allSuccess = true
            
            print("📦 Saving \(allMetrics.count) metrics in \(totalBatches) batches of up to \(batchSize) each")
            
            for batchIndex in 0..<totalBatches {
                let startIndex = batchIndex * batchSize
                let endIndex = min((batchIndex + 1) * batchSize, allMetrics.count)
                let batch = Array(allMetrics[startIndex..<endIndex])
                
                print("💾 Saving batch \(batchIndex + 1)/\(totalBatches) with \(batch.count) metrics")
                let success = await saveMetricsToQueue(batch)
                
                if success {
                    savedBatches += 1
                    print("✅ Batch \(batchIndex + 1) saved successfully")
                } else {
                    print("❌ Failed to save batch \(batchIndex + 1)")
                    allSuccess = false
                }
                
                // Update progress (second 50% for saving)
                let saveProgress = 0.5 + (Double(savedBatches) / Double(totalBatches) * 0.5)
                await MainActor.run {
                    progressHandler(saveProgress)
                }
            }
            
            if allSuccess {
                print("✅ All \(totalBatches) batches saved successfully")
                saveLastSyncDate(endDate)
                
                // Set anchors after successful initial sync
                for type in healthKitTypes {
                    let typeKey = getAnchorKey(for: type)
                    // Create a new anchor representing "now" to avoid re-syncing this data
                    let newAnchor = HKQueryAnchor(fromValue: Int.max)
                    anchors[typeKey] = newAnchor
                    saveAnchor(newAnchor, for: typeKey)
                }
                print("✅ Anchors set for future incremental syncs")
            } else {
                print("⚠️ Some batches failed to save")
            }
            
            return allSuccess
        } else {
            print("⚠️ No metrics to save")
            progressHandler(1.0) // Complete if no data
        }
        
        return true
    }
    
    
    // MARK: - Data Collection
    
    private func collectData(for type: HKSampleType, from startDate: Date, to endDate: Date) async -> [HealthKitMetric]? {
        let predicate = HKQuery.predicateForSamples(withStart: startDate, end: endDate, options: .strictStartDate)
        
        return await withCheckedContinuation { continuation in
            if let quantityType = type as? HKQuantityType {
                collectQuantityData(type: quantityType, predicate: predicate) { metrics in
                    continuation.resume(returning: metrics)
                }
            } else if let categoryType = type as? HKCategoryType {
                collectCategoryData(type: categoryType, predicate: predicate) { metrics in
                    continuation.resume(returning: metrics)
                }
            } else {
                continuation.resume(returning: nil)
            }
        }
    }
    
    private func collectNewData(for type: HKSampleType, anchor: HKQueryAnchor?) async -> ([HealthKitMetric], HKQueryAnchor?)? {
        return await withCheckedContinuation { continuation in
            let query = HKAnchoredObjectQuery(
                type: type,
                predicate: nil, // Get all new samples
                anchor: anchor,
                limit: HKObjectQueryNoLimit
            ) { [weak self] query, samplesOrNil, deletedObjectsOrNil, newAnchor, error in
                guard let self = self else {
                    continuation.resume(returning: nil)
                    return
                }
                
                guard let samples = samplesOrNil, error == nil else {
                    if let error = error {
                        print("❌ HealthKit query error for \(type.identifier): \(error)")
                    }
                    continuation.resume(returning: nil)
                    return
                }
                
                var metrics: [HealthKitMetric] = []
                
                // Process quantity samples
                if let quantitySamples = samples as? [HKQuantitySample], 
                   let quantityType = type as? HKQuantityType {
                    metrics = quantitySamples.compactMap { sample in
                        let metricType = self.getMetricType(for: quantityType)
                        let unit = self.getUnit(for: quantityType)
                        let value = self.getValue(from: sample, type: quantityType).roundedForHealthKit(metricType: metricType)
                        
                        var metadata: [String: Any] = [:]
                        if quantityType.identifier == HKQuantityType.quantityType(forIdentifier: .heartRate)!.identifier {
                            metadata["activity_context"] = self.getActivityContext(from: sample)
                        }
                        
                        return HealthKitMetric(
                            timestamp: sample.startDate,
                            metricType: metricType,
                            value: value,
                            unit: unit,
                            metadata: metadata.isEmpty ? nil : metadata
                        )
                    }
                }
                
                // Process category samples
                else if let categorySamples = samples as? [HKCategorySample],
                        let categoryType = type as? HKCategoryType {
                    metrics = categorySamples.map { sample in
                        let metricType = self.getMetricType(for: categoryType)
                        let value = Double(sample.value)
                        
                        var metadata: [String: Any] = [:]
                        if categoryType.identifier == HKCategoryType.categoryType(forIdentifier: .sleepAnalysis)!.identifier {
                            metadata["sleep_state"] = self.getSleepState(from: sample.value)
                            metadata["duration_minutes"] = Int(sample.endDate.timeIntervalSince(sample.startDate) / 60)
                        }
                        
                        return HealthKitMetric(
                            timestamp: sample.startDate,
                            metricType: metricType,
                            value: value,
                            unit: "category",
                            metadata: metadata.isEmpty ? nil : metadata
                        )
                    }
                }
                
                continuation.resume(returning: (metrics, newAnchor))
            }
            
            healthStore.execute(query)
        }
    }
    
    private func collectQuantityData(type: HKQuantityType, predicate: NSPredicate, completion: @escaping ([HealthKitMetric]) -> Void) {
        let query = HKSampleQuery(sampleType: type, predicate: predicate, limit: HKObjectQueryNoLimit, sortDescriptors: nil) { _, samples, error in
            guard let samples = samples as? [HKQuantitySample], error == nil else {
                completion([])
                return
            }
            
            let metrics = samples.map { sample -> HealthKitMetric in
                let metricType = self.getMetricType(for: type)
                let unit = self.getUnit(for: type)
                let value = self.getValue(from: sample, type: type).roundedForHealthKit(metricType: metricType)
                
                var metadata: [String: Any] = [:]
                
                // Add metadata based on type
                if type.identifier == HKQuantityType.quantityType(forIdentifier: .heartRate)!.identifier {
                    metadata["activity_context"] = self.getActivityContext(from: sample)
                }
                
                return HealthKitMetric(
                    timestamp: sample.startDate,
                    metricType: metricType,
                    value: value,
                    unit: unit,
                    metadata: metadata.isEmpty ? nil : metadata
                )
            }
            
            completion(metrics)
        }
        
        healthStore.execute(query)
    }
    
    private func collectCategoryData(type: HKCategoryType, predicate: NSPredicate, completion: @escaping ([HealthKitMetric]) -> Void) {
        let query = HKSampleQuery(sampleType: type, predicate: predicate, limit: HKObjectQueryNoLimit, sortDescriptors: nil) { _, samples, error in
            guard let samples = samples as? [HKCategorySample], error == nil else {
                completion([])
                return
            }
            
            let metrics = samples.map { sample -> HealthKitMetric in
                let metricType = self.getMetricType(for: type)
                let value = Double(sample.value)
                
                var metadata: [String: Any] = [:]
                
                // Add sleep-specific metadata
                if type.identifier == HKCategoryType.categoryType(forIdentifier: .sleepAnalysis)!.identifier {
                    metadata["sleep_state"] = self.getSleepState(from: sample.value)
                    metadata["duration_minutes"] = Int(sample.endDate.timeIntervalSince(sample.startDate) / 60)
                }
                
                return HealthKitMetric(
                    timestamp: sample.startDate,
                    metricType: metricType,
                    value: value,
                    unit: "category",
                    metadata: metadata
                )
            }
            
            completion(metrics)
        }
        
        healthStore.execute(query)
    }
    
    // MARK: - Helper Methods
    
    private func getTypeString(for type: HKSampleType) -> String {
        if let quantityType = type as? HKQuantityType {
            return quantityType.identifier
        } else if let categoryType = type as? HKCategoryType {
            return categoryType.identifier
        }
        return "unknown"
    }
    
    private func getMetricType(for type: HKSampleType) -> String {
        let identifier = getTypeString(for: type)
        
        switch identifier {
        case HKQuantityType.quantityType(forIdentifier: .heartRate)!.identifier:
            return "heart_rate"
        case HKQuantityType.quantityType(forIdentifier: .stepCount)!.identifier:
            return "steps"
        case HKQuantityType.quantityType(forIdentifier: .activeEnergyBurned)!.identifier:
            return "active_energy"
        case HKQuantityType.quantityType(forIdentifier: .heartRateVariabilitySDNN)!.identifier:
            return "heart_rate_variability"
        case HKQuantityType.quantityType(forIdentifier: .distanceWalkingRunning)!.identifier:
            return "distance"
        case HKQuantityType.quantityType(forIdentifier: .restingHeartRate)!.identifier:
            return "resting_heart_rate"
        case HKCategoryType.categoryType(forIdentifier: .sleepAnalysis)!.identifier:
            return "sleep"
        default:
            return "unknown"
        }
    }
    
    private func getUnit(for type: HKQuantityType) -> String {
        switch type.identifier {
        case HKQuantityType.quantityType(forIdentifier: .heartRate)!.identifier,
             HKQuantityType.quantityType(forIdentifier: .restingHeartRate)!.identifier:
            return "bpm"
        case HKQuantityType.quantityType(forIdentifier: .stepCount)!.identifier:
            return "steps"
        case HKQuantityType.quantityType(forIdentifier: .activeEnergyBurned)!.identifier:
            return "kcal"
        case HKQuantityType.quantityType(forIdentifier: .heartRateVariabilitySDNN)!.identifier:
            return "ms"
        case HKQuantityType.quantityType(forIdentifier: .distanceWalkingRunning)!.identifier:
            return "m"
        default:
            return "unknown"
        }
    }
    
    private func getValue(from sample: HKQuantitySample, type: HKQuantityType) -> Double {
        switch type.identifier {
        case HKQuantityType.quantityType(forIdentifier: .heartRate)!.identifier,
             HKQuantityType.quantityType(forIdentifier: .restingHeartRate)!.identifier:
            return sample.quantity.doubleValue(for: HKUnit.count().unitDivided(by: .minute()))
        case HKQuantityType.quantityType(forIdentifier: .stepCount)!.identifier:
            return sample.quantity.doubleValue(for: .count())
        case HKQuantityType.quantityType(forIdentifier: .activeEnergyBurned)!.identifier:
            return sample.quantity.doubleValue(for: .kilocalorie())
        case HKQuantityType.quantityType(forIdentifier: .heartRateVariabilitySDNN)!.identifier:
            return sample.quantity.doubleValue(for: .secondUnit(with: .milli))
        case HKQuantityType.quantityType(forIdentifier: .distanceWalkingRunning)!.identifier:
            return sample.quantity.doubleValue(for: .meter())
        default:
            return 0
        }
    }
    
    private func getActivityContext(from sample: HKQuantitySample) -> String {
        // Check metadata for motion context
        if let metadata = sample.metadata,
           let context = metadata[HKMetadataKeyHeartRateMotionContext] as? NSNumber {
            switch context.intValue {
            case 1: return "resting"
            case 2: return "active"
            default: return "unknown"
            }
        }
        return "unknown"
    }
    
    private func getSleepState(from value: Int) -> String {
        switch value {
        case HKCategoryValueSleepAnalysis.inBed.rawValue:
            return "in_bed"
        case HKCategoryValueSleepAnalysis.asleepUnspecified.rawValue:
            return "asleep"
        case HKCategoryValueSleepAnalysis.awake.rawValue:
            return "awake"
        case HKCategoryValueSleepAnalysis.asleepCore.rawValue:
            return "asleep_core"
        case HKCategoryValueSleepAnalysis.asleepDeep.rawValue:
            return "asleep_deep"
        case HKCategoryValueSleepAnalysis.asleepREM.rawValue:
            return "asleep_rem"
        default:
            // Log the unknown value for debugging
            print("⚠️ Unknown sleep state value: \(value)")
            return "unknown"
        }
    }
    
    // MARK: - Buffered Data Collection
    
    private func collectNewData() async {
        print("🏥 HealthKit timer fired - collecting new data...")
        
        var allMetrics: [HealthKitMetric] = []
        
        // Collect new data for each type using anchored queries
        for type in healthKitTypes {
            let typeKey = getAnchorKey(for: type)
            let anchor = anchors[typeKey]
            
            if let (metrics, newAnchor) = await collectNewData(for: type, anchor: anchor) {
                if !metrics.isEmpty {
                    print("🏥 Found \(metrics.count) new \(type.identifier) samples")
                    allMetrics.append(contentsOf: metrics)
                }
                
                // Update anchor for next query
                if let newAnchor = newAnchor {
                    anchors[typeKey] = newAnchor
                    saveAnchor(newAnchor, for: typeKey)
                }
            }
        }
        
        // Save to SQLite in batches if needed
        if !allMetrics.isEmpty {
            print("🏥 Collected \(allMetrics.count) new health metrics")
            
            // For regular syncs, batch if more than 1000 metrics
            if allMetrics.count > 1000 {
                let batchSize = 1000
                let totalBatches = (allMetrics.count + batchSize - 1) / batchSize
                var allSuccess = true
                
                print("🏥 Saving \(allMetrics.count) metrics in \(totalBatches) batches")
                
                for batchIndex in 0..<totalBatches {
                    let startIndex = batchIndex * batchSize
                    let endIndex = min((batchIndex + 1) * batchSize, allMetrics.count)
                    let batch = Array(allMetrics[startIndex..<endIndex])
                    
                    let success = await saveMetricsToQueue(batch)
                    if !success {
                        allSuccess = false
                    }
                }
                
                if allSuccess {
                    await MainActor.run {
                        self.lastSyncDate = Date()
                    }
                }
            } else {
                // Small enough to save as single batch
                let success = await saveMetricsToQueue(allMetrics)
                if success {
                    await MainActor.run {
                        self.lastSyncDate = Date()
                    }
                }
            }
        } else {
            print("🏥 No new health metrics found")
        }
    }
    
    
    // MARK: - Data Persistence
    
    private func saveMetricsToQueue(_ metrics: [HealthKitMetric]) async -> Bool {
        let streamData = HealthKitStreamData(
            deviceId: DeviceManager.shared.configuration.deviceId,
            metrics: metrics
        )
        
        do {
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601
            let data = try encoder.encode(streamData)
            
            print("💾 Attempting to save HealthKit data (\(data.count) bytes) to SQLite...")
            let success = SQLiteManager.shared.enqueue(streamName: "ios_healthkit", data: data)
            
            if success {
                // Verify it was saved
                SQLiteManager.shared.debugPrintAllEvents()
                
                // Update stats in upload coordinator
                BatchUploadCoordinator.shared.updateUploadStats()
            }
            
            return success
        } catch {
            print("Failed to encode HealthKit data: \(error)")
            return false
        }
    }
    
    private func loadLastSyncDate() {
        if let timestamp = UserDefaults.standard.object(forKey: lastSyncKey) as? TimeInterval {
            lastSyncDate = Date(timeIntervalSince1970: timestamp)
        }
    }
    
    private func saveLastSyncDate(_ date: Date) {
        Task { @MainActor in
            lastSyncDate = date
        }
        UserDefaults.standard.set(date.timeIntervalSince1970, forKey: lastSyncKey)
    }
    
    // MARK: - Anchor Management
    
    private func getAnchorKey(for type: HKSampleType) -> String {
        return anchorKeyPrefix + type.identifier
    }
    
    private func loadAnchors() {
        for type in healthKitTypes {
            let key = getAnchorKey(for: type)
            if let anchorData = UserDefaults.standard.data(forKey: key),
               let anchor = try? NSKeyedUnarchiver.unarchivedObject(ofClass: HKQueryAnchor.self, from: anchorData) {
                anchors[key] = anchor
            }
        }
    }
    
    private func saveAnchor(_ anchor: HKQueryAnchor, for key: String) {
        if let anchorData = try? NSKeyedArchiver.archivedData(withRootObject: anchor, requiringSecureCoding: true) {
            UserDefaults.standard.set(anchorData, forKey: key)
        }
    }
}