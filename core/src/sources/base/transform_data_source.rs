//! Data source abstraction for transforms
//!
//! Provides a unified interface for transforms to read data from
//! in-memory records (hot path). This enables direct transforms with async S3 archival.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::error::Result;

/// Batch of records from a stream
#[derive(Debug, Clone)]
pub struct StreamBatch {
    pub source_id: Uuid,
    pub stream_name: String,
    pub records: Vec<Value>,
    pub object_id: Uuid,
    pub max_timestamp: Option<DateTime<Utc>>,
}

/// Trait for providing data to transforms
///
/// Currently only supports:
/// - In-memory records (MemoryDataSource) - hot path for real-time transforms
#[async_trait]
pub trait TransformDataSource: Send + Sync {
    /// Read records with checkpoint tracking
    ///
    /// # Arguments
    ///
    /// * `source_id` - UUID of the data source
    /// * `stream_name` - Name of the stream (e.g., "app_export")
    /// * `checkpoint_key` - Unique key for this transform's checkpoint
    ///
    /// # Returns
    ///
    /// Vector of StreamBatch objects containing records and metadata
    async fn read_with_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        checkpoint_key: &str,
    ) -> Result<Vec<StreamBatch>>;

    /// Update checkpoint after successful processing
    ///
    /// # Arguments
    ///
    /// * `source_id` - UUID of the data source
    /// * `stream_name` - Name of the stream
    /// * `checkpoint_key` - Unique key for this transform's checkpoint
    /// * `timestamp` - Latest timestamp successfully processed
    async fn update_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        checkpoint_key: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()>;

    /// Get the data source type for logging/metrics
    fn source_type(&self) -> DataSourceType;
}

/// Type of data source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSourceType {
    /// In-memory records from sync job (hot path - only path)
    Memory,
}

impl std::fmt::Display for DataSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataSourceType::Memory => write!(f, "memory"),
        }
    }
}

/// In-memory data source for direct transforms (hot path)
///
/// Provides records directly from sync job memory without S3 round-trip.
/// This is used for real-time transforms where records are available immediately.
pub struct MemoryDataSource {
    /// In-memory records from sync job
    records: Vec<Value>,
    /// Source ID for these records
    source_id: Uuid,
    /// Stream name
    stream_name: String,
    /// Minimum timestamp in records (currently unused, reserved for future use)
    #[allow(dead_code)]
    min_timestamp: Option<DateTime<Utc>>,
    /// Maximum timestamp in records
    max_timestamp: Option<DateTime<Utc>>,
    /// Database connection for checkpoint updates
    db: sqlx::PgPool,
}

impl MemoryDataSource {
    /// Create a new memory data source
    pub fn new(
        records: Vec<Value>,
        source_id: Uuid,
        stream_name: String,
        min_timestamp: Option<DateTime<Utc>>,
        max_timestamp: Option<DateTime<Utc>>,
        db: sqlx::PgPool,
    ) -> Self {
        Self {
            records,
            source_id,
            stream_name,
            min_timestamp,
            max_timestamp,
            db,
        }
    }
}

#[async_trait]
impl TransformDataSource for MemoryDataSource {
    async fn read_with_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        _checkpoint_key: &str,
    ) -> Result<Vec<StreamBatch>> {
        // Validate source and stream match
        if source_id != self.source_id || stream_name != self.stream_name {
            return Err(crate::error::Error::InvalidInput(format!(
                "Mismatched source/stream: expected {}:{}, got {}:{}",
                self.source_id, self.stream_name, source_id, stream_name
            )));
        }

        // For memory source, we don't have object_id since data isn't in S3 yet
        // Use a nil UUID as placeholder
        let object_id = Uuid::nil();

        // Return all records as a single batch
        Ok(vec![StreamBatch {
            source_id: self.source_id,
            stream_name: self.stream_name.clone(),
            records: self.records.clone(),
            object_id,
            max_timestamp: self.max_timestamp,
        }])
    }

    async fn update_checkpoint(
        &self,
        source_id: Uuid,
        stream_name: &str,
        checkpoint_key: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        // Update checkpoint in database
        sqlx::query(
            "INSERT INTO elt.stream_checkpoints (source_id, stream_name, checkpoint_key, last_processed_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (source_id, stream_name, checkpoint_key)
             DO UPDATE SET last_processed_at = EXCLUDED.last_processed_at,
                           updated_at = NOW()",
        )
        .bind(source_id)
        .bind(stream_name)
        .bind(checkpoint_key)
        .bind(timestamp)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    fn source_type(&self) -> DataSourceType {
        DataSourceType::Memory
    }
}
