//
//  DataCollectionError.swift
//  Ariata
//
//  Unified error types for data collection across all streams
//

import Foundation

/// Protocol for all data collection errors
protocol DataCollectionError: Error {
    /// A user-friendly description of the error
    var errorDescription: String { get }

    /// Whether this error is recoverable with retry
    var isRecoverable: Bool { get }

    /// The stream type that encountered the error
    var streamType: StreamType { get }

    /// Additional context for error logging
    var context: [String: Any] { get }
}

/// Type-erased wrapper for DataCollectionError to use in Result types
struct AnyDataCollectionError: Error {
    let base: any DataCollectionError

    var errorDescription: String { base.errorDescription }
    var isRecoverable: Bool { base.isRecoverable }
    var streamType: StreamType { base.streamType }
    var context: [String: Any] { base.context }

    init(_ error: any DataCollectionError) {
        self.base = error
    }
}

/// Stream types in the application
enum StreamType: String {
    case audio = "ios_mic"
    case location = "ios_location"
    case healthKit = "ios_healthkit"
}

/// Data encoding errors when preparing data for storage
struct DataEncodingError: DataCollectionError {
    let streamType: StreamType
    let underlyingError: Error
    let dataSize: Int?

    var errorDescription: String {
        "Failed to encode \(streamType.rawValue) data"
    }

    var isRecoverable: Bool {
        // Encoding errors are usually not recoverable - indicates data corruption
        false
    }

    var context: [String: Any] {
        var ctx: [String: Any] = [
            "stream": streamType.rawValue,
            "underlying_error": underlyingError.localizedDescription
        ]
        if let size = dataSize {
            ctx["data_size"] = size
        }
        return ctx
    }
}

/// Storage errors when writing to SQLite
struct StorageError: DataCollectionError {
    let streamType: StreamType
    let reason: String
    let attemptNumber: Int

    var errorDescription: String {
        "Failed to store \(streamType.rawValue) data: \(reason)"
    }

    var isRecoverable: Bool {
        // Storage errors are recoverable with retry
        attemptNumber < 3
    }

    var context: [String: Any] {
        [
            "stream": streamType.rawValue,
            "reason": reason,
            "attempt": attemptNumber
        ]
    }
}

/// Permission errors when accessing device features
struct PermissionError: DataCollectionError {
    let streamType: StreamType
    let permissionType: String

    var errorDescription: String {
        "\(permissionType) permission denied for \(streamType.rawValue)"
    }

    var isRecoverable: Bool {
        // Permission errors require user action
        false
    }

    var context: [String: Any] {
        [
            "stream": streamType.rawValue,
            "permission": permissionType
        ]
    }
}

/// Data collection errors from system APIs
struct CollectionError: DataCollectionError {
    let streamType: StreamType
    let reason: String
    let underlyingError: Error?

    var errorDescription: String {
        "Failed to collect \(streamType.rawValue) data: \(reason)"
    }

    var isRecoverable: Bool {
        // Collection errors are usually transient
        true
    }

    var context: [String: Any] {
        var ctx: [String: Any] = [
            "stream": streamType.rawValue,
            "reason": reason
        ]
        if let error = underlyingError {
            ctx["underlying_error"] = error.localizedDescription
        }
        return ctx
    }
}

/// Configuration errors when stream setup fails
struct ConfigurationError: DataCollectionError {
    let streamType: StreamType
    let reason: String

    var errorDescription: String {
        "Configuration error for \(streamType.rawValue): \(reason)"
    }

    var isRecoverable: Bool {
        // Configuration errors may be recoverable after settings change
        true
    }

    var context: [String: Any] {
        [
            "stream": streamType.rawValue,
            "reason": reason
        ]
    }
}

// MARK: - Result Type Extensions

extension Result where Success == Void, Failure == AnyDataCollectionError {
    /// Creates a success result with no value
    static var success: Result<Void, AnyDataCollectionError> {
        .success(())
    }
}
