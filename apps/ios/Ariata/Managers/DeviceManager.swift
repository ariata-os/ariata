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
        } else {
            self.configuration = DeviceConfiguration()
            self.isConfigured = false
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

        return true
    }

    private func saveConfiguration(_ config: DeviceConfiguration) {
        if let encoded = try? JSONEncoder().encode(config) {
            userDefaults.set(encoded, forKey: configKey)
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
        
        // Check if this looks like a pairing code (6 uppercase letters) or device token (longer, base64-like)
        let isPairingCode = configuration.deviceToken.count == 6 && configuration.deviceToken.allSatisfy { $0.isUppercase || $0.isNumber }

        let verificationResponse: NetworkManager.VerificationResponse

        if isPairingCode {
            // First-time pairing - complete pairing with code
            let pairingURL = baseURL.appendingPathComponent("/api/devices/pairing/complete")

            verificationResponse = await NetworkManager.shared.completePairing(
                endpoint: pairingURL,
                pairingCode: configuration.deviceToken,
                deviceInfo: [
                    "device_id": configuration.deviceId,
                    "device_name": configuration.deviceName,
                    "device_model": UIDevice.current.model,
                    "os_version": UIDevice.current.systemVersion,
                    "app_version": Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0.0"
                ]
            )

            if !verificationResponse.success {
                let errorMsg = NetworkManager.shared.lastError?.errorDescription ?? "Failed to complete pairing. Please check the code and try again."
                print("❌ Pairing failed: \(errorMsg)")

                await MainActor.run {
                    self.lastError = errorMsg
                    self.configurationState = .notConfigured
                }
                return false
            }

            // Pairing succeeded! Store the device token from the response
            if let deviceToken = verificationResponse.deviceToken {
                await MainActor.run {
                    self.configuration.deviceToken = deviceToken
                }
                saveConfiguration(configuration)
            }
        } else {
            // Already paired - verify the token and get stream configuration status
            let verifyURL = baseURL.appendingPathComponent("/api/devices/verify")

            verificationResponse = await NetworkManager.shared.verifyDeviceToken(
                endpoint: verifyURL,
                deviceToken: configuration.deviceToken
            )

            if !verificationResponse.success {
                let errorMsg = NetworkManager.shared.lastError?.errorDescription ?? "Failed to verify device. Please check your connection."
                await MainActor.run {
                    self.lastError = errorMsg
                    self.configurationState = .notConfigured
                }
                return false
            }
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
            } else {
                // Token valid but waiting for stream configuration
                self.isConfigured = false  // Don't mark as fully configured yet
                self.configurationState = .tokenValid
                self.lastError = nil  // Clear any errors since token is valid
            }
            
            // Always save the configuration immediately (even if not fully configured)
            // Force save without debounce to ensure stream configurations are persisted
            self.saveConfiguration(self.configuration)

            // Also trigger UserDefaults synchronization to ensure it's written to disk
            UserDefaults.standard.synchronize()
        }

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