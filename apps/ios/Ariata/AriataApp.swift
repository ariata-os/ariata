//
//  AriataApp.swift
//  Ariata
//
//  Created by Adam Jace on 7/30/25.
//

import SwiftUI
import BackgroundTasks

@main
struct AriataApp: App {
    @AppStorage("isOnboardingComplete") private var isOnboardingComplete = false
    @StateObject private var lifecycleObserver = AppLifecycleObserver()

    init() {
        // Register background tasks on app launch
        registerBackgroundTasks()

        // Initialize all managers to ensure singletons are created
        _ = DeviceManager.shared
        _ = NetworkMonitor.shared  // Initialize network monitoring early
        _ = BatchUploadCoordinator.shared
        _ = HealthKitManager.shared
        _ = LocationManager.shared
        _ = AudioManager.shared
        _ = PermissionMonitor.shared
        _ = LowPowerModeMonitor.shared
    }

    var body: some Scene {
        WindowGroup {
            Group {
                if isOnboardingComplete {
                    MainView()
                        .onAppear {
                            // Start all background services
                            startAllServices()
                        }
                } else {
                    OnboardingView(isOnboardingComplete: $isOnboardingComplete)
                }
            }
        }
    }
    
    private func registerBackgroundTasks() {
        // Register background refresh task
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.ariata.ios.refresh",
            using: nil
        ) { task in
            handleBackgroundRefresh(task: task as! BGAppRefreshTask)
        }
        
        // Register background processing task
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.ariata.ios.processing",
            using: nil
        ) { task in
            handleBackgroundProcessing(task: task as! BGProcessingTask)
        }
    }
    
    private func handleBackgroundRefresh(task: BGAppRefreshTask) {
        // Schedule next refresh
        scheduleBackgroundRefresh()

        // Perform quick sync
        let syncTask = Task {
            await BatchUploadCoordinator.shared.performUpload()
        }

        task.expirationHandler = {
            syncTask.cancel()
        }

        Task {
            _ = await syncTask.result
            task.setTaskCompleted(success: true)
        }
    }

    private func handleBackgroundProcessing(task: BGProcessingTask) {
        // Perform longer running tasks
        let processingTask = Task {
            await BatchUploadCoordinator.shared.performUpload()
        }

        task.expirationHandler = {
            processingTask.cancel()
        }

        Task {
            _ = await processingTask.result
            task.setTaskCompleted(success: true)
        }
    }
    
    private func scheduleBackgroundRefresh() {
        let request = BGAppRefreshTaskRequest(identifier: "com.ariata.ios.refresh")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 minutes
        
        do {
            try BGTaskScheduler.shared.submit(request)
        } catch {
            #if DEBUG
            print("Failed to schedule background refresh: \(error)")
            #endif
        }
    }
    
    private func startAllServices() {
        let deviceManager = DeviceManager.shared
        let uploadCoordinator = BatchUploadCoordinator.shared
        let locationManager = LocationManager.shared
        let audioManager = AudioManager.shared
        let healthKitManager = HealthKitManager.shared

        // Schedule background refresh task (required for background execution)
        scheduleBackgroundRefresh()

        // Start periodic uploads (this now handles all data collection)
        uploadCoordinator.startPeriodicUploads()

        let config = deviceManager.configuration

        #if DEBUG
        print("🚀 Starting services with configuration:")
        print("   Stream configurations: \(config.streamConfigurations.count) streams")
        for (key, streamConfig) in config.streamConfigurations {
            print("     - \(key): enabled=\(streamConfig.enabled), initialSyncDays=\(streamConfig.initialSyncDays)")
        }
        #endif

        // Start location tracking if authorized AND enabled
        if locationManager.hasPermission && config.isStreamEnabled("location") {
            locationManager.startTracking()
        }

        // Start audio recording if authorized AND enabled
        if audioManager.hasPermission && config.isStreamEnabled("microphone") {
            audioManager.startRecording()
        }

        // Start HealthKit monitoring if authorized AND enabled
        if healthKitManager.isAuthorized && config.isStreamEnabled("healthkit") {
            // Check if we have anchors (meaning initial sync was done)
            _ = !healthKitManager.anchors.isEmpty

            healthKitManager.startMonitoring()
        }
    }
}
