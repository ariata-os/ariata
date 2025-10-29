//
//  StorageProvider.swift
//  Ariata
//
//  Protocol for data storage operations
//  Enables dependency injection and testing
//

import Foundation

/// Provides SQLite storage operations
protocol StorageProvider {
    /// Enqueue data for upload
    /// - Parameters:
    ///   - streamName: The name of the stream (e.g., "ios_mic", "ios_location")
    ///   - data: The encoded data to store
    /// - Returns: Whether the operation succeeded
    func enqueue(streamName: String, data: Data) -> Bool

    /// Dequeue next batch of events for upload
    /// - Parameter limit: Maximum number of events to dequeue
    /// - Returns: Array of upload events
    func dequeueNext(limit: Int) -> [UploadEvent]

    /// Mark an event as successfully uploaded
    /// - Parameter id: The event ID
    func markAsComplete(id: Int64)

    /// Increment retry count for a failed upload
    /// - Parameter id: The event ID
    func incrementRetry(id: Int64)

    /// Get queue statistics
    func getQueueStats() -> (pending: Int, failed: Int, total: Int, totalSize: Int64)

    /// Get counts by stream type
    func getStreamCounts() -> (healthkit: Int, location: Int, audio: Int)

    /// Get total database size
    func getTotalDatabaseSize() -> Int64

    /// Clean up old events
    func cleanupOldEvents() -> Int

    /// Clean up bad events
    func cleanupBadEvents() -> Int
}

// MARK: - SQLiteManager Extension

extension SQLiteManager: StorageProvider {
    // Already implements all required methods
}
