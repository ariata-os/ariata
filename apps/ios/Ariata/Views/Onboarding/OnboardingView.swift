//
//  OnboardingView.swift
//  Ariata
//
//  Onboarding flow with 3 steps: endpoint config, permissions, initial sync
//

import SwiftUI

struct OnboardingView: View {
    @StateObject private var deviceManager = DeviceManager.shared
    @StateObject private var healthKitManager = HealthKitManager.shared
    
    @State private var currentStep = 1
    @State private var apiEndpoint = "http://localhost:3000"
    @State private var deviceToken = ""
    @State private var isVerifying = false
    @State private var errorMessage: String?
    @State private var syncProgress: Double = 0
    @State private var hasRequestedPermissions = false
    
    @Binding var isOnboardingComplete: Bool
    
    var body: some View {
        NavigationView {
            VStack {
                // Progress indicator
                ProgressIndicator(currentStep: currentStep, totalSteps: 3)
                    .padding(.horizontal)
                    .padding(.top)
                
                // Content based on current step
                Group {
                    switch currentStep {
                    case 1:
                        EndpointConfigurationStep(
                            apiEndpoint: $apiEndpoint,
                            deviceToken: $deviceToken,
                            isVerifying: $isVerifying,
                            errorMessage: $errorMessage,
                            configurationState: $deviceManager.configurationState,
                            configuredStreamCount: $deviceManager.configuredStreamCount,
                            onNext: verifyConfiguration,
                            onRefresh: checkConfigurationStatus
                        )
                    case 2:
                        PermissionsStep(
                            hasRequestedPermissions: $hasRequestedPermissions,
                            onNext: moveToInitialSync
                        )
                    case 3:
                        InitialSyncStep(
                            syncProgress: $syncProgress,
                            onComplete: completeOnboarding
                        )
                    default:
                        EmptyView()
                    }
                }
                .transition(.asymmetric(
                    insertion: .move(edge: .trailing),
                    removal: .move(edge: .leading)
                ))
                
                Spacer()
            }
            .navigationTitle("Setup Ariata")
            .navigationBarTitleDisplayMode(.large)
        }
    }
    
    // MARK: - Actions
    
    private func verifyConfiguration() {
        Task {
            await MainActor.run {
                isVerifying = true
                errorMessage = nil
                
                // Update device configuration with the device token
                deviceManager.updateConfiguration(
                    apiEndpoint: apiEndpoint,
                    deviceToken: deviceToken
                )
            }
            
            // Verify the configuration
            let success = await deviceManager.verifyConfiguration()
            
            await MainActor.run {
                isVerifying = false
                
                if success {
                    // Check if configuration is complete
                    if deviceManager.configurationState == .fullyConfigured {
                        // Streams are configured, move to permissions
                        withAnimation {
                            currentStep = 2
                        }
                    } else if deviceManager.configurationState == .tokenValid {
                        // Token valid but waiting for streams
                        // Stay on current step but show waiting UI
                        errorMessage = nil
                    }
                } else {
                    errorMessage = deviceManager.lastError
                }
            }
        }
    }
    
    private func checkConfigurationStatus() {
        Task {
            await MainActor.run {
                isVerifying = true
            }
            
            // Re-verify to check if streams are now configured
            let success = await deviceManager.verifyConfiguration()
            
            await MainActor.run {
                isVerifying = false
                
                if success && deviceManager.configurationState == .fullyConfigured {
                    // Configuration is complete, move to permissions
                    withAnimation {
                        currentStep = 2
                    }
                } else if !success {
                    errorMessage = deviceManager.lastError
                }
            }
        }
    }
    
    private func moveToInitialSync() {
        withAnimation {
            currentStep = 3
        }
        
        // Start initial sync
        performInitialSync()
    }
    
    private func performInitialSync() {
        Task {
            // Upload batches as they're saved
            let uploadTask = Task {
                // Give initial sync a moment to start saving batches
                try? await Task.sleep(nanoseconds: 2_000_000_000) // 2 seconds
                
                // Keep uploading while initial sync is running
                while healthKitManager.isSyncing {
                    print("🚀 Uploading batches during initial sync")
                    await BatchUploadCoordinator.shared.performUpload()
                    try? await Task.sleep(nanoseconds: 5_000_000_000) // 5 seconds between uploads
                }
            }
            
            let success = await healthKitManager.performInitialSync { progress in
                Task { @MainActor in
                    withAnimation {
                        self.syncProgress = progress
                    }
                }
            }
            
            // Cancel upload task if still running
            uploadTask.cancel()
            
            if success {
                print("✅ Initial sync completed successfully")
                
                // Final upload to ensure everything is sent
                print("🚀 Final upload after initial sync")
                await BatchUploadCoordinator.shared.performUpload()
                
                // Now start regular monitoring (AFTER initial sync is done)
                print("🏥 Starting regular HealthKit monitoring")
                healthKitManager.startMonitoring()
                
                // Start the upload coordinator for background syncing
                BatchUploadCoordinator.shared.startPeriodicUploads()
                
                // Small delay for UI
                try? await Task.sleep(nanoseconds: 500_000_000)
                
                await MainActor.run {
                    completeOnboarding()
                }
            } else {
                print("❌ Initial sync failed")
            }
        }
    }
    
    private func completeOnboarding() {
        withAnimation {
            isOnboardingComplete = true
        }
    }
}

// MARK: - Progress Indicator

struct ProgressIndicator: View {
    let currentStep: Int
    let totalSteps: Int
    
    var body: some View {
        HStack(spacing: 8) {
            ForEach(1...totalSteps, id: \.self) { step in
                Circle()
                    .fill(step <= currentStep ? Color.accentColor : Color.gray.opacity(0.3))
                    .frame(width: 10, height: 10)
                
                if step < totalSteps {
                    Rectangle()
                        .fill(step < currentStep ? Color.accentColor : Color.gray.opacity(0.3))
                        .frame(height: 2)
                }
            }
        }
        .padding(.vertical)
    }
}

// MARK: - Step 1: Endpoint Configuration

struct EndpointConfigurationStep: View {
    @Binding var apiEndpoint: String
    @Binding var deviceToken: String
    @Binding var isVerifying: Bool
    @Binding var errorMessage: String?
    @Binding var configurationState: DeviceConfigurationState
    @Binding var configuredStreamCount: Int
    let onNext: () -> Void
    let onRefresh: () -> Void
    
    @State private var showTokenHelp = false
    @State private var refreshTimer: Timer?
    
    var isValid: Bool {
        !apiEndpoint.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
        deviceToken.count >= 6
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Connect to Server")
                .font(.title2)
                .bold()
                .padding(.horizontal)
            
            VStack(alignment: .leading, spacing: 16) {
                // API Endpoint
                VStack(alignment: .leading, spacing: 8) {
                    Label("API Endpoint", systemImage: "network")
                        .font(.headline)
                    
                    TextField("https://your-server.com", text: $apiEndpoint)
                        .font(.system(size: 17))
                        .padding()
                        .frame(minHeight: 52)
                        .background(Color(.systemGray6))
                        .cornerRadius(10)
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(Color(.systemGray4), lineWidth: 0.5)
                        )
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .keyboardType(.URL)
                    
                    Text("The URL of your Ariata server")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                // Device Token
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Label("Device Token", systemImage: "key.fill")
                            .font(.headline)
                        
                        Button(action: { showTokenHelp.toggle() }) {
                            Image(systemName: "questionmark.circle")
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    TextField("A7K2P9X4", text: $deviceToken)
                        .font(.system(size: 20, weight: .medium, design: .monospaced))
                        .padding()
                        .frame(minHeight: 52)
                        .background(Color(.systemGray6))
                        .cornerRadius(10)
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(Color(.systemGray4), lineWidth: 0.5)
                        )
                        .autocapitalization(.allCharacters)  // Auto-uppercase all input
                        .disableAutocorrection(true)
                        .onChange(of: deviceToken) { oldValue, newValue in
                            // Ensure token is always uppercase
                            deviceToken = newValue.uppercased()
                        }
                    
                    if showTokenHelp {
                        Text("Generate a device token in the web app when adding this iOS device as a data source.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .padding(.vertical, 4)
                            .transition(.opacity)
                    }
                }
            }
            .padding(.horizontal)
            
            // Error message
            if let error = errorMessage {
                HStack {
                    Image(systemName: "exclamationmark.triangle")
                        .foregroundColor(.red)
                    Text(error)
                        .font(.caption)
                        .foregroundColor(.red)
                }
                .padding(.horizontal)
                .padding(.vertical, 8)
                .background(Color.red.opacity(0.1))
                .cornerRadius(8)
                .padding(.horizontal)
            }
            
            // Show different UI based on configuration state
            if configurationState == .tokenValid {
                // Token is valid but waiting for stream configuration
                VStack(spacing: 16) {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("Token Verified")
                            .font(.headline)
                            .foregroundColor(.green)
                    }
                    .padding(.horizontal)
                    
                    VStack(spacing: 8) {
                        ProgressView()
                            .progressViewStyle(CircularProgressViewStyle())
                        
                        Text("Waiting for stream configuration")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        
                        Text("Please complete the configuration in your web browser")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .padding()
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(12)
                    .padding(.horizontal)
                    
                    Button(action: onRefresh) {
                        HStack {
                            Image(systemName: "arrow.clockwise")
                            Text("Check Configuration Status")
                        }
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.accentColor)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                    }
                    .disabled(isVerifying)
                    .padding(.horizontal)
                }
            } else {
                // Normal verify button
                Button(action: onNext) {
                    HStack {
                        if isVerifying {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle())
                                .scaleEffect(0.8)
                        } else {
                            Text("Verify Connection")
                            Image(systemName: "arrow.right")
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isValid ? Color.accentColor : Color.gray.opacity(0.3))
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .disabled(!isValid || isVerifying)
                .padding(.horizontal)
            }
            
            Spacer()
        }
        .padding(.top)
        .onAppear {
            // Start auto-refresh timer if waiting for configuration
            if configurationState == .tokenValid {
                startRefreshTimer()
            }
        }
        .onDisappear {
            stopRefreshTimer()
        }
        .onChange(of: configurationState) { newState in
            if newState == .tokenValid {
                startRefreshTimer()
            } else {
                stopRefreshTimer()
            }
        }
    }
    
    private func startRefreshTimer() {
        stopRefreshTimer()
        // Auto-check every 5 seconds
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            if !isVerifying {
                onRefresh()
            }
        }
    }
    
    private func stopRefreshTimer() {
        refreshTimer?.invalidate()
        refreshTimer = nil
    }
}

// MARK: - Step 2: Permissions

struct PermissionsStep: View {
    @StateObject private var healthKitManager = HealthKitManager.shared
    @StateObject private var locationManager = LocationManager.shared
    @StateObject private var audioManager = AudioManager.shared
    @Binding var hasRequestedPermissions: Bool
    let onNext: () -> Void
    
    @State private var showingSettings = false
    @Environment(\.scenePhase) var scenePhase
    
    var allPermissionsGranted: Bool {
        healthKitManager.hasAllPermissions() && 
        locationManager.hasPermission && 
        audioManager.hasPermission
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Grant Permissions")
                .font(.title2)
                .bold()
                .padding(.horizontal)
            
            Text("Ariata needs the following permissions to track your data:")
                .font(.body)
                .foregroundColor(.secondary)
                .padding(.horizontal)
            
            VStack(spacing: 16) {
                // HealthKit Permission
                PermissionRow(
                    icon: "heart.fill",
                    title: "Health Data",
                    description: "Heart rate, steps, sleep, energy, and more",
                    isGranted: healthKitManager.hasAllPermissions()
                )
                
                // Location Permission
                PermissionRow(
                    icon: "location.fill",
                    title: "Location (Always)",
                    description: "Track your movements and locations",
                    isGranted: locationManager.hasPermission
                )
                
                // Microphone Permission
                PermissionRow(
                    icon: "mic.fill",
                    title: "Microphone",
                    description: "Record and transcribe audio",
                    isGranted: audioManager.hasPermission
                )
            }
            .padding(.horizontal)
            
            // Request permissions button
            if !hasRequestedPermissions {
                Button(action: requestAllPermissions) {
                    HStack {
                        Text("Request Permissions")
                        Image(systemName: "shield.checkmark")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .padding(.horizontal)
            } else if !allPermissionsGranted {
                // Open settings button
                VStack(spacing: 12) {
                    Text("Some permissions were denied")
                        .font(.caption)
                        .foregroundColor(.red)
                    
                    Button(action: openSettings) {
                        HStack {
                            Text("Open Settings")
                            Image(systemName: "arrow.up.forward.square")
                        }
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.orange)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                    }
                }
                .padding(.horizontal)
            } else {
                // Continue button
                Button(action: onNext) {
                    HStack {
                        Text("Continue")
                        Image(systemName: "arrow.right")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.green)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .padding(.horizontal)
            }
            
            Spacer()
        }
        .padding(.top)
        .onChange(of: scenePhase) { oldValue, newValue in
            if newValue == .active && hasRequestedPermissions {
                // Re-check permissions when returning from Settings
                healthKitManager.checkAuthorizationStatus()
                locationManager.checkAuthorizationStatus()
                audioManager.checkAuthorizationStatus()
            }
        }
    }
    
    private func requestAllPermissions() {
        Task {
            // Request all permissions in sequence
            
            // 1. Request HealthKit
            _ = await healthKitManager.requestAuthorization()
            
            // 2. Request Location (Always)
            _ = await locationManager.requestAuthorization()
            
            // 3. Request Microphone
            _ = await audioManager.requestAuthorization()
            
            await MainActor.run {
                hasRequestedPermissions = true
            }
        }
    }
    
    private func openSettings() {
        if let url = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(url)
        }
    }
}

struct PermissionRow: View {
    let icon: String
    let title: String
    let description: String
    let isGranted: Bool
    
    var body: some View {
        HStack(spacing: 16) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(isGranted ? .green : .orange)
                .frame(width: 30)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.headline)
                Text(description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            Image(systemName: isGranted ? "checkmark.circle.fill" : "xmark.circle")
                .foregroundColor(isGranted ? .green : .red)
                .font(.title2)
        }
        .padding()
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Step 3: Initial Sync

struct InitialSyncStep: View {
    @Binding var syncProgress: Double
    let onComplete: () -> Void
    @StateObject private var deviceManager = DeviceManager.shared
    
    var progressPercentage: Int {
        Int(syncProgress * 100)
    }
    
    var isComplete: Bool {
        syncProgress >= 1.0
    }
    
    var syncDays: Int {
        deviceManager.configuration.getInitialSyncDays(for: "healthkit")
    }
    
    var body: some View {
        VStack(spacing: 32) {
            Text("Syncing Health Data")
                .font(.title2)
                .bold()
            
            Text(isComplete ? "Sync complete! Ready to start tracking." : "Fetching the last \(syncDays) days of health data...")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
            
            // Progress circle
            ZStack {
                Circle()
                    .stroke(Color.gray.opacity(0.3), lineWidth: 20)
                    .frame(width: 200, height: 200)
                
                Circle()
                    .trim(from: 0, to: syncProgress)
                    .stroke(isComplete ? Color.green : Color.accentColor, style: StrokeStyle(lineWidth: 20, lineCap: .round))
                    .frame(width: 200, height: 200)
                    .rotationEffect(.degrees(-90))
                    .animation(.easeInOut(duration: 0.5), value: syncProgress)
                
                VStack {
                    if isComplete {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 60))
                            .foregroundColor(.green)
                    } else {
                        Text("\(progressPercentage)%")
                            .font(.system(size: 48, weight: .bold, design: .rounded))
                        Text("Complete")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            if !isComplete {
                VStack(spacing: 8) {
                    Image(systemName: "info.circle")
                        .foregroundColor(.blue)
                    Text("Keep the app open during initial sync")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("This ensures all data is uploaded successfully")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding()
                .background(Color.blue.opacity(0.1))
                .cornerRadius(12)
                .padding(.horizontal)
            }
            
            if isComplete {
                Button(action: onComplete) {
                    HStack {
                        Text("Get Started")
                        Image(systemName: "arrow.right")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.green)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .padding(.horizontal)
            }
            
            Spacer()
        }
        .padding(.top, 48)
        .onChange(of: syncProgress) { oldValue, newValue in
            // Auto-complete when reaching 100%
            if newValue >= 1.0 && oldValue < 1.0 {
                // Give a small delay to show the completion state
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                    onComplete()
                }
            }
        }
    }
}

// MARK: - Preview

struct OnboardingView_Previews: PreviewProvider {
    static var previews: some View {
        OnboardingView(isOnboardingComplete: .constant(false))
    }
}