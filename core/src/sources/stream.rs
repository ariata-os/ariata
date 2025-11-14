//! Stream trait - unified interface for all data streams
//!
//! This module defines the core trait that all stream implementations must implement.
//! A stream represents a single time-series data table (e.g., Google Calendar, iOS HealthKit)
//! and provides sync functionality.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::base::{SyncMode, SyncResult};
use crate::error::Result;

/// Core trait for all data streams
///
/// A stream represents a single time-series data table that can be synced.
/// Each source (auth boundary) can have multiple streams.
///
/// # Example
///
/// ```rust
/// // Google source has two streams:
/// let calendar_stream: Box<dyn Stream> = ...;
/// let gmail_stream: Box<dyn Stream> = ...;
///
/// // Sync the calendar
/// let result = calendar_stream.sync(SyncMode::Incremental { cursor: None }).await?;
/// ```
#[async_trait]
pub trait Stream: Send + Sync {
    /// Sync this stream with the given mode
    ///
    /// This is the main operation that fetches data from the provider
    /// and writes it to the database.
    ///
    /// # Arguments
    /// * `mode` - Sync mode (Full Refresh or Incremental)
    ///
    /// # Returns
    /// A SyncResult containing statistics about the sync operation
    async fn sync(&self, mode: SyncMode) -> Result<SyncResult>;

    /// Get the stream's database table name
    ///
    /// Must follow the pattern: `stream_{source}_{stream}`
    /// Example: "stream_google_calendar", "stream_ios_healthkit"
    fn table_name(&self) -> &str;

    /// Get the stream's identifier
    ///
    /// This is the short name used in the registry and streams table.
    /// Example: "calendar", "gmail", "healthkit"
    fn stream_name(&self) -> &str;

    /// Get the source identifier
    ///
    /// Example: "google", "strava", "ios"
    fn source_name(&self) -> &str;

    /// Load stream-specific configuration from the streams table
    ///
    /// This is called by the StreamFactory before sync() to load
    /// any stream-specific config (e.g., which calendars to sync).
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `source_id` - UUID of the source this stream belongs to
    async fn load_config(&mut self, db: &PgPool, source_id: Uuid) -> Result<()>;

    /// Check if this stream supports incremental sync
    fn supports_incremental(&self) -> bool {
        true
    }

    /// Check if this stream supports full refresh
    fn supports_full_refresh(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock stream for testing
    struct MockStream {
        stream_name: String,
        source_name: String,
        table_name: String,
    }

    #[async_trait]
    impl Stream for MockStream {
        async fn sync(&self, _mode: SyncMode) -> Result<SyncResult> {
            Ok(SyncResult::new(chrono::Utc::now()))
        }

        fn table_name(&self) -> &str {
            &self.table_name
        }

        fn stream_name(&self) -> &str {
            &self.stream_name
        }

        fn source_name(&self) -> &str {
            &self.source_name
        }

        async fn load_config(&mut self, _db: &PgPool, _source_id: Uuid) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_stream() {
        let stream = MockStream {
            stream_name: "calendar".to_string(),
            source_name: "google".to_string(),
            table_name: "stream_google_calendar".to_string(),
        };

        assert_eq!(stream.stream_name(), "calendar");
        assert_eq!(stream.source_name(), "google");
        assert_eq!(stream.table_name(), "stream_google_calendar");
        assert!(stream.supports_incremental());
        assert!(stream.supports_full_refresh());
    }
}
