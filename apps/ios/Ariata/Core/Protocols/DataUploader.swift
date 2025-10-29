//
//  DataUploader.swift
//  Ariata
//
//  Protocol for uploading data to the backend
//  Enables dependency injection and testing
//

import Foundation

/// Provides data upload capabilities
protocol DataUploader {
    /// Update upload statistics
    func updateUploadStats()

    /// Trigger a manual upload immediately
    func triggerManualUpload() async

    /// Get queue size as formatted string
    func getQueueSizeString() -> String
}

// MARK: - BatchUploadCoordinator Extension

extension BatchUploadCoordinator: DataUploader {
    // Already implements all required methods
}
