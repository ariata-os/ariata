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

    private let locationManager = CLLocationManager()
    private var locationContinuation: AsyncStream<CLLocation>.Continuation?
    private var locationTimer: DispatchSourceTimer?
    private var healthCheckTimer: DispatchSourceTimer?
    private var lastLocation: CLLocation?
    private var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    private let timerQueue = DispatchQueue(label: "com.ariata.location.timer", qos: .userInitiated)
    
    override init() {
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

        // Start health check timer
        startHealthCheckTimer()
    }

    deinit {
        locationTimer?.cancel()
        locationTimer = nil
        healthCheckTimer?.cancel()
        healthCheckTimer = nil
    }
    
    // MARK: - Authorization
    
    func requestAuthorization() async -> Bool {
        return await withCheckedContinuation { continuation in
            // If already authorized, return true
            if authorizationStatus == .authorizedAlways {
                continuation.resume(returning: true)
                return
            }
            
            // Request authorization on main thread
            Task { @MainActor in
                locationManager.requestAlwaysAuthorization()
            }
            
            // Wait a bit for the authorization dialog to be handled
            Task {
                try? await Task.sleep(nanoseconds: 3_000_000_000) // 3 seconds
                continuation.resume(returning: authorizationStatus == .authorizedAlways)
            }
        }
    }
    
    func checkAuthorizationStatus() {
        authorizationStatus = locationManager.authorizationStatus
    }
    
    var hasPermission: Bool {
        return authorizationStatus == .authorizedAlways
    }
    
    // MARK: - Location Tracking
    
    func startTracking() {
        guard authorizationStatus == .authorizedAlways else {
            print("❌ Location tracking requires Always authorization")
            return
        }
        
        // Check if location is enabled in configuration
        let isEnabled = DeviceManager.shared.configuration.isStreamEnabled("location")
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

        // Use DispatchSourceTimer for more reliable background execution (same as AudioManager)
        print("⏱️ Creating location timer that fires every \(samplingIntervalSeconds) seconds")
        let timer = DispatchSource.makeTimerSource(queue: timerQueue)
        timer.schedule(deadline: .now(), repeating: samplingIntervalSeconds)
        timer.setEventHandler { [weak self] in
            print("⏰ Location timer fired - sampling location...")
            self?.sampleCurrentLocation()
        }
        timer.resume()
        locationTimer = timer

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
        
        // Create the stream data with single location
        let streamData = CoreLocationStreamData(
            deviceId: DeviceManager.shared.configuration.deviceId,
            locations: [locationData]
        )
        
        // Encode and save to SQLite
        do {
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601
            let data = try encoder.encode(streamData)
            
            let success = SQLiteManager.shared.enqueue(streamName: "ios_location", data: data)
            
            if success {
                print("✅ Saved location sample to SQLite queue")
                Task { @MainActor in
                    self.lastSaveDate = Date()
                }
                
                // Update stats in upload coordinator
                BatchUploadCoordinator.shared.updateUploadStats()
            } else {
                print("❌ Failed to save location sample to SQLite queue")
            }
        } catch {
            print("❌ Failed to encode location data: \(error)")
        }
        
        endBackgroundTask()
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

// MARK: - Health Check

extension LocationManager {
    private func startHealthCheckTimer() {
        // Run health check on main queue to avoid thread safety issues (same as AudioManager)
        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(deadline: .now() + healthCheckIntervalSeconds, repeating: healthCheckIntervalSeconds)
        timer.setEventHandler { [weak self] in
            self?.performHealthCheck()
        }
        timer.resume()
        healthCheckTimer = timer
    }

    func performHealthCheck() {
        let shouldBeTracking = hasPermission && DeviceManager.shared.configuration.isStreamEnabled("location")
        let actuallyTracking = isTracking

        if shouldBeTracking && !actuallyTracking {
            print("🔄 Health check: Location should be tracking but isn't - restarting")
            stopTracking()  // Clean up any bad state
            startTracking() // Fresh start
        } else if !shouldBeTracking && actuallyTracking {
            print("🔄 Health check: Location shouldn't be tracking - stopping")
            stopTracking()
        } else if shouldBeTracking && actuallyTracking {
            // Verify we're getting fresh location updates
            if let lastLoc = lastLocation {
                let age = Date().timeIntervalSince(lastLoc.timestamp)
                if age > 60 {
                    print("⚠️ Health check: Location data is stale (age: \(age)s) - restarting")
                    stopTracking()
                    startTracking()
                }
            }

            // Verify timer is still running
            if locationTimer == nil {
                print("⚠️ Health check: Timer stopped unexpectedly - restarting")
                startLocationTimer()
            }
        }
    }
}