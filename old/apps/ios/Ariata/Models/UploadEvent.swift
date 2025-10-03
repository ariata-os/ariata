//
//  UploadEvent.swift
//  Ariata
//
//  SQLite model for upload queue with retry logic
//

import Foundation
import SQLite3

struct UploadEvent {
    let id: Int64
    let streamName: String
    let dataBlob: Data
    let createdAt: Date
    var uploadAttempts: Int
    var lastAttemptDate: Date?
    var status: UploadStatus
    
    enum UploadStatus: String {
        case pending = "pending"
        case uploading = "uploading"
        case failed = "failed"
        case completed = "completed"
    }
    
    init(id: Int64 = 0,
         streamName: String,
         dataBlob: Data,
         createdAt: Date = Date(),
         uploadAttempts: Int = 0,
         lastAttemptDate: Date? = nil,
         status: UploadStatus = .pending) {
        self.id = id
        self.streamName = streamName
        self.dataBlob = dataBlob
        self.createdAt = createdAt
        self.uploadAttempts = uploadAttempts
        self.lastAttemptDate = lastAttemptDate
        self.status = status
    }
    
    // Calculate next retry delay based on attempts
    var nextRetryDelay: TimeInterval {
        switch uploadAttempts {
        case 0: return 0
        case 1: return 30
        case 2: return 60
        case 3: return 120
        case 4: return 240
        default: return 300 // Max 5 minutes
        }
    }
    
    // Check if event should be retried
    var shouldRetry: Bool {
        guard uploadAttempts < 5 else { return false }
        guard status == .failed else { return false }
        
        if let lastAttempt = lastAttemptDate {
            let timeSinceLastAttempt = Date().timeIntervalSince(lastAttempt)
            return timeSinceLastAttempt >= nextRetryDelay
        }
        
        return true
    }
    
    // Check if event should be cleaned up (3 days old)
    var shouldCleanup: Bool {
        let threeDaysAgo = Date().addingTimeInterval(-3 * 24 * 60 * 60)
        return createdAt < threeDaysAgo && status == .completed
    }
    
    // Get data size in a readable format
    var dataSizeString: String {
        let bytes = dataBlob.count
        if bytes < 1024 {
            return "\(bytes) B"
        } else if bytes < 1024 * 1024 {
            return String(format: "%.1f KB", Double(bytes) / 1024.0)
        } else {
            return String(format: "%.1f MB", Double(bytes) / (1024.0 * 1024.0))
        }
    }
}

// SQL table creation statement
extension UploadEvent {
    static let createTableSQL = """
        CREATE TABLE IF NOT EXISTS upload_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stream_name TEXT NOT NULL,
            data_blob BLOB NOT NULL,
            created_at REAL NOT NULL,
            upload_attempts INTEGER DEFAULT 0,
            last_attempt_date REAL,
            status TEXT DEFAULT 'pending',
            CHECK (status IN ('pending', 'uploading', 'failed', 'completed'))
        );
        
        CREATE INDEX IF NOT EXISTS idx_status ON upload_queue(status);
        CREATE INDEX IF NOT EXISTS idx_created_at ON upload_queue(created_at);
    """
}