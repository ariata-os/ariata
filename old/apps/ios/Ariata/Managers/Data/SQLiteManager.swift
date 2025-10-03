//
//  SQLiteManager.swift
//  Ariata
//
//  Manages SQLite database for upload queue with retry logic
//

import Foundation
import SQLite3

class SQLiteManager {
    static let shared = SQLiteManager()
    
    private var db: OpaquePointer?
    private let dbPath: String
    private let queue = DispatchQueue(label: "com.ariata.sqlite")
    
    private init() {
        // Get documents directory
        let documentsPath = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true).first!
        dbPath = "\(documentsPath)/ariata_upload_queue.db"
        
        // Initialize database on the serial queue
        queue.sync {
            openDatabase()
            createTables()
        }
    }
    
    deinit {
        closeDatabase()
    }
    
    // MARK: - Database Setup
    
    private func openDatabase() {
        if sqlite3_open(dbPath, &db) != SQLITE_OK {
            print("Unable to open database at \(dbPath)")
        }
    }
    
    private func closeDatabase() {
        if db != nil {
            sqlite3_close(db)
            db = nil
        }
    }
    
    private func createTables() {
        let createTableString = UploadEvent.createTableSQL
        
        if sqlite3_exec(db, createTableString, nil, nil, nil) != SQLITE_OK {
            print("Error creating upload_queue table")
        }
    }
    
    // MARK: - Queue Operations
    
    func enqueue(streamName: String, data: Data) -> Bool {
        return queue.sync {
            let insertSQL = """
                INSERT INTO upload_queue (stream_name, data_blob, created_at, upload_attempts, status)
                VALUES (?, ?, ?, 0, 'pending')
            """
            
            var statement: OpaquePointer?
            var success = false
            
            if sqlite3_prepare_v2(db, insertSQL, -1, &statement, nil) == SQLITE_OK {
                // Use NSString for proper memory handling with SQLite
                sqlite3_bind_text(statement, 1, (streamName as NSString).utf8String, -1, nil)
                sqlite3_bind_blob(statement, 2, (data as NSData).bytes, Int32(data.count), nil)
                sqlite3_bind_double(statement, 3, Date().timeIntervalSince1970)
                
                if sqlite3_step(statement) == SQLITE_DONE {
                    success = true
                    let rowId = sqlite3_last_insert_rowid(db)
                    print("✅ Enqueued event for stream: \(streamName) (ID: \(rowId), Size: \(data.count) bytes)")
                } else {
                    let errorMsg = String(cString: sqlite3_errmsg(db))
                    print("❌ Failed to enqueue event for stream: \(streamName) - Error: \(errorMsg)")
                }
            } else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                print("❌ Failed to prepare insert statement - Error: \(errorMsg)")
            }
            
            sqlite3_finalize(statement)
            return success
        }
    }
    
    func dequeueNext(limit: Int = 10) -> [UploadEvent] {
        var events: [UploadEvent] = []
        var eventIds: [Int64] = []
        
        // First, fetch the events
        queue.sync {
            let selectSQL = """
                SELECT id, stream_name, data_blob, created_at, upload_attempts, last_attempt_date, status
                FROM upload_queue
                WHERE status IN ('pending', 'failed')
                AND (upload_attempts < 5 OR upload_attempts IS NULL)
                ORDER BY created_at ASC
                LIMIT ?
            """
            
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, selectSQL, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_int(statement, 1, Int32(limit))
                
                while sqlite3_step(statement) == SQLITE_ROW {
                    let id = sqlite3_column_int64(statement, 0)
                    let streamName: String
                    if let streamNameCStr = sqlite3_column_text(statement, 1) {
                        streamName = String(cString: streamNameCStr)
                    } else {
                        streamName = ""
                    }
                    
                    let blobPointer = sqlite3_column_blob(statement, 2)
                    let blobSize = sqlite3_column_bytes(statement, 2)
                    let dataBlob = Data(bytes: blobPointer!, count: Int(blobSize))
                    
                    let createdAt = Date(timeIntervalSince1970: sqlite3_column_double(statement, 3))
                    let uploadAttempts = Int(sqlite3_column_int(statement, 4))
                    
                    let lastAttemptInterval = sqlite3_column_double(statement, 5)
                    let lastAttemptDate = lastAttemptInterval > 0 ? Date(timeIntervalSince1970: lastAttemptInterval) : nil
                    
                    let statusString: String
                    if let statusCStr = sqlite3_column_text(statement, 6) {
                        statusString = String(cString: statusCStr)
                    } else {
                        statusString = "pending"
                    }
                    let status = UploadEvent.UploadStatus(rawValue: statusString) ?? .pending
                    
                    let event = UploadEvent(
                        id: id,
                        streamName: streamName,
                        dataBlob: dataBlob,
                        createdAt: createdAt,
                        uploadAttempts: uploadAttempts,
                        lastAttemptDate: lastAttemptDate,
                        status: status
                    )
                    
                    // Only include if retry delay has passed and stream name is valid
                    if (event.shouldRetry || event.status == .pending) && !streamName.isEmpty {
                        events.append(event)
                        eventIds.append(id)
                        print("📦 Dequeued event id=\(id) stream=\(streamName) status=\(statusString)")
                    } else if streamName.isEmpty {
                        print("⚠️ Skipping event id=\(id) with empty stream name")
                    }
                }
            }
            
            sqlite3_finalize(statement)
        }
        
        // Then update their status in a separate sync block
        if !eventIds.isEmpty {
            queue.sync {
                for id in eventIds {
                    let updateSQL = """
                        UPDATE upload_queue
                        SET status = 'uploading'
                        WHERE id = ?
                    """
                    
                    var updateStatement: OpaquePointer?
                    
                    if sqlite3_prepare_v2(db, updateSQL, -1, &updateStatement, nil) == SQLITE_OK {
                        sqlite3_bind_int64(updateStatement, 1, id)
                        sqlite3_step(updateStatement)
                    }
                    
                    sqlite3_finalize(updateStatement)
                }
            }
        }
        
        return events
    }
    
    // This method is no longer needed since we update status in dequeueNext
    // Keeping it for compatibility but it's effectively a no-op
    func markAsUploading(id: Int64) {
        // Status is already set to 'uploading' by dequeueNext
        print("Warning: markAsUploading called but status should already be set by dequeueNext")
    }
    
    func markAsComplete(id: Int64) {
        queue.sync {
            let updateSQL = """
                UPDATE upload_queue
                SET status = 'completed'
                WHERE id = ?
            """
            
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, updateSQL, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_int64(statement, 1, id)
                sqlite3_step(statement)
            }
            
            sqlite3_finalize(statement)
        }
    }
    
    func incrementRetry(id: Int64) {
        queue.sync {
            let updateSQL = """
                UPDATE upload_queue
                SET status = 'failed',
                    upload_attempts = upload_attempts + 1,
                    last_attempt_date = ?
                WHERE id = ?
            """
            
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, updateSQL, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_double(statement, 1, Date().timeIntervalSince1970)
                sqlite3_bind_int64(statement, 2, id)
                sqlite3_step(statement)
            }
            
            sqlite3_finalize(statement)
        }
    }
    
    // MARK: - Cleanup Operations
    
    func cleanupBadEvents() -> Int {
        queue.sync {
            var deletedCount = 0
            
            print("🧹 Running cleanup for bad events...")
            
            // Delete events with empty stream names
            let deleteEmptySQL = """
                DELETE FROM upload_queue
                WHERE LENGTH(TRIM(stream_name)) = 0 OR stream_name IS NULL
            """
            
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, deleteEmptySQL, -1, &statement, nil) == SQLITE_OK {
                if sqlite3_step(statement) == SQLITE_DONE {
                    deletedCount = Int(sqlite3_changes(db))
                    if deletedCount > 0 {
                        print("🗑️ Cleaned up \(deletedCount) events with empty stream names")
                    } else {
                        print("✅ No bad events to clean up")
                    }
                } else {
                    print("❌ Failed to execute cleanup: \(String(cString: sqlite3_errmsg(db)))")
                }
            } else {
                print("❌ Failed to prepare cleanup statement: \(String(cString: sqlite3_errmsg(db)))")
            }
            
            sqlite3_finalize(statement)
            
            return deletedCount
        }
    }
    
    func cleanupOldEvents() -> Int {
        queue.sync {
            var deletedCount = 0
            
            // Delete completed events older than 3 days
            let deleteSQL = """
                DELETE FROM upload_queue
                WHERE status = 'completed'
                AND created_at < ?
            """
            
            var statement: OpaquePointer?
            let threeDaysAgo = Date().addingTimeInterval(-3 * 24 * 60 * 60).timeIntervalSince1970
            
            if sqlite3_prepare_v2(db, deleteSQL, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_double(statement, 1, threeDaysAgo)
                
                if sqlite3_step(statement) == SQLITE_DONE {
                    deletedCount = Int(sqlite3_changes(db))
                }
            }
            
            sqlite3_finalize(statement)
            
            // Also delete failed events with max retries older than 3 days
            let deleteFailedSQL = """
                DELETE FROM upload_queue
                WHERE status = 'failed'
                AND upload_attempts >= 5
                AND created_at < ?
            """
            
            if sqlite3_prepare_v2(db, deleteFailedSQL, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_double(statement, 1, threeDaysAgo)
                
                if sqlite3_step(statement) == SQLITE_DONE {
                    deletedCount += Int(sqlite3_changes(db))
                }
            }
            
            sqlite3_finalize(statement)
            
            return deletedCount
        }
    }
    
    // MARK: - Queue Statistics
    
    func getQueueStats() -> (pending: Int, failed: Int, total: Int, totalSize: Int64) {
        queue.sync {
            var pending = 0
            var failed = 0
            var total = 0
            var totalSize: Int64 = 0
            
            let statsSQL = """
                SELECT status, COUNT(*) as count, SUM(LENGTH(data_blob)) as size
                FROM upload_queue
                WHERE status IN ('pending', 'failed', 'uploading')
                GROUP BY status
            """
            
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, statsSQL, -1, &statement, nil) == SQLITE_OK {
                while sqlite3_step(statement) == SQLITE_ROW {
                    let status = String(cString: sqlite3_column_text(statement, 0))
                    let count = Int(sqlite3_column_int(statement, 1))
                    let size = sqlite3_column_int64(statement, 2)
                    
                    switch status {
                    case "pending", "uploading":
                        pending += count
                    case "failed":
                        failed += count
                    default:
                        break
                    }
                    
                    total += count
                    totalSize += size
                }
            }
            
            sqlite3_finalize(statement)
            
            return (pending, failed, total, totalSize)
        }
    }
    
    func getStreamCounts() -> (healthkit: Int, location: Int, audio: Int) {
        queue.sync {
            var healthkit = 0
            var location = 0
            var audio = 0
            
            let streamSQL = """
                SELECT stream_name, COUNT(*) as count
                FROM upload_queue
                WHERE status IN ('pending', 'uploading')
                GROUP BY stream_name
            """
            
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, streamSQL, -1, &statement, nil) == SQLITE_OK {
                while sqlite3_step(statement) == SQLITE_ROW {
                    let streamName = String(cString: sqlite3_column_text(statement, 0))
                    let count = Int(sqlite3_column_int(statement, 1))
                    
                    switch streamName {
                    case "ios_healthkit":
                        healthkit = count
                    case "ios_location":
                        location = count
                    case "ios_mic":
                        audio = count
                    default:
                        break
                    }
                }
            }
            
            sqlite3_finalize(statement)
            
            return (healthkit, location, audio)
        }
    }
    
    // MARK: - Debug Methods
    
    func debugPrintAllEvents() {
        queue.sync {
            print("🔍 DEBUG: All events in upload_queue:")
            
            let selectSQL = "SELECT id, stream_name, status, upload_attempts, LENGTH(data_blob) as size FROM upload_queue ORDER BY id"
            var statement: OpaquePointer?
            
            if sqlite3_prepare_v2(db, selectSQL, -1, &statement, nil) == SQLITE_OK {
                var count = 0
                while sqlite3_step(statement) == SQLITE_ROW {
                    count += 1
                    let id = sqlite3_column_int64(statement, 0)
                    
                    let streamName: String
                    if let streamNameCStr = sqlite3_column_text(statement, 1) {
                        streamName = String(cString: streamNameCStr)
                    } else {
                        streamName = "(NULL)"
                    }
                    
                    let status: String
                    if let statusCStr = sqlite3_column_text(statement, 2) {
                        status = String(cString: statusCStr)
                    } else {
                        status = "(NULL)"
                    }
                    
                    let attempts = Int(sqlite3_column_int(statement, 3))
                    let size = Int(sqlite3_column_int(statement, 4))
                    
                    print("  ID: \(id), Stream: '\(streamName)', Status: \(status), Attempts: \(attempts), Size: \(size) bytes")
                }
                print("  Total events: \(count)")
            }
            
            sqlite3_finalize(statement)
        }
    }
    
    // MARK: - Storage Management
    
    func getTotalDatabaseSize() -> Int64 {
        var size: Int64 = 0
        
        if let attributes = try? FileManager.default.attributesOfItem(atPath: dbPath) {
            size = attributes[.size] as? Int64 ?? 0
        }
        
        return size
    }
}