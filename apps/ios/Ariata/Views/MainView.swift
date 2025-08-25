//
//  MainView.swift
//  Ariata
//
//  Main dashboard view after onboarding
//

import SwiftUI

struct MainView: View {
    @StateObject private var deviceManager = DeviceManager.shared
    @StateObject private var healthKitManager = HealthKitManager.shared
    @StateObject private var uploadCoordinator = BatchUploadCoordinator.shared
    
    @State private var showingSettings = false
    @State private var isManualSyncing = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 20) {
                    // Status Card
                    StatusCard()
                        .padding(.horizontal)
                    
                    // Quick Stats
                    QuickStatsView()
                        .padding(.horizontal)
                    
                    // Manual Sync Button
                    ManualSyncButton(
                        isManualSyncing: $isManualSyncing,
                        onSync: performManualSync
                    )
                    .padding(.horizontal)
                    
                    // Data Collection Status
                    DataCollectionStatus()
                        .padding(.horizontal)
                    
                    // Debug Info (if enabled)
                    #if DEBUG
                    DebugInfoSection()
                        .padding(.horizontal)
                    #endif
                }
                .padding(.vertical)
            }
            .navigationTitle("Ariata")
            .navigationBarTitleDisplayMode(.large)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingSettings = true }) {
                        Image(systemName: "gearshape")
                    }
                }
            }
            .sheet(isPresented: $showingSettings) {
                SettingsView()
            }
        }
    }
    
    private func performManualSync() {
        Task {
            await MainActor.run {
                isManualSyncing = true
            }
            
            print("🔄 Manual sync triggered")
            
            // Trigger full sync and upload (collects from all streams)
            await uploadCoordinator.triggerManualUpload()
            
            await MainActor.run {
                isManualSyncing = false
            }
        }
    }
}

// MARK: - Status Card

struct StatusCard: View {
    @ObservedObject private var deviceManager = DeviceManager.shared
    @ObservedObject private var uploadCoordinator = BatchUploadCoordinator.shared
    @State private var countdownTimer: Timer?
    @State private var secondsUntilSync: Int = 0
    
    var statusColor: Color {
        if !deviceManager.isConfigured {
            return .red
        } else if uploadCoordinator.isUploading {
            return .blue
        } else {
            return .green
        }
    }
    
    var statusText: String {
        if !deviceManager.isConfigured {
            return "Not Connected"
        } else if uploadCoordinator.isUploading {
            return "Syncing..."
        } else {
            return "Connected"
        }
    }
    
    var countdownText: String {
        let minutes = secondsUntilSync / 60
        let seconds = secondsUntilSync % 60
        return String(format: "Next sync: %d:%02d", minutes, seconds)
    }
    
    var body: some View {
        VStack(spacing: 16) {
            HStack {
                Circle()
                    .fill(statusColor)
                    .frame(width: 12, height: 12)
                
                Text(statusText)
                    .font(.headline)
                
                Spacer()
                
                if deviceManager.isConfigured && !uploadCoordinator.isUploading {
                    Text(countdownText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            if deviceManager.isConfigured {
                HStack {
                    Label(deviceManager.configuration.deviceName, systemImage: "iphone")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Text(deviceManager.configuration.apiEndpoint)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
        }
        .padding()
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
        .onAppear {
            startCountdownTimer()
        }
        .onDisappear {
            countdownTimer?.invalidate()
        }
        .onChange(of: uploadCoordinator.lastUploadDate) {
            updateCountdown()
        }
    }
    
    private func startCountdownTimer() {
        updateCountdown()
        countdownTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            updateCountdown()
        }
    }
    
    private func updateCountdown() {
        let syncInterval: TimeInterval = 300 // 5 minutes
        
        if let lastUpload = uploadCoordinator.lastUploadDate {
            let nextSyncTime = lastUpload.addingTimeInterval(syncInterval)
            let remainingTime = nextSyncTime.timeIntervalSince(Date())
            
            if remainingTime > 0 {
                secondsUntilSync = Int(remainingTime)
            } else {
                secondsUntilSync = 0
            }
        } else {
            // If no last upload, assume sync will happen soon
            secondsUntilSync = Int(syncInterval)
        }
    }
}

// MARK: - Quick Stats

struct QuickStatsView: View {
    @ObservedObject private var uploadCoordinator = BatchUploadCoordinator.shared
    
    var body: some View {
        HStack(spacing: 16) {
            StatCard(
                title: "HealthKit",
                value: "\(uploadCoordinator.streamCounts.healthkit)",
                icon: "heart.fill",
                color: .red
            )
            
            StatCard(
                title: "Location",
                value: "\(uploadCoordinator.streamCounts.location)",
                icon: "location.fill",
                color: .blue
            )
            
            StatCard(
                title: "Audio",
                value: "\(uploadCoordinator.streamCounts.audio)",
                icon: "mic.fill",
                color: .green
            )
        }
    }
}

struct StatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color
    
    var body: some View {
        VStack(spacing: 4) {
            HStack(spacing: 4) {
                Image(systemName: icon)
                    .font(.caption)
                    .foregroundColor(color)
                
                Text(value)
                    .font(.headline)
                    .bold()
            }
            
            Text(title)
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(color.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Manual Sync Button

struct ManualSyncButton: View {
    @Binding var isManualSyncing: Bool
    let onSync: () -> Void
    
    var body: some View {
        Button(action: onSync) {
            HStack {
                if isManualSyncing {
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle())
                        .scaleEffect(0.8)
                } else {
                    Image(systemName: "arrow.triangle.2.circlepath")
                }
                
                Text("Sync Now")
                    .font(.headline)
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(Color.accentColor)
            .foregroundColor(.white)
            .cornerRadius(12)
        }
        .disabled(isManualSyncing)
    }
}

// MARK: - Data Collection Status

struct DataCollectionStatus: View {
    @ObservedObject private var healthKitManager = HealthKitManager.shared
    @ObservedObject private var locationManager = LocationManager.shared
    @ObservedObject private var audioManager = AudioManager.shared
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Data Collection")
                .font(.headline)
            
            VStack(spacing: 8) {
                DataStreamRow(
                    name: "HealthKit",
                    isActive: healthKitManager.isAuthorized,
                    lastSync: healthKitManager.lastSyncDate
                )
                
                DataStreamRow(
                    name: "Location",
                    isActive: locationManager.isTracking,
                    lastSync: locationManager.lastSaveDate
                )
                
                DataStreamRow(
                    name: "Audio",
                    isActive: audioManager.isRecording,
                    lastSync: audioManager.lastSaveDate
                )
            }
        }
        .padding()
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
    }
}

struct DataStreamRow: View {
    let name: String
    let isActive: Bool
    let lastSync: Date?
    
    var body: some View {
        HStack {
            Label(name, systemImage: isActive ? "checkmark.circle.fill" : "xmark.circle")
                .foregroundColor(isActive ? .green : .red)
            
            Spacer()
            
            if let lastSync = lastSync {
                Text(lastSync, style: .relative)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}

// MARK: - Debug Info

struct DebugInfoSection: View {
    @ObservedObject private var deviceManager = DeviceManager.shared
    @State private var isExpanded = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Button(action: { isExpanded.toggle() }) {
                HStack {
                    Label("Debug Info", systemImage: "hammer")
                        .font(.headline)
                    
                    Spacer()
                    
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .foregroundColor(.secondary)
                }
            }
            
            if isExpanded {
                Text(deviceManager.getDebugInfo())
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.secondary)
                    .padding()
                    .background(Color.black.opacity(0.05))
                    .cornerRadius(8)
            }
        }
        .padding()
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Preview

struct MainView_Previews: PreviewProvider {
    static var previews: some View {
        MainView()
    }
}