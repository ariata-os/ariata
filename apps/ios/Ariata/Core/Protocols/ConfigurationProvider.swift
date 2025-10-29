//
//  ConfigurationProvider.swift
//  Ariata
//
//  Protocol for providing device configuration to managers
//  Enables dependency injection and testing
//

import Foundation

/// Provides device configuration and stream settings
protocol ConfigurationProvider {
    /// Unique device identifier
    var deviceId: String { get }

    /// Whether the device is fully configured
    var isConfigured: Bool { get }

    /// The API endpoint URL
    var apiEndpoint: String { get }

    /// The device token for authentication
    var deviceToken: String { get }

    /// The full ingest URL for uploads
    var ingestURL: URL? { get }

    /// Check if a specific stream is enabled
    /// - Parameter streamKey: The stream identifier (e.g., "mic", "location", "healthkit")
    /// - Returns: Whether the stream is enabled in the web app configuration
    func isStreamEnabled(_ streamKey: String) -> Bool

    /// Get initial sync days for a stream
    /// - Parameter streamKey: The stream identifier
    /// - Returns: Number of days to sync initially
    func getInitialSyncDays(for streamKey: String) -> Int
}

/// Provides observable configuration updates for SwiftUI views
protocol ObservableConfigurationProvider: ConfigurationProvider, ObservableObject {
    /// Current configuration state
    var configurationState: DeviceConfigurationState { get }

    /// Number of configured streams
    var configuredStreamCount: Int { get }
}

// MARK: - DeviceConfiguration Extension

extension DeviceConfiguration: ConfigurationProvider {
    // Already implements all required properties through the struct
}

// MARK: - DeviceManager Extension

extension DeviceManager: ObservableConfigurationProvider {
    var deviceId: String {
        configuration.deviceId
    }

    var apiEndpoint: String {
        configuration.apiEndpoint
    }

    var deviceToken: String {
        configuration.deviceToken
    }

    var ingestURL: URL? {
        configuration.ingestURL
    }

    func isStreamEnabled(_ streamKey: String) -> Bool {
        configuration.isStreamEnabled(streamKey)
    }

    func getInitialSyncDays(for streamKey: String) -> Int {
        configuration.getInitialSyncDays(for: streamKey)
    }
}
