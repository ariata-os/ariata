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
    pub last_sync_at: Option<DateTime<Utc>>,
    pub enabled_streams_count: i64,
    pub total_streams_count: i64,
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
