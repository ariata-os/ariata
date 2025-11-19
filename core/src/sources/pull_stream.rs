use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::Result;

// Re-export existing SyncMode and SyncResult from base
pub use super::base::{SyncMode, SyncResult};

/// Trait for sources where the backend initiates synchronization
/// by pulling data from an external API (e.g., Google Calendar, Notion).
///
/// Pull streams are characterized by:
/// - Backend controls when sync happens (via scheduler)
/// - Data lives in remote APIs that backend can access
/// - Uses OAuth tokens or API keys managed by backend
/// - Supports incremental sync with cursors/tokens
///
/// # Examples
///
/// ```ignore
/// impl PullStream for GoogleCalendarStream {
///     async fn sync_pull(&self, mode: SyncMode) -> Result<SyncResult> {
///         // 1. Get OAuth token from TokenManager
///         // 2. Fetch events from Google Calendar API
///         // 3. Write records to StreamWriter
///         // 4. Save sync cursor for next run
///         // 5. Return stats
///     }
/// }
/// ```
#[async_trait]
pub trait PullStream: Send + Sync {
    /// Backend initiates: actively fetch data from the external source
    ///
    /// This method should:
    /// 1. Connect to the external API using stored credentials
    /// 2. Fetch records (incrementally or full refresh based on mode)
    /// 3. Write records to StreamWriter
    /// 4. Update sync cursor/token in database
    /// 5. Return statistics about what was synced
    async fn sync_pull(&self, mode: SyncMode) -> Result<SyncResult>;

    /// Load OAuth tokens, API keys, or other config from database
    ///
    /// Called before sync_pull() to ensure stream has necessary credentials
    async fn load_config(&mut self, db: &PgPool, source_id: Uuid) -> Result<()>;

    /// Table name in data schema (e.g., "stream_google_calendar")
    fn table_name(&self) -> &str;

    /// Stream identifier (e.g., "calendar", "gmail")
    fn stream_name(&self) -> &str;

    /// Provider identifier (e.g., "google", "notion")
    fn source_name(&self) -> &str;

    /// Whether this stream supports incremental sync with cursors
    ///
    /// If true, the stream can use SyncMode::Incremental to sync only
    /// new/updated records since last sync.
    fn supports_incremental(&self) -> bool {
        true
    }

    /// Whether this stream supports full refresh (re-sync all data)
    ///
    /// If true, the stream can use SyncMode::FullRefresh to re-sync
    /// all records from scratch, ignoring any sync cursors.
    fn supports_full_refresh(&self) -> bool {
        true
    }
}
