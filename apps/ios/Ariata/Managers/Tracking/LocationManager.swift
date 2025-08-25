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
    
    @Published var authorizationStatus: CLAuthorizationStatus = .notDetermined
    @Published var isTracking = false
    @Published var lastSaveDate: Date?
    
    private let locationManager = CLLocationManager()
    private var locationContinuation: AsyncStream<CLLocation>.Continuation?
    private var locationTimer: Timer?
    private var lastLocation: CLLocation?
    private var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    
    override init() {
        super.init()
        locationManager.delegate = self
        // Match the requirements: kCLLocationAccuracyNearestTenMeters
        locationManager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
        locationManager.distanceFilter = kCLDistanceFilterNone
        locationManager.allowsBackgroundLocationUpdates = true
        locationManager.pausesLocationUpdatesAutomatically = false
        locationManager.showsBackgroundLocationIndicator = true
        
        // Check initial status
        authorizationStatus = locationManager.authorizationStatus
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
}

// MARK: - Location Sampling

extension LocationManager {
    private func startLocationTimer() {
        // Invalidate any existing timer
        locationTimer?.invalidate()
        
        // Create a new timer that fires every 10 seconds
        print("⏱️ Creating location timer that fires every 10 seconds")
        locationTimer = Timer.scheduledTimer(withTimeInterval: 10.0, repeats: true) { [weak self] _ in
            print("⏰ Location timer fired - sampling location...")
            self?.sampleCurrentLocation()
        }
        
        // Ensure timer runs in common modes (including background)
        if let timer = locationTimer {
            RunLoop.current.add(timer, forMode: .common)
            print("✅ Location timer added to RunLoop.common")
        }
        
        // Fire immediately to capture the first location
        print("🚀 Triggering immediate location sample")
        locationTimer?.fire()
    }
    
    private func stopLocationTimer() {
        locationTimer?.invalidate()
        locationTimer = nil
    }
    
    private func sampleCurrentLocation() {
        guard let location = lastLocation else {
            print("⚠️ No location available to sample")
            return
        }
        
        // Only sample if the location is fresh (within last 30 seconds)
        let age = Date().timeIntervalSince(location.timestamp)
        guard age < 30 else {
            print("⚠️ Location too old to sample (age: \(age) seconds)")
            return
        }
        
        // Create location data
        let locationData = LocationData(location: location)
        
        print("📍 Sampled location: lat=\(location.coordinate.latitude), lon=\(location.coordinate.longitude)")
        
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