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
    @StateObject private var deviceManager = DeviceManager.shared
    @StateObject private var uploadCoordinator = BatchUploadCoordinator.shared
    @StateObject private var healthKitManager = HealthKitManager.shared
    @StateObject private var locationManager = LocationManager.shared
    @StateObject private var audioManager = AudioManager.shared
    @AppStorage("isOnboardingComplete") private var isOnboardingComplete = false
    
    init() {
        // Register background tasks on app launch
        registerBackgroundTasks()
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
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.willEnterForegroundNotification)) { _ in
                // Update stats when app comes to foreground
                uploadCoordinator.updateUploadStats()
                
                // Check and recover audio recording state if needed
                if isOnboardingComplete && audioManager.hasPermission && !audioManager.isRecording {
                    print("⚠️ Audio recording was stopped, restarting...")
                    audioManager.startRecording()
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
            await uploadCoordinator.performUpload()
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
            await uploadCoordinator.performUpload()
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
            print("Failed to schedule background refresh: \(error)")
        }
    }
    
    private func startAllServices() {
        // Start periodic uploads (this now handles all data collection)
        uploadCoordinator.startPeriodicUploads()
        
        let config = deviceManager.configuration
        
        print("🚀 Starting services with configuration:")
        print("   Stream configurations: \(config.streamConfigurations.count) streams")
        for (key, streamConfig) in config.streamConfigurations {
            print("     - \(key): enabled=\(streamConfig.enabled), initialSyncDays=\(streamConfig.initialSyncDays)")
        }
        
        // Start location tracking if authorized AND enabled
        if locationManager.hasPermission && config.isStreamEnabled("location") {
            locationManager.startTracking()
            print("✅ Started location tracking (enabled in web app)")
        } else if locationManager.hasPermission {
            print("⏸️ Location tracking disabled in web app")
        } else {
            print("❌ Location tracking - no permission")
        }
        
        // Start audio recording if authorized AND enabled
        if audioManager.hasPermission && config.isStreamEnabled("mic") {
            audioManager.startRecording()
            print("✅ Started audio recording (enabled in web app)")
        } else if audioManager.hasPermission {
            print("⏸️ Audio recording disabled in web app")
        } else {
            print("❌ Audio recording - no permission")
        }
        
        // Start HealthKit monitoring if authorized AND enabled
        if healthKitManager.isAuthorized && config.isStreamEnabled("healthkit") {
            // Check if we have anchors (meaning initial sync was done)
            let hasAnchors = !healthKitManager.anchors.isEmpty
            
            if hasAnchors {
                // We have anchors, so initial sync was done - start regular monitoring
                healthKitManager.startMonitoring()
                print("✅ Started HealthKit monitoring (incremental sync)")
            } else {
                // No anchors means initial sync hasn't been done yet
                // This happens when the app is restarted after onboarding
                // In this case, onboarding should have set the anchors
                // If not, we need to do initial sync first
                print("⚠️ HealthKit initial sync may not have completed properly")
                print("⚠️ Starting monitoring anyway - will collect all data")
                healthKitManager.startMonitoring()
            }
        } else if healthKitManager.isAuthorized {
            print("⏸️ HealthKit monitoring disabled in web app")
        } else {
            print("❌ HealthKit monitoring - no permission")
        }
        
        print("✅ Service startup complete")
    }
}
