//! Storage helpers for sources

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::{
    error::Result,
    sources::SourceRecord,
};

/// Helper for storing source data
#[async_trait]
pub trait StorageHelper: Send + Sync {
    /// Store a batch of records
    async fn store_batch(&self, records: Vec<SourceRecord>) -> Result<usize>;

    /// Store a single record
    async fn store_record(&self, record: SourceRecord) -> Result<()>;

    /// Check if a record already exists
    async fn record_exists(&self, source: &str, id: &str) -> Result<bool>;

    /// Get the last sync time for a source
    async fn get_last_sync(&self, source: &str) -> Result<Option<DateTime<Utc>>>;

    /// Update the last sync time
    async fn update_last_sync(&self, source: &str, timestamp: DateTime<Utc>) -> Result<()>;
}

/// In-memory storage for testing
pub struct MemoryStorage {
    records: std::sync::Arc<tokio::sync::Mutex<Vec<SourceRecord>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            records: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn get_records(&self) -> Vec<SourceRecord> {
        self.records.lock().await.clone()
    }
}

#[async_trait]
impl StorageHelper for MemoryStorage {
    async fn store_batch(&self, records: Vec<SourceRecord>) -> Result<usize> {
        let count = records.len();
        let mut storage = self.records.lock().await;
        storage.extend(records);
        Ok(count)
    }

    async fn store_record(&self, record: SourceRecord) -> Result<()> {
        let mut storage = self.records.lock().await;
        storage.push(record);
        Ok(())
    }

    async fn record_exists(&self, source: &str, id: &str) -> Result<bool> {
        let storage = self.records.lock().await;
        Ok(storage.iter().any(|r| r.source == source && r.id == id))
    }

    async fn get_last_sync(&self, source: &str) -> Result<Option<DateTime<Utc>>> {
        let storage = self.records.lock().await;
        let last = storage.iter()
            .filter(|r| r.source == source)
            .map(|r| r.timestamp)
            .max();
        Ok(last)
    }

    async fn update_last_sync(&self, _source: &str, _timestamp: DateTime<Utc>) -> Result<()> {
        // In-memory storage doesn't persist this
        Ok(())
    }
}

/// Helper for managing binary/large data
pub struct BinaryDataHelper;

impl BinaryDataHelper {
    /// Determine if data should be stored in object storage vs database
    pub fn should_use_object_storage(data: &Value) -> bool {
        // Estimate size - if over 1MB, use object storage
        let json_str = data.to_string();
        json_str.len() > 1_000_000
    }

    /// Generate object storage key for a record
    pub fn generate_object_key(source: &str, id: &str, timestamp: DateTime<Utc>) -> String {
        format!("{}/{}/{}/{}.json",
            source,
            timestamp.format("%Y/%m/%d"),
            timestamp.format("%H"),
            id
        )
    }
}