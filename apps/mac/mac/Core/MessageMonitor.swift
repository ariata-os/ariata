import Foundation
import SQLite3

class MessageMonitor {
    private let queue: Queue
    private var lastSyncDate: Date?
    private let dbPath = NSString(string: "~/Library/Messages/chat.db").expandingTildeInPath
    private var timer: DispatchSourceTimer?
    private let syncInterval: TimeInterval = 300 // 5 minutes
    
    // Configuration
    private let initialSyncDays = 7 // Default: sync last 7 days on initial sync
    private let batchSize = 500
    
    // Full Disk Access detection
    private var hasFullDiskAccess = false
    private var lastPermissionCheck = Date.distantPast
    private let permissionCheckInterval: TimeInterval = 300 // Check every 5 minutes
    private var permissionCheckAttempts = 0
    
    init(queue: Queue) {
        self.queue = queue
        loadLastSyncDate()
        // Check initial permission state
        hasFullDiskAccess = canAccessMessagesDB()
    }
    
    func start() {
        print("Starting message monitor...")
        print("  Database path: \(dbPath)")
        print("  Has Full Disk Access: \(hasFullDiskAccess)")
        if let lastSync = lastSyncDate {
            print("  Last sync date: \(ISO8601DateFormatter().string(from: lastSync))")
        } else {
            print("  Last sync date: nil (will perform initial \(initialSyncDays)-day sync)")
        }

        // Perform initial sync asynchronously to avoid blocking caller
        DispatchQueue.global(qos: .background).async { [weak self] in
            self?.syncMessages()
        }

        // Set up periodic sync using DispatchSourceTimer (more reliable than Timer for background execution)
        let syncTimer = DispatchSource.makeTimerSource(queue: .global(qos: .background))
        syncTimer.schedule(deadline: .now() + syncInterval, repeating: syncInterval)
        syncTimer.setEventHandler { [weak self] in
            self?.syncMessages()
        }
        syncTimer.resume()
        self.timer = syncTimer

        print("Message monitor started (syncing every \(Int(syncInterval)) seconds)")
    }
    
    func stop() {
        timer?.cancel()
        timer = nil
        saveLastSyncDate()
        print("Message monitor stopped")
    }
    
    private func syncMessages() {
        // Check for Full Disk Access if we don't have it yet
        if !hasFullDiskAccess {
            let now = Date()
            if now.timeIntervalSince(lastPermissionCheck) >= permissionCheckInterval {
                lastPermissionCheck = now
                permissionCheckAttempts += 1
                
                if canAccessMessagesDB() {
                    print("✅ Full Disk Access detected! Starting iMessage sync...")
                    hasFullDiskAccess = true
                    // Reset attempts counter
                    permissionCheckAttempts = 0
                    // Fall through to perform sync
                } else {
                    if permissionCheckAttempts == 1 {
                        print("⚠️ Cannot read Messages database - Full Disk Access required")
                        print("   To enable: System Settings → Privacy & Security → Full Disk Access → Add ariata-mac")
                        print("   Ariata will automatically detect when permission is granted (checking every 5 minutes)")
                    } else if permissionCheckAttempts % 12 == 0 { // Log every hour
                        print("⏳ Still waiting for Full Disk Access (checked \(permissionCheckAttempts) times)")
                    }
                    return
                }
            } else {
                // Not time to check yet, skip this sync cycle
                return
            }
        }
        
        print("Syncing messages...")
        
        guard FileManager.default.fileExists(atPath: dbPath) else {
            print("⚠️ Messages database not found at: \(dbPath)")
            return
        }
        
        var db: OpaquePointer?
        defer {
            if db != nil {
                sqlite3_close(db)
            }
        }
        
        // Open database in read-only mode
        if sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil) != SQLITE_OK {
            let error = String(cString: sqlite3_errmsg(db))
            // If we lose access, reset the flag
            if error.contains("authorization denied") || error.contains("Operation not permitted") {
                hasFullDiskAccess = false
                print("⚠️ Lost Full Disk Access - will check again in 5 minutes")
            } else {
                print("⚠️ Unable to open messages database: \(error)")
            }
            return
        }
        
        print("✓ Opened Messages database successfully")
        
        // Determine sync window
        let syncFromDate: Date
        if let lastSync = lastSyncDate {
            // Incremental sync: from last sync date
            syncFromDate = lastSync
            print("Incremental sync from: \(ISO8601DateFormatter().string(from: syncFromDate))")
        } else {
            // Initial sync: last N days
            syncFromDate = Calendar.current.date(byAdding: .day, value: -initialSyncDays, to: Date()) ?? Date()
            print("Initial sync from: \(ISO8601DateFormatter().string(from: syncFromDate))")
        }
        
        // Convert to Core Data timestamp
        let coreDataTimestamp = dateToCoreDateTimestamp(syncFromDate)
        
        // Safety check: ensure timestamp is reasonable (not in far future)
        // Messages timestamps should be between 2001 and current time
        let maxTimestamp = dateToCoreDateTimestamp(Date()) + (365 * 24 * 60 * 60 * 1_000_000_000) // 1 year in future max
        guard coreDataTimestamp < maxTimestamp else {
            print("⚠️ Invalid sync timestamp detected. Resetting to 7 days ago.")
            lastSyncDate = nil
            syncMessages() // Retry with fresh sync
            return
        }
        
        // Query for messages
        let query = """
            SELECT
                m.guid as message_id,
                c.guid as chat_id,
                m.handle_id,
                m.text,
                m.attributedBody,
                m.service,
                m.is_from_me,
                m.date,
                m.date_read,
                m.date_delivered,
                m.is_read,
                m.is_delivered,
                m.is_sent,
                m.cache_has_attachments,
                c.display_name as group_title,
                m.associated_message_guid,
                m.associated_message_type,
                m.expressive_send_style_id,
                (SELECT COUNT(*) FROM message_attachment_join WHERE message_id = m.ROWID) as attachment_count
            FROM message m
            LEFT JOIN chat_message_join cmj ON m.ROWID = cmj.message_id
            LEFT JOIN chat c ON cmj.chat_id = c.ROWID
            WHERE m.date > ?
            ORDER BY m.date ASC
            LIMIT ?
        """
        
        var statement: OpaquePointer?
        defer {
            if statement != nil {
                sqlite3_finalize(statement)
            }
        }
        
        if sqlite3_prepare_v2(db, query, -1, &statement, nil) != SQLITE_OK {
            print("Failed to prepare query: \(String(cString: sqlite3_errmsg(db)))")
            return
        }
        
        // Bind parameters
        sqlite3_bind_int64(statement, 1, Int64(coreDataTimestamp))
        sqlite3_bind_int(statement, 2, Int32(batchSize))
        
        var messages: [Message] = []
        var latestMessageDate: Date?
        
        // Execute query and collect results
        var rowCount = 0
        while sqlite3_step(statement) == SQLITE_ROW {
            guard let stmt = statement else { continue }
            
            rowCount += 1
            if rowCount % 100 == 0 {
                print("Processing message \(rowCount)...")
            }
            
            let message = parseMessageRow(statement: stmt)
            messages.append(message)
            
            // Track the latest message date for next sync (only if valid)
            let calendar = Calendar.current
            let year = calendar.component(.year, from: message.date)
            if year >= 2000 && year <= 2100 {
                if latestMessageDate == nil || message.date > latestMessageDate! {
                    latestMessageDate = message.date
                }
            } else {
                print("⚠️ Skipping message with invalid date: \(message.date) (year: \(year))")
            }
        }
        
        if messages.isEmpty {
            print("No new messages to sync")
        } else {
            print("Found \(messages.count) messages to sync")
            
            // Add messages to queue for upload
            for message in messages {
                queue.addMessage(message)
            }
            
            // Update last sync date
            if let latestDate = latestMessageDate {
                lastSyncDate = latestDate
                saveLastSyncDate()
            }
        }
    }
    
    private func parseMessageRow(statement: OpaquePointer) -> Message {
        // Extract all fields from the query result
        let messageId = sqlite3_column_type(statement, 0) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 0))
            : ""
        let chatId = sqlite3_column_type(statement, 1) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 1))
            : ""

        let handleId: String? = sqlite3_column_type(statement, 2) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 2))
            : nil

        var text: String? = sqlite3_column_type(statement, 3) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 3))
            : nil

        // Extract attributedBody blob for sent messages (column 4)
        if text == nil, sqlite3_column_type(statement, 4) == SQLITE_BLOB {
            if let blobPointer = sqlite3_column_blob(statement, 4) {
                let blobSize = sqlite3_column_bytes(statement, 4)
                let data = Data(bytes: blobPointer, count: Int(blobSize))
                text = extractTextFromAttributedBody(data)
            }
        }

        let service = sqlite3_column_type(statement, 5) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 5))
            : "iMessage"

        let isFromMe = sqlite3_column_int(statement, 6) != 0

        // Convert Core Data timestamps to Date
        let dateTimestamp = Double(sqlite3_column_int64(statement, 7))
        let date = Message.dateFromCoreDataTimestamp(dateTimestamp)

        let dateRead: Date? = sqlite3_column_type(statement, 8) != SQLITE_NULL
            ? Message.dateFromCoreDataTimestamp(Double(sqlite3_column_int64(statement, 8)))
            : nil

        let dateDelivered: Date? = sqlite3_column_type(statement, 9) != SQLITE_NULL
            ? Message.dateFromCoreDataTimestamp(Double(sqlite3_column_int64(statement, 9)))
            : nil

        let isRead = sqlite3_column_int(statement, 10) != 0
        let isDelivered = sqlite3_column_int(statement, 11) != 0
        let isSent = sqlite3_column_int(statement, 12) != 0
        let cacheHasAttachments = sqlite3_column_int(statement, 13) != 0

        let groupTitle: String? = sqlite3_column_type(statement, 14) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 14))
            : nil

        let associatedMessageGuid: String? = sqlite3_column_type(statement, 15) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 15))
            : nil

        let associatedMessageType: Int? = sqlite3_column_type(statement, 16) != SQLITE_NULL
            ? Int(sqlite3_column_int(statement, 16))
            : nil

        let expressiveSendStyleId: String? = sqlite3_column_type(statement, 17) != SQLITE_NULL
            ? String(cString: sqlite3_column_text(statement, 17))
            : nil

        let attachmentCount: Int? = sqlite3_column_type(statement, 18) != SQLITE_NULL
            ? Int(sqlite3_column_int(statement, 18))
            : nil
        
        return Message(
            messageId: messageId,
            chatId: chatId,
            handleId: handleId,
            text: text,
            service: service,
            isFromMe: isFromMe,
            date: date,
            dateRead: dateRead,
            dateDelivered: dateDelivered,
            isRead: isRead,
            isDelivered: isDelivered,
            isSent: isSent,
            cacheHasAttachments: cacheHasAttachments,
            attachmentCount: attachmentCount,
            attachmentInfo: nil, // TODO: Query attachments separately if needed
            groupTitle: groupTitle,
            associatedMessageGuid: associatedMessageGuid,
            associatedMessageType: associatedMessageType,
            expressiveSendStyleId: expressiveSendStyleId
        )
    }

    /// Extract plain text from NSAttributedString blob stored in Messages database
    /// The attributedBody field contains an archived NSAttributedString
    private func extractTextFromAttributedBody(_ data: Data) -> String? {
        guard !data.isEmpty else { return nil }

        do {
            // Try to unarchive the NSAttributedString from the blob
            if let attributedString = try NSKeyedUnarchiver.unarchivedObject(ofClass: NSAttributedString.self, from: data) {
                let text = attributedString.string
                // Only return non-empty strings
                return text.isEmpty ? nil : text
            }
        } catch {
            // If decoding fails, try legacy unarchiver (for older macOS versions)
            do {
                if let attributedString = try NSKeyedUnarchiver.unarchiveTopLevelObjectWithData(data) as? NSAttributedString {
                    let text = attributedString.string
                    return text.isEmpty ? nil : text
                }
            } catch {
                // Silently fail - some messages may have unsupported formats
                return nil
            }
        }

        return nil
    }

    private func dateToCoreDateTimestamp(_ date: Date) -> Double {
        // Convert Date to Core Data timestamp (nanoseconds since 2001-01-01)
        let coreDataEpochOffset: TimeInterval = 978307200
        let secondsSince2001 = date.timeIntervalSince1970 - coreDataEpochOffset
        // Convert to nanoseconds for Messages.app database
        return secondsSince2001 * 1_000_000_000
    }
    
    private func loadLastSyncDate() {
        // Load from UserDefaults or local storage
        if let storedDate = UserDefaults.standard.object(forKey: "ariata.messages.lastSyncDate") as? Date {
            // Validate the date is reasonable (between 2000 and 2100)
            let calendar = Calendar.current
            let year = calendar.component(.year, from: storedDate)
            
            if year >= 2000 && year <= 2100 {
                lastSyncDate = storedDate
                print("Loaded last sync date: \(ISO8601DateFormatter().string(from: storedDate))")
            } else {
                print("⚠️ Discarding invalid stored sync date (year \(year)). Will perform initial sync.")
                // Clear the corrupted value
                UserDefaults.standard.removeObject(forKey: "ariata.messages.lastSyncDate")
                lastSyncDate = nil
            }
        }
    }
    
    private func saveLastSyncDate() {
        if let date = lastSyncDate {
            // Validate before saving
            let calendar = Calendar.current
            let year = calendar.component(.year, from: date)
            
            if year >= 2000 && year <= 2100 {
                UserDefaults.standard.set(date, forKey: "ariata.messages.lastSyncDate")
                print("Saved last sync date: \(ISO8601DateFormatter().string(from: date))")
            } else {
                print("⚠️ Refusing to save invalid sync date (year \(year))")
            }
        }
    }
    
    private func canAccessMessagesDB() -> Bool {
        // Check file exists first
        guard FileManager.default.fileExists(atPath: dbPath) else {
            print("⚠️ Messages database file does not exist at: \(dbPath)")
            return false
        }

        // Try multiple times with small delays (WAL mode can cause transient locks)
        for attempt in 1...3 {
            var db: OpaquePointer?
            defer {
                if db != nil {
                    sqlite3_close(db)
                }
            }

            let result = sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil)
            if result == SQLITE_OK {
                return true
            }

            // If not last attempt, wait briefly and retry
            if attempt < 3 {
                Thread.sleep(forTimeInterval: 0.1)  // 100ms delay
            }
        }

        print("⚠️ Failed to open Messages database after 3 attempts")
        return false
    }
}