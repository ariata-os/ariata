//
//  LowPowerModeMonitor.swift
//  Ariata
//
//  Monitors iOS Low Power Mode and notifies observers of state changes
//

import Foundation
import Combine

/// Monitors Low Power Mode state and publishes changes
class LowPowerModeMonitor: ObservableObject {
    static let shared = LowPowerModeMonitor()

    /// Published property indicating if Low Power Mode is currently enabled
    @Published private(set) var isLowPowerModeEnabled: Bool

    /// Notification posted when Low Power Mode state changes
    static let stateChangedNotification = Notification.Name("LowPowerModeStateChanged")

    private init() {
        // Initialize with current state
        self.isLowPowerModeEnabled = ProcessInfo.processInfo.isLowPowerModeEnabled

        // Observe system notifications for Low Power Mode changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(lowPowerModeChanged),
            name: Notification.Name.NSProcessInfoPowerStateDidChange,
            object: nil
        )

        #if DEBUG
        print("📱 LowPowerModeMonitor initialized: Low Power Mode = \(isLowPowerModeEnabled)")
        #endif
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    @objc private func lowPowerModeChanged() {
        let newState = ProcessInfo.processInfo.isLowPowerModeEnabled

        guard newState != isLowPowerModeEnabled else {
            return // No change, ignore
        }

        #if DEBUG
        print("📱 Low Power Mode changed: \(isLowPowerModeEnabled) → \(newState)")
        #endif

        // Update published property (triggers SwiftUI updates)
        DispatchQueue.main.async { [weak self] in
            self?.isLowPowerModeEnabled = newState
        }

        // Post notification for non-SwiftUI observers
        NotificationCenter.default.post(
            name: LowPowerModeMonitor.stateChangedNotification,
            object: nil,
            userInfo: ["isEnabled": newState]
        )
    }

    /// Manual check of current Low Power Mode state
    func checkCurrentState() -> Bool {
        return ProcessInfo.processInfo.isLowPowerModeEnabled
    }
}
