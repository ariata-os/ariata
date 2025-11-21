//
//  PermissionMonitor.swift
//  Ariata
//
//  Monitors permission changes and notifies user of downgrades
//

import Foundation
import Combine
import SwiftUI
import CoreLocation
import AVFoundation

/// Monitors all permission changes and alerts user to downgrades
class PermissionMonitor: ObservableObject {
    static let shared = PermissionMonitor()

    // MARK: - Published Properties

    /// Current permission issues requiring user attention
    @Published var currentIssues: [PermissionIssue] = []

    /// Has any active permission issue
    var hasIssues: Bool {
        !currentIssues.isEmpty
    }

    // MARK: - Dependencies

    private let locationManager = LocationManager.shared
    private let audioManager = AudioManager.shared
    private let healthKitManager = HealthKitManager.shared

    private var cancellables = Set<AnyCancellable>()

    // MARK: - Previous States

    private var previousLocationAuthorized: Bool?
    private var previousAudioAuthorized: Bool?
    private var previousHealthKitAuthorized: Bool?

    // MARK: - Initialization

    private init() {
        setupPermissionMonitoring()

        // Store initial states
        previousLocationAuthorized = locationManager.hasAlwaysPermission
        previousAudioAuthorized = audioManager.hasPermission
        previousHealthKitAuthorized = healthKitManager.isAuthorized
    }

    // MARK: - Setup

    private func setupPermissionMonitoring() {
        // Monitor Location permission changes
        locationManager.$authorizationStatus
            .removeDuplicates()
            .sink { [weak self] status in
                self?.handleLocationPermissionChange(status)
            }
            .store(in: &cancellables)

        // Monitor Microphone permission changes
        // Note: microphoneAuthorizationStatus is AVAudioSession.RecordPermission (UInt enum)
        // We monitor it by checking periodically since it's not a standard observable type
        Timer.publish(every: 5.0, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                self?.checkMicrophonePermission()
            }
            .store(in: &cancellables)

        // Monitor HealthKit permission changes (check every 30 seconds)
        Timer.publish(every: 30.0, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                self?.checkHealthKitPermission()
            }
            .store(in: &cancellables)
    }

    // MARK: - Permission Change Handlers

    private func handleLocationPermissionChange(_ status: CLAuthorizationStatus) {
        let isAuthorized = (status == .authorizedAlways)

        // Check for downgrade (was authorized, now not)
        if let wasAuthorized = previousLocationAuthorized,
           wasAuthorized && !isAuthorized {

            #if DEBUG
            print("Location permission downgraded: \(status)")
            #endif

            // Stop tracking immediately
            if locationManager.isTracking {
                locationManager.stopTracking()
            }

            // Create issue
            let issue = PermissionIssue(
                type: .location,
                message: "Location permission changed to '\(statusDescription(status))'. Background tracking requires 'Always' permission.",
                action: "Open Settings"
            )

            addIssue(issue)
        } else if !isAuthorized && previousLocationAuthorized == nil {
            // First check and not authorized - add issue but don't log as downgrade
            let issue = PermissionIssue(
                type: .location,
                message: "Location permission required. Please enable 'Always' permission in Settings.",
                action: "Open Settings"
            )
            addIssue(issue)
        } else if isAuthorized {
            // Permission restored - remove issue
            removeIssue(type: .location)
        }

        previousLocationAuthorized = isAuthorized
    }

    private func checkMicrophonePermission() {
        let isAuthorized = audioManager.hasPermission

        // Check for downgrade
        if let wasAuthorized = previousAudioAuthorized,
           wasAuthorized && !isAuthorized {

            #if DEBUG
            print("Microphone permission revoked")
            #endif

            // Stop recording immediately
            if audioManager.isRecording {
                audioManager.stopRecording()
            }

            // Create issue
            let issue = PermissionIssue(
                type: .microphone,
                message: "Microphone permission was revoked. Audio recording has been stopped.",
                action: "Open Settings"
            )

            addIssue(issue)
        } else if !isAuthorized && previousAudioAuthorized == nil {
            // First check and not authorized
            let issue = PermissionIssue(
                type: .microphone,
                message: "Microphone permission required. Please enable in Settings.",
                action: "Open Settings"
            )
            addIssue(issue)
        } else if isAuthorized {
            // Permission restored
            removeIssue(type: .microphone)
        }

        previousAudioAuthorized = isAuthorized
    }

    private func checkHealthKitPermission() {
        let isAuthorized = healthKitManager.isAuthorized

        // Check for downgrade
        if let wasAuthorized = previousHealthKitAuthorized,
           wasAuthorized && !isAuthorized {

            #if DEBUG
            print("HealthKit permission revoked")
            #endif

            // Create issue
            let issue = PermissionIssue(
                type: .healthKit,
                message: "HealthKit permission was revoked. Health data collection has been stopped.",
                action: "Open Settings"
            )

            addIssue(issue)
        } else if !isAuthorized && previousHealthKitAuthorized == nil {
            // First check and not authorized
            let issue = PermissionIssue(
                type: .healthKit,
                message: "HealthKit permission required. Please enable in Settings.",
                action: "Open Settings"
            )
            addIssue(issue)
        } else if isAuthorized {
            // Permission restored
            removeIssue(type: .healthKit)
        }

        previousHealthKitAuthorized = isAuthorized
    }

    // MARK: - Issue Management

    private func addIssue(_ issue: PermissionIssue) {
        DispatchQueue.main.async {
            // Remove any existing issue of same type
            self.currentIssues.removeAll { $0.type == issue.type }
            // Add new issue
            self.currentIssues.append(issue)
        }
    }

    private func removeIssue(type: PermissionIssue.PermissionType) {
        DispatchQueue.main.async {
            self.currentIssues.removeAll { $0.type == type }
        }
    }

    // MARK: - Actions

    func openSettings() {
        if let url = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(url)
        }
    }

    // MARK: - Helpers

    private func statusDescription(_ status: CLAuthorizationStatus) -> String {
        switch status {
        case .notDetermined:
            return "Not Determined"
        case .restricted:
            return "Restricted"
        case .denied:
            return "Denied"
        case .authorizedAlways:
            return "Always"
        case .authorizedWhenInUse:
            return "While Using App"
        @unknown default:
            return "Unknown"
        }
    }
}
