//
//  SettingsView.swift
//  Ariata
//
//  Settings and configuration view
//

import SwiftUI
import AVFoundation

struct SettingsView: View {
    @ObservedObject private var deviceManager = DeviceManager.shared
    @ObservedObject private var healthKitManager = HealthKitManager.shared
    @ObservedObject private var uploadCoordinator = BatchUploadCoordinator.shared
    @ObservedObject private var locationManager = LocationManager.shared
    @ObservedObject private var audioManager = AudioManager.shared
    
    @Environment(\.dismiss) var dismiss
    @State private var showingResetAlert = false
    @State private var showingStorageDetails = false
    @State private var showingEndpointEdit = false
    
    var body: some View {
        NavigationView {
            Form {
                // Device Section
                Section(header: Text("Device")) {
                    HStack {
                        Text("Device ID")
                        Spacer()
                        Text(String(deviceManager.configuration.deviceId.suffix(8)))
                            .font(.system(.body, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Status")
                        Spacer()
                        Text(deviceManager.statusMessage)
                            .foregroundColor(.secondary)
                    }
                    
                    if deviceManager.isConfigured {
                        HStack {
                            Text("API Endpoint")
                            Spacer()
                            Text(deviceManager.configuration.apiEndpoint)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        
                        Button(action: { showingEndpointEdit = true }) {
                            Label("Edit Endpoint", systemImage: "pencil")
                                .foregroundColor(.accentColor)
                        }
                    }
                }
                
                // Permissions Section
                Section(header: Text("Permissions")) {
                    PermissionStatusRow(
                        title: "HealthKit",
                        isGranted: healthKitManager.isAuthorized
                    )
                    
                    PermissionStatusRow(
                        title: "Location (Always)",
                        isGranted: locationManager.hasPermission
                    )
                    
                    PermissionStatusRow(
                        title: "Microphone",
                        isGranted: audioManager.hasPermission
                    )
                    
                    Button(action: openAppSettings) {
                        Label("Open iOS Settings", systemImage: "gear")
                    }
                }
                
                // Storage Section
                Section(header: Text("Storage")) {
                    HStack {
                        Text("Queue Size")
                        Spacer()
                        Text(uploadCoordinator.getQueueSizeString())
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Pending Uploads")
                        Spacer()
                        Text("\(uploadCoordinator.uploadStats.pending)")
                            .foregroundColor(.secondary)
                    }
                    
                    Button(action: { showingStorageDetails = true }) {
                        Label("Storage Details", systemImage: "info.circle")
                    }
                }
                
                // Data Collection Settings
                Section(header: Text("Data Collection")) {
                    Toggle("Location Tracking", isOn: Binding(
                        get: { locationManager.isTracking },
                        set: { enabled in
                            if enabled && locationManager.hasPermission {
                                locationManager.startTracking()
                            } else {
                                locationManager.stopTracking()
                            }
                        }
                    ))
                    .disabled(!locationManager.hasPermission)
                    
                    Toggle("Audio Recording", isOn: Binding(
                        get: { audioManager.isRecording },
                        set: { enabled in
                            if enabled && audioManager.hasPermission {
                                audioManager.startRecording()
                            } else {
                                audioManager.stopRecording()
                            }
                        }
                    ))
                    .disabled(!audioManager.hasPermission)
                }
                
                // Audio Input Settings
                if audioManager.hasPermission && !audioManager.availableAudioInputs.isEmpty {
                    Section(header: Text("Audio Input")) {
                        Picker("Microphone", selection: Binding(
                            get: {
                                audioManager.selectedAudioInput ?? audioManager.availableAudioInputs.first
                            },
                            set: { newInput in
                                audioManager.selectAudioInput(newInput)
                            }
                        )) {
                            ForEach(audioManager.availableAudioInputs, id: \.uid) { input in
                                Text(audioManager.getDisplayName(for: input))
                                    .tag(input as AVAudioSessionPortDescription?)
                            }
                        }
                        .pickerStyle(MenuPickerStyle())
                        
                        if let selectedInput = audioManager.selectedAudioInput {
                            HStack {
                                Text("Current Input")
                                Spacer()
                                Text(audioManager.getDisplayName(for: selectedInput))
                                    .foregroundColor(.secondary)
                                    .font(.caption)
                            }
                        }
                        
                        Text("Select 'iPhone Microphone' to prevent Bluetooth devices from being used")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                // Sync Settings
                Section(header: Text("Sync Settings")) {
                    HStack {
                        Text("Auto Sync")
                        Spacer()
                        Text("Every 5 minutes")
                            .foregroundColor(.secondary)
                    }
                    
                    if let lastUpload = uploadCoordinator.lastUploadDate {
                        HStack {
                            Text("Last Upload")
                            Spacer()
                            Text(lastUpload, style: .relative)
                                .foregroundColor(.secondary)
                        }
                    }
                }
                
                // About Section
                Section(header: Text("About")) {
                    HStack {
                        Text("Version")
                        Spacer()
                        Text(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Build")
                        Spacer()
                        Text(Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "1")
                            .foregroundColor(.secondary)
                    }
                }
                
                // Actions Section
                Section {
                    Button(action: { showingResetAlert = true }) {
                        Label("Reset App", systemImage: "exclamationmark.triangle")
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .alert("Reset App?", isPresented: $showingResetAlert) {
                Button("Cancel", role: .cancel) { }
                Button("Reset", role: .destructive) {
                    resetApp()
                }
            } message: {
                Text("This will clear all settings and require you to set up the app again. Pending uploads will be lost.")
            }
            .sheet(isPresented: $showingStorageDetails) {
                StorageDetailsView()
            }
            .sheet(isPresented: $showingEndpointEdit) {
                EndpointEditView()
            }
        }
    }
    
    private func openAppSettings() {
        if let url = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(url)
        }
    }
    
    private func resetApp() {
        // Stop all services (this stops all data collection)
        uploadCoordinator.stopPeriodicUploads()
        
        // Stop individual trackers
        locationManager.stopTracking()
        audioManager.stopRecording()
        
        // Clear configuration
        deviceManager.clearConfiguration()
        
        // Clear UserDefaults
        if let bundleId = Bundle.main.bundleIdentifier {
            UserDefaults.standard.removePersistentDomain(forName: bundleId)
        }
        
        // Exit app (user will need to restart)
        exit(0)
    }
}

// MARK: - Permission Status Row

struct PermissionStatusRow: View {
    let title: String
    let isGranted: Bool
    
    var body: some View {
        HStack {
            Text(title)
            Spacer()
            Image(systemName: isGranted ? "checkmark.circle.fill" : "xmark.circle")
                .foregroundColor(isGranted ? .green : .red)
        }
    }
}

// MARK: - Storage Details View

struct StorageDetailsView: View {
    @ObservedObject private var uploadCoordinator = BatchUploadCoordinator.shared
    @Environment(\.dismiss) var dismiss
    
    private let sqliteManager = SQLiteManager.shared
    
    @State private var databaseSize: String = "Calculating..."
    @State private var availableStorage: String = "Calculating..."
    
    var body: some View {
        NavigationView {
            List {
                Section(header: Text("Upload Queue")) {
                    DetailRow(
                        label: "Pending",
                        value: "\(uploadCoordinator.uploadStats.pending) events"
                    )
                    
                    DetailRow(
                        label: "Failed",
                        value: "\(uploadCoordinator.uploadStats.failed) events"
                    )
                    
                    DetailRow(
                        label: "Total Size",
                        value: uploadCoordinator.getQueueSizeString()
                    )
                }
                
                Section(header: Text("Storage")) {
                    DetailRow(
                        label: "Database Size",
                        value: databaseSize
                    )
                    
                    DetailRow(
                        label: "Available Storage",
                        value: availableStorage
                    )
                }
                
                Section(header: Text("Cleanup Policy")) {
                    Text("• Uploaded data is retained for 3 days")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text("• Failed uploads are retried up to 5 times")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text("• Storage warnings appear below 100MB")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Storage Details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .onAppear {
                calculateStorageInfo()
            }
        }
    }
    
    private func calculateStorageInfo() {
        // Database size
        let dbSize = sqliteManager.getTotalDatabaseSize()
        databaseSize = formatBytes(dbSize)
        
        // Available storage
        if let systemAttributes = try? FileManager.default.attributesOfFileSystem(
            forPath: NSHomeDirectory()
        ) {
            if let freeSpace = systemAttributes[.systemFreeSize] as? Int64 {
                availableStorage = formatBytes(freeSpace)
            }
        }
    }
    
    private func formatBytes(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: bytes)
    }
}

struct DetailRow: View {
    let label: String
    let value: String
    
    var body: some View {
        HStack {
            Text(label)
            Spacer()
            Text(value)
                .foregroundColor(.secondary)
        }
    }
}

// MARK: - Preview

struct SettingsView_Previews: PreviewProvider {
    static var previews: some View {
        SettingsView()
    }
}