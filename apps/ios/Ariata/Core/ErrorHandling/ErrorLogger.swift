//
//  ErrorLogger.swift
//  Ariata
//
//  Centralized error logging and telemetry for data collection errors
//

import Foundation

/// Centralized error logger for tracking and reporting data collection failures
final class ErrorLogger {
    static let shared = ErrorLogger()

    // MARK: - Error Tracking

    private struct ErrorRecord {
        let error: any DataCollectionError
        let timestamp: Date
        let deviceId: String
    }

    private var errorHistory: [ErrorRecord] = []
    private let maxHistorySize = 100
    private let lock = NSLock()

    // MARK: - Statistics

    private(set) var totalErrors: Int = 0
    private(set) var errorsByStream: [StreamType: Int] = [:]
    private(set) var recoverableErrors: Int = 0
    private(set) var nonRecoverableErrors: Int = 0

    private init() {}

    // MARK: - Logging Methods

    /// Log a data collection error
    /// - Parameters:
    ///   - error: The error that occurred
    ///   - deviceId: The device ID for context
    func log(_ error: AnyDataCollectionError, deviceId: String = "") {
        lock.lock()
        defer { lock.unlock() }

        let baseError = error.base

        // Update statistics
        totalErrors += 1
        errorsByStream[baseError.streamType, default: 0] += 1

        if baseError.isRecoverable {
            recoverableErrors += 1
        } else {
            nonRecoverableErrors += 1
        }

        // Add to history (with circular buffer logic)
        let record = ErrorRecord(error: baseError, timestamp: Date(), deviceId: deviceId)
        errorHistory.append(record)

        if errorHistory.count > maxHistorySize {
            errorHistory.removeFirst()
        }

        // Log to console with structured format
        logToConsole(error: baseError, deviceId: deviceId)

        // TODO: Send to remote telemetry service when implemented
        // sendToTelemetry(error: baseError, deviceId: deviceId)
    }

    /// Log a successful retry after an error
    /// - Parameters:
    ///   - streamType: The stream that recovered
    ///   - attemptNumber: Which attempt succeeded
    func logSuccessfulRetry(streamType: StreamType, attemptNumber: Int) {
        print("✅ [\(streamType.rawValue)] Retry succeeded on attempt #\(attemptNumber)")
    }

    // MARK: - Private Logging Methods

    private func logToConsole(error: any DataCollectionError, deviceId: String) {
        let emoji = error.isRecoverable ? "⚠️" : "❌"
        let recoverable = error.isRecoverable ? "recoverable" : "non-recoverable"

        print("\(emoji) [\(error.streamType.rawValue)] \(error.errorDescription) (\(recoverable))")

        // Log context if available
        if !error.context.isEmpty {
            print("   Context: \(error.context)")
        }

        // Log specific error type details
        switch error {
        case let encodingError as DataEncodingError:
            print("   Encoding error: \(encodingError.underlyingError.localizedDescription)")
            if let size = encodingError.dataSize {
                print("   Data size: \(size) bytes")
            }

        case let storageError as StorageError:
            print("   Storage reason: \(storageError.reason)")
            print("   Attempt: \(storageError.attemptNumber)")

        case let permissionError as PermissionError:
            print("   Permission: \(permissionError.permissionType)")

        case let collectionError as CollectionError:
            print("   Collection reason: \(collectionError.reason)")
            if let underlying = collectionError.underlyingError {
                print("   Underlying: \(underlying.localizedDescription)")
            }

        case let configError as ConfigurationError:
            print("   Config reason: \(configError.reason)")

        default:
            break
        }
    }

    // MARK: - Statistics and Reporting

    /// Get a summary of recent errors
    func getErrorSummary() -> String {
        lock.lock()
        defer { lock.unlock() }

        var summary = "Error Statistics:\n"
        summary += "  Total: \(totalErrors)\n"
        summary += "  Recoverable: \(recoverableErrors)\n"
        summary += "  Non-recoverable: \(nonRecoverableErrors)\n"
        summary += "\nBy Stream:\n"

        for (stream, count) in errorsByStream.sorted(by: { $0.key.rawValue < $1.key.rawValue }) {
            summary += "  \(stream.rawValue): \(count)\n"
        }

        return summary
    }

    /// Get recent error history
    func getRecentErrors(limit: Int = 10) -> [(timestamp: Date, description: String, streamType: StreamType)] {
        lock.lock()
        defer { lock.unlock() }

        return errorHistory
            .suffix(limit)
            .map { record in
                (
                    timestamp: record.timestamp,
                    description: record.error.errorDescription,
                    streamType: record.error.streamType
                )
            }
    }

    /// Clear error history and statistics (useful for testing)
    func clear() {
        lock.lock()
        defer { lock.unlock() }

        errorHistory.removeAll()
        totalErrors = 0
        errorsByStream.removeAll()
        recoverableErrors = 0
        nonRecoverableErrors = 0
    }

    // MARK: - Future: Remote Telemetry

    /// Send error to remote telemetry service (placeholder for future implementation)
    private func sendToTelemetry(error: any DataCollectionError, deviceId: String) {
        // TODO: Implement remote telemetry when backend endpoint is ready
        // This would batch errors and send them periodically to avoid network overhead
    }
}
