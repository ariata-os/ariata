//! StreamWriter - Buffered writer for stream data to object storage
//!
//! This module provides a high-level abstraction for writing stream data to S3/MinIO
//! with automatic batching, encryption, and metadata tracking.
//!
//! # Features
//! - Automatic buffering (up to 1000 records or 10MB)
//! - Per-source/stream/date encryption with SSE-C
//! - JSONL format for efficient appending
//! - Metadata tracking in stream_objects table
//! - Automatic flushing on size/count thresholds

use std::collections::HashMap;
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::database::Database;
use crate::error::{Error, Result};
use crate::storage::{Storage, EncryptionKey, encryption, models::StreamKeyBuilder};

/// Configuration for stream writer
#[derive(Clone)]
pub struct StreamWriterConfig {
    /// Maximum number of records to buffer before flushing
    pub max_buffer_records: usize,

    /// Maximum buffer size in bytes before flushing
    pub max_buffer_bytes: usize,

    /// Master encryption key (32 bytes, from STREAM_ENCRYPTION_MASTER_KEY env var)
    pub master_key: [u8; 32],
}

impl Default for StreamWriterConfig {
    fn default() -> Self {
        Self {
            max_buffer_records: 1000,
            max_buffer_bytes: 10 * 1024 * 1024, // 10 MB
            master_key: [0u8; 32], // Must be set from environment
        }
    }
}

/// Buffer for a single stream
struct StreamBuffer {
    source_id: Uuid,
    stream_name: String,
    records: Vec<Value>,
    size_bytes: usize,
    min_timestamp: Option<chrono::DateTime<Utc>>,
    max_timestamp: Option<chrono::DateTime<Utc>>,
}

impl StreamBuffer {
    fn new(source_id: Uuid, stream_name: String) -> Self {
        Self {
            source_id,
            stream_name,
            records: Vec::new(),
            size_bytes: 0,
            min_timestamp: None,
            max_timestamp: None,
        }
    }

    fn add_record(&mut self, record: Value, timestamp: Option<chrono::DateTime<Utc>>) -> Result<()> {
        // Estimate size (serialized JSON + newline)
        let json_str = serde_json::to_string(&record)
            .map_err(|e| Error::Other(format!("Failed to serialize record: {}", e)))?;
        self.size_bytes += json_str.len() + 1; // +1 for newline

        // Update timestamp range
        if let Some(ts) = timestamp {
            self.min_timestamp = Some(match self.min_timestamp {
                Some(min) if ts < min => ts,
                Some(min) => min,
                None => ts,
            });
            self.max_timestamp = Some(match self.max_timestamp {
                Some(max) if ts > max => ts,
                Some(max) => max,
                None => ts,
            });
        }

        self.records.push(record);
        Ok(())
    }

    fn should_flush(&self, config: &StreamWriterConfig) -> bool {
        self.records.len() >= config.max_buffer_records || self.size_bytes >= config.max_buffer_bytes
    }

    fn clear(&mut self) {
        self.records.clear();
        self.size_bytes = 0;
        self.min_timestamp = None;
        self.max_timestamp = None;
    }
}

/// High-level writer for stream data
pub struct StreamWriter {
    storage: Storage,
    db: Database,
    config: StreamWriterConfig,
    buffers: HashMap<String, StreamBuffer>,
}

impl StreamWriter {
    /// Create a new stream writer
    pub fn new(storage: Storage, db: Database, config: StreamWriterConfig) -> Self {
        Self {
            storage,
            db,
            config,
            buffers: HashMap::new(),
        }
    }

    /// Write a single record to the stream
    ///
    /// Records are buffered in memory until flush threshold is reached.
    ///
    /// # Arguments
    /// * `source_id` - UUID of the source (user/device)
    /// * `stream_name` - Name of the stream (e.g., "healthkit", "location")
    /// * `record` - JSON record to write
    /// * `timestamp` - Optional timestamp for the record (used for metadata tracking)
    pub async fn write_record(
        &mut self,
        source_id: Uuid,
        stream_name: &str,
        record: Value,
        timestamp: Option<chrono::DateTime<Utc>>,
    ) -> Result<()> {
        let buffer_key = format!("{}:{}", source_id, stream_name);

        // Get or create buffer
        let buffer = self.buffers
            .entry(buffer_key.clone())
            .or_insert_with(|| StreamBuffer::new(source_id, stream_name.to_string()));

        // Add record to buffer
        buffer.add_record(record, timestamp)?;

        // Flush if threshold reached
        if buffer.should_flush(&self.config) {
            self.flush_buffer(&buffer_key).await?;
        }

        Ok(())
    }

    /// Flush a specific buffer to storage
    async fn flush_buffer(&mut self, buffer_key: &str) -> Result<()> {
        let buffer = match self.buffers.get_mut(buffer_key) {
            Some(b) if !b.records.is_empty() => b,
            _ => return Ok(()), // Nothing to flush
        };

        let source_id = buffer.source_id;
        let stream_name = buffer.stream_name.clone();
        let records = std::mem::take(&mut buffer.records);
        let min_timestamp = buffer.min_timestamp;
        let max_timestamp = buffer.max_timestamp;

        // Get current date (for key partitioning and encryption)
        let date = Utc::now().date_naive();

        // Generate S3 key
        let key_builder = StreamKeyBuilder::new(source_id, &stream_name, date);
        let s3_key = key_builder.build();

        // Derive encryption key
        let encryption_key_bytes = encryption::derive_stream_key(
            &self.config.master_key,
            source_id,
            &stream_name,
            date,
        )?;
        let encryption_key = EncryptionKey {
            key_base64: encryption::encode_key_base64(&encryption_key_bytes),
        };

        // Upload to S3 as JSONL with encryption
        self.storage.upload_jsonl_encrypted(&s3_key, &records, &encryption_key).await?;

        // Calculate actual size (after serialization)
        let actual_size = records.iter()
            .map(|r| serde_json::to_string(r).map(|s| s.len() + 1).unwrap_or(0))
            .sum::<usize>() as i64;

        // Record metadata in database
        sqlx::query(
            "INSERT INTO elt.stream_objects
             (source_id, stream_name, s3_key, record_count, size_bytes, min_timestamp, max_timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(source_id)
        .bind(&stream_name)
        .bind(&s3_key)
        .bind(records.len() as i32)
        .bind(actual_size)
        .bind(min_timestamp)
        .bind(max_timestamp)
        .execute(self.db.pool())
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // Clear buffer
        buffer.clear();

        tracing::info!(
            source_id = %source_id,
            stream_name = %stream_name,
            s3_key = %s3_key,
            record_count = records.len(),
            size_bytes = actual_size,
            "Flushed stream buffer to S3"
        );

        Ok(())
    }

    /// Flush all buffers to storage
    ///
    /// Call this before shutting down or when you want to ensure all data is persisted.
    pub async fn flush_all(&mut self) -> Result<()> {
        let buffer_keys: Vec<String> = self.buffers.keys().cloned().collect();

        for key in buffer_keys {
            self.flush_buffer(&key).await?;
        }

        Ok(())
    }

    /// Get current buffer statistics (for monitoring)
    pub fn buffer_stats(&self) -> Vec<BufferStats> {
        self.buffers.values().map(|buffer| BufferStats {
            source_id: buffer.source_id,
            stream_name: buffer.stream_name.clone(),
            record_count: buffer.records.len(),
            size_bytes: buffer.size_bytes,
        }).collect()
    }
}

/// Buffer statistics for monitoring
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub source_id: Uuid,
    pub stream_name: String,
    pub record_count: usize,
    pub size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stream_buffer() {
        let source_id = Uuid::new_v4();
        let mut buffer = StreamBuffer::new(source_id, "test_stream".to_string());

        let record1 = json!({"value": 100, "timestamp": "2025-01-15T10:00:00Z"});
        let ts1 = chrono::DateTime::parse_from_rfc3339("2025-01-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        buffer.add_record(record1, Some(ts1)).unwrap();

        assert_eq!(buffer.records.len(), 1);
        assert!(buffer.size_bytes > 0);
        assert_eq!(buffer.min_timestamp, Some(ts1));
        assert_eq!(buffer.max_timestamp, Some(ts1));

        let record2 = json!({"value": 200, "timestamp": "2025-01-15T11:00:00Z"});
        let ts2 = chrono::DateTime::parse_from_rfc3339("2025-01-15T11:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        buffer.add_record(record2, Some(ts2)).unwrap();

        assert_eq!(buffer.records.len(), 2);
        assert_eq!(buffer.min_timestamp, Some(ts1));
        assert_eq!(buffer.max_timestamp, Some(ts2));
    }

    #[test]
    fn test_should_flush() {
        let source_id = Uuid::new_v4();
        let mut buffer = StreamBuffer::new(source_id, "test_stream".to_string());

        let config = StreamWriterConfig {
            max_buffer_records: 2,
            max_buffer_bytes: 1000,
            master_key: [0u8; 32],
        };

        assert!(!buffer.should_flush(&config));

        buffer.add_record(json!({"value": 1}), None).unwrap();
        assert!(!buffer.should_flush(&config));

        buffer.add_record(json!({"value": 2}), None).unwrap();
        assert!(buffer.should_flush(&config)); // Reached max_buffer_records
    }
}
