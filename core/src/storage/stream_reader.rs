//! StreamReader - Read stream data from S3/object storage with checkpoint tracking
//!
//! This module provides the StreamReader abstraction for transforms to read stream data
//! from S3/object storage instead of querying Postgres tables. It supports:
//! - Checkpoint-based iteration (read only new data since last transform)
//! - Efficient JSONL parsing from S3 objects
//! - Decryption of encrypted stream data
//! - Batch processing with configurable batch size

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    storage::{Storage, models::StreamObject},
};

/// Configuration for StreamReader
#[derive(Debug, Clone)]
pub struct StreamReaderConfig {
    /// Maximum number of records to read per batch
    pub batch_size: usize,

    /// Maximum number of S3 objects to process in parallel
    pub max_parallel_objects: usize,

    /// Master encryption key for stream data decryption
    pub master_key: [u8; 32],
}

impl Default for StreamReaderConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            max_parallel_objects: 4,
            master_key: [0u8; 32], // Must be set from environment
        }
    }
}

/// StreamReader - Read stream data from S3/object storage
///
/// Provides checkpoint-based iteration over stream data stored in S3.
/// Transforms use this to read raw stream data and write normalized ontology data.
pub struct StreamReader {
    storage: Arc<Storage>,
    db: sqlx::PgPool,
    config: StreamReaderConfig,
}

impl StreamReader {
    /// Create a new StreamReader
    ///
    /// # Arguments
    /// * `storage` - Storage layer for S3 access
    /// * `db` - Database connection for checkpoint tracking
    /// * `config` - Reader configuration
    pub fn new(storage: Arc<Storage>, db: sqlx::PgPool, config: StreamReaderConfig) -> Self {
        Self { storage, db, config }
    }

    /// Read stream data since last checkpoint
    ///
    /// Returns batches of records that have been written since the last successful transform.
    /// Updates checkpoint after each batch is successfully processed.
    ///
    /// # Arguments
    /// * `source_id` - Source UUID
    /// * `stream_name` - Stream name (e.g., "healthkit", "gmail")
    /// * `checkpoint_key` - Unique key for this transform's checkpoint (e.g., "healthkit_to_heart_rate")
    ///
    /// # Returns
    /// Iterator of record batches with checkpoint metadata
    pub async fn read_with_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        checkpoint_key: &str,
    ) -> Result<Vec<StreamBatch>> {
        // Get last checkpoint timestamp
        let last_checkpoint = self.get_checkpoint(source_id, stream_name, checkpoint_key).await?;

        tracing::debug!(
            source_id = %source_id,
            stream_name,
            checkpoint_key,
            last_checkpoint = ?last_checkpoint,
            "Reading stream data with checkpoint"
        );

        // Find S3 objects written after checkpoint
        let objects = self.find_objects_after(source_id, stream_name, last_checkpoint).await?;

        if objects.is_empty() {
            tracing::debug!(
                source_id = %source_id,
                stream_name,
                "No new objects found after checkpoint"
            );
            return Ok(vec![]);
        }

        tracing::info!(
            source_id = %source_id,
            stream_name,
            object_count = objects.len(),
            "Found new objects to process"
        );

        // Read and parse objects into batches
        let mut batches = Vec::new();
        for object in objects {
            let records = self.read_object(&object).await?;

            if !records.is_empty() {
                batches.push(StreamBatch {
                    source_id,
                    stream_name: stream_name.to_string(),
                    records,
                    object_id: object.id,
                    max_timestamp: object.max_timestamp,
                });
            }
        }

        Ok(batches)
    }

    /// Update checkpoint after successful transform
    ///
    /// Call this after successfully processing a batch to record progress.
    ///
    /// # Arguments
    /// * `source_id` - Source UUID
    /// * `stream_name` - Stream name
    /// * `checkpoint_key` - Unique key for this transform's checkpoint
    /// * `timestamp` - Timestamp to checkpoint at (usually max_timestamp from batch)
    pub async fn update_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        checkpoint_key: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO elt.stream_checkpoints (source_id, stream_name, checkpoint_key, last_processed_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (source_id, stream_name, checkpoint_key)
            DO UPDATE SET
                last_processed_at = EXCLUDED.last_processed_at,
                updated_at = NOW()
            "#,
        )
        .bind(source_id)
        .bind(stream_name)
        .bind(checkpoint_key)
        .bind(timestamp)
        .execute(&self.db)
        .await
        .map_err(|e| Error::Database(format!("Failed to update checkpoint: {e}")))?;

        tracing::debug!(
            source_id = %source_id,
            stream_name,
            checkpoint_key,
            timestamp = %timestamp,
            "Updated checkpoint"
        );

        Ok(())
    }

    /// Get last checkpoint timestamp
    async fn get_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        checkpoint_key: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            r#"
            SELECT last_processed_at
            FROM elt.stream_checkpoints
            WHERE source_id = $1
              AND stream_name = $2
              AND checkpoint_key = $3
            "#,
        )
        .bind(source_id)
        .bind(stream_name)
        .bind(checkpoint_key)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::Database(format!("Failed to get checkpoint: {e}")))?;

        Ok(row.flatten())
    }

    /// Find S3 objects written after checkpoint
    async fn find_objects_after(
        &self,
        source_id: Uuid,
        stream_name: &str,
        after: Option<DateTime<Utc>>,
    ) -> Result<Vec<StreamObject>> {
        let query = if let Some(checkpoint) = after {
            sqlx::query_as::<_, StreamObject>(
                r#"
                SELECT id, source_id, stream_name, s3_key, record_count, size_bytes,
                       min_timestamp, max_timestamp, created_at
                FROM elt.stream_objects
                WHERE source_id = $1
                  AND stream_name = $2
                  AND max_timestamp > $3
                ORDER BY max_timestamp ASC
                "#,
            )
            .bind(source_id)
            .bind(stream_name)
            .bind(checkpoint)
        } else {
            // No checkpoint - read all objects
            sqlx::query_as::<_, StreamObject>(
                r#"
                SELECT id, source_id, stream_name, s3_key, record_count, size_bytes,
                       min_timestamp, max_timestamp, created_at
                FROM elt.stream_objects
                WHERE source_id = $1
                  AND stream_name = $2
                ORDER BY max_timestamp ASC
                "#,
            )
            .bind(source_id)
            .bind(stream_name)
        };

        let objects = query
            .fetch_all(&self.db)
            .await
            .map_err(|e| Error::Database(format!("Failed to fetch stream objects: {e}")))?;

        Ok(objects)
    }

    /// Read and parse a single S3 object
    async fn read_object(&self, object: &StreamObject) -> Result<Vec<Value>> {
        tracing::debug!(
            s3_key = %object.s3_key,
            record_count = object.record_count,
            "Reading S3 object"
        );

        // Download JSONL file from S3 (with decryption)
        let jsonl_bytes = self.storage
            .download_stream_jsonl(
                object.source_id,
                &object.stream_name,
                &object.s3_key,
                &self.config.master_key,
            )
            .await?;

        // Parse JSONL into records
        let jsonl_str = String::from_utf8(jsonl_bytes)
            .map_err(|e| Error::Other(format!("Invalid UTF-8 in JSONL: {e}")))?;

        let mut records = Vec::new();
        for (line_num, line) in jsonl_str.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(line) {
                Ok(record) => records.push(record),
                Err(e) => {
                    tracing::warn!(
                        s3_key = %object.s3_key,
                        line_num,
                        error = %e,
                        "Failed to parse JSONL line, skipping"
                    );
                }
            }
        }

        tracing::debug!(
            s3_key = %object.s3_key,
            records_parsed = records.len(),
            "Parsed S3 object"
        );

        Ok(records)
    }
}

/// A batch of stream records from a single S3 object
#[derive(Debug)]
pub struct StreamBatch {
    /// Source ID
    pub source_id: Uuid,

    /// Stream name
    pub stream_name: String,

    /// Records in this batch
    pub records: Vec<Value>,

    /// S3 object ID this batch came from
    pub object_id: Uuid,

    /// Maximum timestamp in this batch (for checkpointing)
    pub max_timestamp: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_reader_config_default() {
        let config = StreamReaderConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_parallel_objects, 4);
    }

    #[test]
    fn test_stream_batch_debug() {
        let batch = StreamBatch {
            source_id: Uuid::new_v4(),
            stream_name: "healthkit".to_string(),
            records: vec![serde_json::json!({"test": "data"})],
            object_id: Uuid::new_v4(),
            max_timestamp: Some(Utc::now()),
        };

        let debug_str = format!("{:?}", batch);
        assert!(debug_str.contains("healthkit"));
    }
}
