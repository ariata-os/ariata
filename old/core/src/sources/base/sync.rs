//! Sync state management utilities

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::{
    error::Result,
    sources::SyncState,
};

/// Manager for source sync state
#[async_trait]
pub trait SyncStateManager: Send + Sync {
    /// Get sync state for a source
    async fn get_state(&self, source: &str) -> Result<SyncState>;

    /// Update sync state
    async fn update_state(&self, state: SyncState) -> Result<()>;

    /// Reset sync state (for full resync)
    async fn reset_state(&self, source: &str) -> Result<()>;

    /// Check if incremental sync is supported
    fn supports_incremental(&self) -> bool {
        true
    }
}

/// In-memory sync state manager for testing
pub struct MemorySyncStateManager {
    states: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, SyncState>>>,
}

impl MemorySyncStateManager {
    pub fn new() -> Self {
        Self {
            states: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl SyncStateManager for MemorySyncStateManager {
    async fn get_state(&self, source: &str) -> Result<SyncState> {
        let states = self.states.lock().await;
        Ok(states.get(source).cloned().unwrap_or_else(|| SyncState {
            source: source.to_string(),
            last_sync: None,
            sync_token: None,
            cursor: None,
            checkpoint: None,
        }))
    }

    async fn update_state(&self, state: SyncState) -> Result<()> {
        let mut states = self.states.lock().await;
        states.insert(state.source.clone(), state);
        Ok(())
    }

    async fn reset_state(&self, source: &str) -> Result<()> {
        let mut states = self.states.lock().await;
        states.remove(source);
        Ok(())
    }
}

/// Helper for managing sync tokens and incremental sync
pub struct IncrementalSyncHelper;

impl IncrementalSyncHelper {
    /// Check if we should do a full sync based on last sync time
    pub fn should_full_sync(last_sync: Option<DateTime<Utc>>, max_age_days: i64) -> bool {
        match last_sync {
            None => true,
            Some(last) => {
                let age = Utc::now().signed_duration_since(last);
                age.num_days() > max_age_days
            }
        }
    }

    /// Handle sync token expiration
    pub async fn handle_token_expiration<F, T>(
        state: &SyncState,
        mut fetch_fn: F,
    ) -> Result<(T, Option<String>)>
    where
        F: FnMut(Option<String>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(T, Option<String>)>> + Send>>,
    {
        // First try with existing token
        match fetch_fn(state.sync_token.clone()).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if error is due to expired token
                if e.to_string().contains("token") || e.to_string().contains("expired") {
                    // Retry without token (full sync)
                    tracing::warn!("Sync token expired, performing full sync");
                    fetch_fn(None).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Create checkpoint for resumable sync
    pub fn create_checkpoint(
        page: usize,
        total_processed: usize,
        last_id: Option<String>,
    ) -> Value {
        serde_json::json!({
            "page": page,
            "total_processed": total_processed,
            "last_id": last_id,
            "checkpoint_time": Utc::now(),
        })
    }

    /// Resume from checkpoint
    pub fn resume_from_checkpoint(checkpoint: &Value) -> (usize, usize, Option<String>) {
        let page = checkpoint.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let total = checkpoint.get("total_processed").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let last_id = checkpoint.get("last_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        (page, total, last_id)
    }
}