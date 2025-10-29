//
//  LocationManager.swift
//  Ariata
//
//  Handles location tracking and permissions
//

import Foundation
import CoreLocation
import Combine
import UIKit

class LocationManager: NSObject, ObservableObject {
    static let shared = LocationManager()

    // MARK: - Constants
    private let samplingIntervalSeconds = 10.0
    private let healthCheckIntervalSeconds = 30.0
    private let locationFreshnessThresholdSeconds = 30.0

    @Published var authorizationStatus: CLAuthorizationStatus = .notDetermined
    @Published var isTracking = false
    @Published var lastSaveDate: Date?

    // MARK: - Dependencies
    private let configProvider: ConfigurationProvider
    private let storageProvider: StorageProvider
    private let dataUploader: DataUploader

    private let locationManager = CLLocationManager()
    private var locationContinuation: AsyncStream<CLLocation>.Continuation?
    private var locationTimer: ReliableTimer?
    private var lastLocation: CLLocation?
    private var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    private let timerQueue = DispatchQueue(label: "com.ariata.location.timer", qos: .userInitiated)

    /// Initialize with dependency injection
    init(configProvider: ConfigurationProvider,
         storageProvider: StorageProvider,
         dataUploader: DataUploader) {
        self.configProvider = configProvider
        self.storageProvider = storageProvider
        self.dataUploader = dataUploader

        super.init()

        locationManager.delegate = self
        // Match the requirements: kCLLocationAccuracyNearestTenMeters
        locationManager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
        locationManager.distanceFilter = kCLDistanceFilterNone
        locationManager.allowsBackgroundLocationUpdates = true
        locationManager.pausesLocationUpdatesAutomatically = false
        locationManager.showsBackgroundLocationIndicator = true
        locationManager.activityType = .fitness // Optimized for continuous tracking

        // Check initial status
        authorizationStatus = locationManager.authorizationStatus

        // Register with centralized health check coordinator
        HealthCheckCoordinator.shared.register(self)
    }

    /// Legacy singleton initializer - uses default dependencies
    private override convenience init() {
        self.init(
            configProvider: DeviceManager.shared,
            storageProvider: SQLiteManager.shared,
            dataUploader: BatchUploadCoordinator.shared
        )
    }

    deinit {
        locationTimer?.cancel()
        locationTimer = nil
        HealthCheckCoordinator.shared.unregister(self)
    }
    
    // MARK: - Authorization

    func requestAuthorization() async -> Bool {
        return await withCheckedContinuation { continuation in
            // If already authorized (either When In Use or Always), return true
            if authorizationStatus == .authorizedAlways || authorizationStatus == .authorizedWhenInUse {
                continuation.resume(returning: true)
                return
            }

            // Request authorization on main thread
            Task { @MainActor in
                locationManager.requestAlwaysAuthorization()
            }

            // Wait for the authorization dialog to be handled
            // iOS will first grant "When In Use", then prompt for "Always" later
            Task {
                try? await Task.sleep(nanoseconds: 3_000_000_000) // 3 seconds

                // Accept either When In Use or Always as success
                // iOS will prompt for upgrade to Always automatically later
                let granted = authorizationStatus == .authorizedAlways ||
                              authorizationStatus == .authorizedWhenInUse
                continuation.resume(returning: granted)
            }
        }
    }

    func checkAuthorizationStatus() {
        authorizationStatus = locationManager.authorizationStatus
    }

    var hasPermission: Bool {
        // Accept either When In Use or Always
        // We request Always, but iOS may grant When In Use first
        return authorizationStatus == .authorizedAlways ||
               authorizationStatus == .authorizedWhenInUse
    }
    
    // MARK: - Location Tracking
    
    func startTracking() {
        guard authorizationStatus == .authorizedAlways else {
            print("❌ Location tracking requires Always authorization")
            return
        }
        
        // Check if location is enabled in configuration
        let isEnabled = configProvider.isStreamEnabled("location")
        guard isEnabled else {
            print("⏸️ Location stream disabled in web app configuration")
            return
        }
        
        print("📍 Starting location tracking")
        isTracking = true
        locationManager.startUpdatingLocation()
        locationManager.startMonitoringSignificantLocationChanges()
        
        // Start the 10-second timer for location sampling
        print("⏱️ Starting location timer with 10-second interval")
        startLocationTimer()
    }
    
    func stopTracking() {
        print("📍 Stopping location tracking")
        isTracking = false
        locationManager.stopUpdatingLocation()
        locationManager.stopMonitoringSignificantLocationChanges()

        // Stop the timer
        stopLocationTimer()
    }
}

// MARK: - CLLocationManagerDelegate

extension LocationManager: CLLocationManagerDelegate {
    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        DispatchQueue.main.async {
            self.authorizationStatus = manager.authorizationStatus
        }
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        // Store the most recent location for sampling
        if let location = locations.last {
            lastLocation = location
            locationContinuation?.yield(location)
        }
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        print("❌ Location manager error: \(error.localizedDescription)")
    }

    func locationManagerDidPauseLocationUpdates(_ manager: CLLocationManager) {
        print("⚠️ iOS paused location updates - this is expected in low-power scenarios")
        // Don't restart immediately - iOS will resume when appropriate
        // Health check will handle recovery if needed
    }

    func locationManagerDidResumeLocationUpdates(_ manager: CLLocationManager) {
        print("✅ iOS resumed location updates")
        // Ensure timer is running
        if isTracking && locationTimer == nil {
            print("🔄 Restarting location timer after iOS resume")
            startLocationTimer()
        }
    }
}

// MARK: - Location Sampling

extension LocationManager {
    private func startLocationTimer() {
        // Cancel any existing timer
        locationTimer?.cancel()

        print("⏱️ Creating location timer that fires every \(samplingIntervalSeconds) seconds")
        locationTimer = ReliableTimer.builder()
            .interval(samplingIntervalSeconds)
            .queue(timerQueue)
            .handler { [weak self] in
                print("⏰ Location timer fired - sampling location...")
                self?.sampleCurrentLocation()
            }
            .build()

        print("✅ Location timer started on dedicated queue")
    }

    private func stopLocationTimer() {
        locationTimer?.cancel()
        locationTimer = nil
        print("⏹️ Location timer stopped")
    }

    private func sampleCurrentLocation() {
        guard let location = lastLocation else {
            print("⚠️ No location available to sample")
            return
        }

        // Only sample if the location is fresh (within threshold)
        let age = Date().timeIntervalSince(location.timestamp)
        guard age < locationFreshnessThresholdSeconds else {
            print("⚠️ Location too old to sample (age: \(age) seconds)")
            return
        }

        // Create location data
        let locationData = LocationData(location: location)

        print("📍 Sampled location: lat=\(location.coordinate.latitude), lon=\(location.coordinate.longitude), speed=\(location.speed), course=\(location.course)")

        // Save directly to SQLite
        saveLocationSample(locationData)
    }
    
    private func saveLocationSample(_ locationData: LocationData) {
        // Begin background task to ensure save completes
        beginBackgroundTask()

        let deviceId = configProvider.deviceId

        // Attempt to save with retry mechanism
        let result = saveWithRetry(location: locationData, deviceId: deviceId, maxAttempts: 3)

        switch result {
        case .success:
            print("✅ Saved location sample to SQLite queue")
            Task { @MainActor in
                self.lastSaveDate = Date()
            }
            dataUploader.updateUploadStats()

        case .failure(let error):
            ErrorLogger.shared.log(error, deviceId: deviceId)
        }

        endBackgroundTask()
    }

    /// Attempts to save location sample with exponential backoff retry
    private func saveWithRetry(location: LocationData, deviceId: String, maxAttempts: Int) -> Result<Void, AnyDataCollectionError> {
        let streamData = CoreLocationStreamData(
            deviceId: deviceId,
            locations: [location]
        )

        for attempt in 1...maxAttempts {
            // Encode the data
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601

            let data: Data
            do {
                data = try encoder.encode(streamData)
            } catch {
                let encodingError = DataEncodingError(
                    streamType: .location,
                    underlyingError: error,
                    dataSize: nil
                )
                return .failure(AnyDataCollectionError(encodingError))
            }

            // Attempt to save to SQLite
            let success = storageProvider.enqueue(streamName: "ios_location", data: data)

            if success {
                if attempt > 1 {
                    ErrorLogger.shared.logSuccessfulRetry(streamType: .location, attemptNumber: attempt)
                }
                return .success
            }

            // If not last attempt, wait before retrying
            if attempt < maxAttempts {
                let delay = Double(attempt) * 0.5  // 0.5s, 1.0s backoff
                Thread.sleep(forTimeInterval: delay)
            }
        }

        // All attempts failed
        let storageError = StorageError(
            streamType: .location,
            reason: "Failed to enqueue to SQLite after \(maxAttempts) attempts",
            attemptNumber: maxAttempts
        )
        return .failure(AnyDataCollectionError(storageError))
    }
    
    private func beginBackgroundTask() {
        backgroundTask = UIApplication.shared.beginBackgroundTask { [weak self] in
            self?.endBackgroundTask()
        }
    }

    private func endBackgroundTask() {
        if backgroundTask != .invalid {
            UIApplication.shared.endBackgroundTask(backgroundTask)
            backgroundTask = .invalid
        }
    }
}

// MARK: - HealthCheckable

extension LocationManager: HealthCheckable {
    var healthCheckName: String {
        "LocationManager"
    }

    func performHealthCheck() -> HealthStatus {
        // Check if stream is enabled
        guard configProvider.isStreamEnabled("location") else {
            return .disabled
        }

        // Check permission
        guard hasPermission else {
            return .unhealthy(reason: "Location permission not granted or not set to Always")
        }

        let shouldBeTracking = true
        let actuallyTracking = isTracking

        if shouldBeTracking && !actuallyTracking {
            // Attempt recovery
            stopTracking()
            startTracking()
            return .unhealthy(reason: "Tracking stopped unexpectedly, restarting")
        } else if !shouldBeTracking && actuallyTracking {
            stopTracking()
            return .healthy
        } else if actuallyTracking {
            // Verify we're getting fresh location updates
            if let lastLoc = lastLocation {
                let age = Date().timeIntervalSince(lastLoc.timestamp)
                if age > 60 {
                    stopTracking()
                    startTracking()
                    return .unhealthy(reason: "Location data stale (\(Int(age))s old), restarting")
                }
            }

            // Verify timer is still running
            if locationTimer == nil {
                startLocationTimer()
                return .unhealthy(reason: "Timer stopped unexpectedly, restarting")
            }

            return .healthy
        }

        return .healthy
    }
}