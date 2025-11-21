//
//  AppLifecycleObserver.swift
//  Ariata
//
//  Observes app lifecycle events with proper cleanup
//

import Foundation
import UIKit

/// Observes app lifecycle events with guaranteed cleanup via deinit
class AppLifecycleObserver: ObservableObject {
    private var observers: [NSObjectProtocol] = []

    init() {
        setupObservers()
    }

    private func setupObservers() {
        // Observe foreground transitions
        let foregroundObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.willEnterForegroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleWillEnterForeground()
        }
        observers.append(foregroundObserver)
    }

    private func handleWillEnterForeground() {
        // Update stats when app comes to foreground
        BatchUploadCoordinator.shared.updateUploadStats()

        // Trigger health check for all services when returning to foreground
        // Delay slightly to ensure system is ready after returning to foreground
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            // Audio health check will handle any needed recovery
            _ = AudioManager.shared.performHealthCheck()
            // Location health check ensures tracking continues (especially important after network changes)
            _ = LocationManager.shared.performHealthCheck()
        }
    }

    deinit {
        // Guaranteed cleanup of all observers
        for observer in observers {
            NotificationCenter.default.removeObserver(observer)
        }
        observers.removeAll()
    }
}
