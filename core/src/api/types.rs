//! Shared types used across the API

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents a configured data source
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Source {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub source_type: String,
    pub name: String,
    pub is_active: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Source status with sync statistics
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SourceStatus {
    pub id: Uuid,
    pub name: String,
    pub source_type: String,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub total_syncs: i64,
    pub successful_syncs: i64,
    pub failed_syncs: i64,
    pub last_sync_status: Option<String>,
    pub last_sync_duration_ms: Option<i32>,
}

/// Sync log entry
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SyncLog {
    pub id: Uuid,
    pub source_id: Uuid,
    pub sync_mode: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub status: String,
    pub records_fetched: Option<i32>,
    pub records_written: Option<i32>,
    pub records_failed: Option<i32>,
    pub error_message: Option<String>,
}
