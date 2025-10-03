//! Source implementations for various data providers

pub mod base;
pub mod google;
pub mod strava;
pub mod notion;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::error::Result;

/// Base trait for all data sources
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Name of the source
    fn name(&self) -> &str;

    /// Whether this source requires OAuth
    fn requires_oauth(&self) -> bool {
        false
    }

    /// Fetch data from the source
    async fn fetch(&self, since: Option<DateTime<Utc>>) -> Result<Vec<SourceRecord>>;

    /// Get the current sync state
    async fn get_sync_state(&self) -> Result<SyncState>;

    /// Update sync state after successful sync
    async fn update_sync_state(&self, state: SyncState) -> Result<()>;
}

/// Generic record from a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

/// Sync state for incremental syncs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub source: String,
    pub last_sync: Option<DateTime<Utc>>,
    pub sync_token: Option<String>,
    pub cursor: Option<String>,
    pub checkpoint: Option<serde_json::Value>,
}