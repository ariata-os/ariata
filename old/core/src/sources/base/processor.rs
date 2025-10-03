//! Base processor traits and utilities for data transformation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::{
    error::Result,
    sources::SourceRecord,
};

/// Base trait for processing source data
#[async_trait]
pub trait BaseProcessor: Send + Sync {
    /// Process raw API response into source records
    async fn process_response(&self, response: Value) -> Result<Vec<SourceRecord>>;

    /// Transform a single item into a source record
    fn transform_item(&self, item: Value) -> Result<SourceRecord>;

    /// Validate a record before storage
    fn validate_record(&self, record: &SourceRecord) -> Result<()> {
        if record.id.is_empty() {
            return Err(crate::error::Error::Other("Record ID cannot be empty".to_string()));
        }
        if record.source.is_empty() {
            return Err(crate::error::Error::Other("Source name cannot be empty".to_string()));
        }
        Ok(())
    }

    /// Extract timestamp from raw data
    fn extract_timestamp(&self, data: &Value) -> DateTime<Utc> {
        // Try common timestamp field names
        for field in &["updated", "updated_at", "modified", "timestamp", "created_at", "created"] {
            if let Some(ts_str) = data.get(field).and_then(|v| v.as_str()) {
                if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                    return ts.with_timezone(&Utc);
                }
            }
        }
        // Default to current time if no timestamp found
        Utc::now()
    }

    /// Extract ID from raw data
    fn extract_id(&self, data: &Value) -> Option<String> {
        // Try common ID field names
        for field in &["id", "_id", "uid", "uuid"] {
            if let Some(id) = data.get(field) {
                return Some(match id {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => continue,
                });
            }
        }
        None
    }

    /// Batch process multiple items
    async fn batch_process(&self, items: Vec<Value>) -> Result<Vec<SourceRecord>> {
        let mut records = Vec::with_capacity(items.len());

        for item in items {
            match self.transform_item(item) {
                Ok(record) => {
                    if self.validate_record(&record).is_ok() {
                        records.push(record);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to process item: {}", e);
                }
            }
        }

        Ok(records)
    }
}

/// Helper for incremental sync pagination
pub struct PaginationHelper {
    pub page_size: usize,
    pub current_page: usize,
    pub has_more: bool,
}

impl PaginationHelper {
    pub fn new(page_size: usize) -> Self {
        Self {
            page_size,
            current_page: 0,
            has_more: true,
        }
    }

    pub fn next_page(&mut self) {
        self.current_page += 1;
    }

    pub fn offset(&self) -> usize {
        self.current_page * self.page_size
    }
}