//! Sync mode abstractions for ELT operations
//!
//! Defines standard sync strategies matching enterprise patterns from
//! Airbyte, Fivetran, and other modern ELT tools.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Sync strategies for data extraction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMode {
    /// Full refresh - replace all existing data
    FullRefresh,

    /// Incremental - fetch only new/changed records using cursor/token
    Incremental {
        /// Sync cursor/token from last successful sync
        cursor: Option<String>,
    },
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Incremental { cursor: None }
    }
}

impl SyncMode {
    /// Create a full refresh sync mode
    pub fn full_refresh() -> Self {
        Self::FullRefresh
    }

    /// Create an incremental sync mode with optional cursor
    pub fn incremental(cursor: Option<String>) -> Self {
        Self::Incremental { cursor }
    }

    /// Check if this is a full refresh
    pub fn is_full_refresh(&self) -> bool {
        matches!(self, Self::FullRefresh)
    }

    /// Get the cursor if this is incremental sync
    pub fn cursor(&self) -> Option<&str> {
        match self {
            Self::Incremental { cursor } => cursor.as_deref(),
            _ => None,
        }
    }
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of records fetched from source
    pub records_fetched: usize,

    /// Number of records successfully written
    pub records_written: usize,

    /// Number of records that failed validation/write
    pub records_failed: usize,

    /// New cursor for next incremental sync
    pub next_cursor: Option<String>,

    /// Timestamp when sync started
    pub started_at: DateTime<Utc>,

    /// Timestamp when sync completed
    pub completed_at: DateTime<Utc>,

    /// In-memory records for direct transform (hot path)
    /// Optional for backward compatibility. When present, transforms can use
    /// these records directly without reading from S3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<Value>>,

    /// Archive job ID for tracking async S3 archival
    /// Optional for backward compatibility. When present, indicates that
    /// S3 archival is happening asynchronously in the background.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_job_id: Option<Uuid>,
}

impl SyncResult {
    /// Create a new sync result
    pub fn new(started_at: DateTime<Utc>) -> Self {
        Self {
            records_fetched: 0,
            records_written: 0,
            records_failed: 0,
            next_cursor: None,
            started_at,
            completed_at: Utc::now(),
            records: None,
            archive_job_id: None,
        }
    }

    /// Calculate sync duration in milliseconds
    pub fn duration_ms(&self) -> i64 {
        self.completed_at
            .signed_duration_since(self.started_at)
            .num_milliseconds()
    }

    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.records_fetched == 0 {
            return 1.0;
        }
        self.records_written as f64 / self.records_fetched as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_mode_creation() {
        let full = SyncMode::full_refresh();
        assert!(full.is_full_refresh());
        assert_eq!(full.cursor(), None);

        let incremental = SyncMode::incremental(Some("token123".to_string()));
        assert!(!incremental.is_full_refresh());
        assert_eq!(incremental.cursor(), Some("token123"));
    }

    #[test]
    fn test_sync_mode_default() {
        let mode = SyncMode::default();
        assert!(!mode.is_full_refresh());
        assert_eq!(mode.cursor(), None);
    }

    #[test]
    fn test_sync_result_metrics() {
        let start = Utc::now();
        let mut result = SyncResult::new(start);

        result.records_fetched = 100;
        result.records_written = 95;
        result.records_failed = 5;

        assert_eq!(result.success_rate(), 0.95);
        assert!(result.duration_ms() >= 0);
    }

    #[test]
    fn test_sync_mode_serialization() {
        let mode = SyncMode::Incremental {
            cursor: Some("abc123".to_string()),
        };

        let json = serde_json::to_string(&mode).unwrap();
        let deserialized: SyncMode = serde_json::from_str(&json).unwrap();

        assert_eq!(mode, deserialized);
    }
}
