//
//  DeviceManager.swift
//  Ariata
//
//  Manages device configuration and authentication state
//

import Foundation
import UIKit
import Combine

enum DeviceConfigurationState {
    case notConfigured
    case tokenValid           // Token is valid but streams not configured
    case fullyConfigured      // Token valid AND streams configured
}

class DeviceManager: ObservableObject {
    static let shared = DeviceManager()
    
    @Published var configuration: DeviceConfiguration
    @Published var isConfigured: Bool = false
    @Published var configurationState: DeviceConfigurationState = .notConfigured
    @Published var isVerifying: Bool = false
    @Published var lastError: String?
    @Published var configuredStreamCount: Int = 0
    
    private let userDefaults = UserDefaults.standard
    private let configKey = "com.ariata.deviceConfiguration"
    
    private var cancellables = Set<AnyCancellable>()
    
    private init() {
        // Load saved configuration or create new one
        if let savedData = userDefaults.data(forKey: configKey),
           let savedConfig = try? JSONDecoder().decode(DeviceConfiguration.self, from: savedData) {
            self.configuration = savedConfig
            self.isConfigured = savedConfig.isConfigured
            print("📱 Loaded saved configuration:")
            print("   Device ID: \(savedConfig.deviceId)")
            print("   Is configured: \(savedConfig.isConfigured)")
            print("   Stream configurations: \(savedConfig.streamConfigurations.count) streams")
            for (key, config) in savedConfig.streamConfigurations {
                print("     - \(key): enabled=\(config.enabled), initialSyncDays=\(config.initialSyncDays)")
            }
        } else {
            self.configuration = DeviceConfiguration()
            self.isConfigured = false
            print("📱 No saved configuration found, created new one")
        }
        
        // Observe configuration changes to save automatically
        $configuration
            .debounce(for: .milliseconds(500), scheduler: RunLoop.main)
            .sink { [weak self] config in
                self?.saveConfiguration(config)
                self?.isConfigured = config.isConfigured
            }
            .store(in: &cancellables)
    }
    
    // MARK: - Configuration Management
    
    func updateConfiguration(apiEndpoint: String, deviceToken: String) {
        configuration.apiEndpoint = apiEndpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        configuration.deviceToken = deviceToken.trimmingCharacters(in: .whitespacesAndNewlines)
        configuration.configuredDate = Date()
        
        // Save configuration to UserDefaults
        saveConfiguration(configuration)
        
        print("📝 Updated configuration:")
        print("   Endpoint: \(configuration.apiEndpoint)")
        print("   Token: \(configuration.deviceToken.prefix(15))...")
    }
    
    func updateEndpoint(_ newEndpoint: String) async -> Bool {
        let trimmedEndpoint = newEndpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        
        // Validate the endpoint format
        guard validateEndpoint(trimmedEndpoint) else {
            await MainActor.run {
                self.lastError = "Invalid endpoint URL format"
            }
            return false
        }
        
        // Test the connection to the new endpoint
        let isReachable = await NetworkManager.shared.testConnection(endpoint: trimmedEndpoint)
        if !isReachable {
            await MainActor.run {
                self.lastError = "Cannot reach the new endpoint"
            }
            return false
        }
        
        // Update the configuration
        await MainActor.run {
            self.configuration.apiEndpoint = trimmedEndpoint
            self.lastError = nil
            
            // Force save the configuration
            self.saveConfiguration(self.configuration)
        }
        
        print("✅ Endpoint updated successfully to: \(trimmedEndpoint)")
        return true
    }
    
    private func saveConfiguration(_ config: DeviceConfiguration) {
        if let encoded = try? JSONEncoder().encode(config) {
            userDefaults.set(encoded, forKey: configKey)
            print("💾 Saved configuration with \(config.streamConfigurations.count) stream configs")
            for (key, streamConfig) in config.streamConfigurations {
                print("     - \(key): enabled=\(streamConfig.enabled)")
            }
        } else {
            print("❌ Failed to encode configuration for saving")
        }
    }
    
    func clearConfiguration() {
        configuration = DeviceConfiguration()
        userDefaults.removeObject(forKey: configKey)
        isConfigured = false
        lastError = nil
    }
    
    // MARK: - Connection Verification
    
    func verifyConfiguration() async -> Bool {
        await MainActor.run {
            isVerifying = true
            lastError = nil
        }
        
        defer { 
            Task { @MainActor in
                self.isVerifying = false
            }
        }
        
        // Validate configuration
        guard !configuration.apiEndpoint.isEmpty else {
            await MainActor.run {
                self.lastError = "Please enter an API endpoint URL"
            }
            return false
        }
        
        guard !configuration.deviceToken.isEmpty else {
            await MainActor.run {
                self.lastError = "Please enter a device token"
            }
            return false
        }
        
        guard let baseURL = URL(string: configuration.apiEndpoint) else {
            await MainActor.run {
                self.lastError = "Invalid API endpoint URL format"
            }
            return false
        }
        
        // Validate token format (should be at least 6 characters)
        guard configuration.deviceToken.count >= 6 else {
            await MainActor.run {
                self.lastError = "Invalid device token format. Token should be at least 6 characters"
            }
            return false
        }
        
        // Verify token with server
        let verifyURL = baseURL.appendingPathComponent("/api/device/verify")
        
        print("🔍 Attempting verification:")
        print("   Base URL: \(baseURL)")
        print("   Verify URL: \(verifyURL)")
        print("   Token (full): \(configuration.deviceToken)")
        print("   Token (preview): \(configuration.deviceToken.prefix(4))...")
        print("   Token length: \(configuration.deviceToken.count) characters")
        
        let verificationResponse = await NetworkManager.shared.verifyDeviceToken(
            endpoint: verifyURL,
            deviceToken: configuration.deviceToken,
            deviceInfo: [
                "deviceName": configuration.deviceName,
                "deviceId": configuration.deviceId,
                "platform": "iOS",
                "osVersion": UIDevice.current.systemVersion,
                "model": UIDevice.current.model
            ]
        )
        
        if !verificationResponse.success {
            let errorMsg = NetworkManager.shared.lastError?.errorDescription ?? "Failed to verify device token. Please check the token and try again."
            print("❌ Verification failed: \(errorMsg)")
            
            await MainActor.run {
                self.lastError = errorMsg
                self.configurationState = .notConfigured
            }
            return false
        }
        
        // Update configuration state based on whether streams are configured
        await MainActor.run {
            self.configuration.configuredDate = Date()
            self.configuredStreamCount = verificationResponse.configuredStreamCount
            
            // Store stream configurations from web app
            self.configuration.streamConfigurations = verificationResponse.streamConfigurations.reduce(into: [:]) { result, pair in
                result[pair.key] = StreamConfig(
                    enabled: pair.value.enabled,
                    initialSyncDays: pair.value.initialSyncDays,
                    displayName: pair.value.displayName
                )
            }
            
            if verificationResponse.configurationComplete {
                // Fully configured - token valid AND streams configured
                self.isConfigured = true
                self.configurationState = .fullyConfigured
                self.lastError = nil
                print("✅ Device fully configured with \(verificationResponse.configuredStreamCount) streams")
                print("   Stream configurations:")
                for (key, config) in self.configuration.streamConfigurations {
                    print("     - \(key): enabled=\(config.enabled), initialSyncDays=\(config.initialSyncDays)")
                }
            } else {
                // Token valid but waiting for stream configuration
                self.isConfigured = false  // Don't mark as fully configured yet
                self.configurationState = .tokenValid
                self.lastError = nil  // Clear any errors since token is valid
                print("⏳ Token valid, waiting for stream configuration on web app")
            }
            
            // Always save the configuration immediately (even if not fully configured)
            // Force save without debounce to ensure stream configurations are persisted
            self.saveConfiguration(self.configuration)
            
            // Also trigger UserDefaults synchronization to ensure it's written to disk
            UserDefaults.standard.synchronize()
        }
        
        print("✅ Device verification completed")
        print("   Endpoint: \(configuration.apiEndpoint)")
        print("   Token: \(configuration.deviceToken.prefix(15))...")
        print("   Configuration state: \(self.configurationState)")
        print("   Configured streams: \(self.configuredStreamCount)")
        
        return verificationResponse.success  // Return true if token is valid (regardless of stream config)
    }
    
    // MARK: - Validation
    
    func validateEndpoint(_ endpoint: String) -> Bool {
        let trimmed = endpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        
        // Basic URL validation
        if trimmed.isEmpty { return false }
        
        // Check if it's a valid URL
        if let url = URL(string: trimmed) {
            // Allow http for local development
            return url.scheme == "http" || url.scheme == "https"
        }
        
        return false
    }
    
    func validateToken(_ token: String) -> Bool {
        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        return !trimmed.isEmpty && trimmed.count >= 6 // Minimum token length
    }
    
    // MARK: - Status Helpers
    
    var hasValidConfiguration: Bool {
        return validateEndpoint(configuration.apiEndpoint) && 
               validateToken(configuration.deviceToken)
    }
    
    var statusMessage: String {
        if isConfigured {
            if let configuredDate = configuration.configuredDate {
                let formatter = RelativeDateTimeFormatter()
                formatter.unitsStyle = .abbreviated
                return "Configured \(formatter.localizedString(for: configuredDate, relativeTo: Date()))"
            }
            return "Device configured"
        } else if !configuration.apiEndpoint.isEmpty || !configuration.deviceToken.isEmpty {
            return "Not configured - complete setup"
        } else {
            return "Not configured"
        }
    }
    
    // MARK: - Debug Helpers
    
    func getDebugInfo() -> String {
        var info = "Device Configuration:\n"
        info += "- Device ID: \(configuration.deviceId)\n"
        info += "- Configured: \(isConfigured)\n"
        info += "- Endpoint: \(configuration.apiEndpoint.isEmpty ? "Not set" : configuration.apiEndpoint)\n"
        info += "- Token: \(configuration.deviceToken.isEmpty ? "Not set" : "***\(String(configuration.deviceToken.suffix(4)))")\n"
        
        return info
    }
}