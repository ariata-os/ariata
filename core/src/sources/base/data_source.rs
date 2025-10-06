//! Core trait for all data sources (OAuth-based and device-based)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::Result;
use super::{SyncMode, SyncResult};

/// Health status for a data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub message: String,
    pub last_check: DateTime<Utc>,
    pub last_successful_sync: Option<DateTime<Utc>>,
    pub error_count: u32,
}

/// Core trait that all data sources must implement
#[async_trait]
pub trait DataSource: Send + Sync {
    // ===== Identity =====

    /// Get the source ID (UUID from sources table)
    fn source_id(&self) -> Uuid;

    /// Get the source type (e.g., "google", "strava", "notion", "ios", "mac")
    fn source_type(&self) -> &str;

    /// Get the stream name (e.g., "calendar", "gmail", "activities")
    /// Combined with source_type to form table name: stream_{source_type}_{stream_name}
    fn stream_name(&self) -> &str;

    /// Get the fully qualified stream table name
    fn stream_table_name(&self) -> String {
        format!("stream_{}_{}", self.source_type(), self.stream_name())
    }

    // ===== Configuration =====

    /// Validate configuration JSON before saving
    /// Called when creating/updating source configuration
    fn validate_config(config: &Value) -> Result<()>
    where
        Self: Sized;

    /// Get required OAuth scopes (empty for device sources)
    fn required_scopes(&self) -> Vec<String> {
        Vec::new()
    }

    // ===== Sync Interface =====

    /// Execute a sync operation
    /// Implementation should:
    /// - Fetch data from source API
    /// - Transform to stream schema
    /// - Write to database
    /// - Update sync cursor
    /// - Return SyncResult with metrics
    async fn sync(&self, mode: &SyncMode) -> Result<SyncResult>;

    /// Perform a health check on the source
    /// Should verify:
    /// - Authentication is valid (for OAuth sources)
    /// - API is reachable
    /// - No critical errors
    async fn health_check(&self) -> Result<HealthStatus>;

    // ===== Scheduling =====

    /// Get the cron schedule for this source (default: every 5 minutes)
    /// Format: "seconds minutes hours day_of_month month day_of_week"
    /// Example: "0 */5 * * * *" = every 5 minutes
    fn sync_schedule(&self) -> String {
        "0 */5 * * * *".to_string()
    }

    /// Check if sync is needed (default: always sync on schedule)
    /// Override to implement custom logic (e.g., only sync during business hours)
    async fn should_sync(&self) -> bool {
        true
    }
}

/// Helper to parse source_type and stream_name from a stream table name
/// Example: "stream_google_calendar" -> Some(("google", "calendar"))
pub fn parse_stream_table_name(table_name: &str) -> Option<(String, String)> {
    let stripped = table_name.strip_prefix("stream_")?;
    let parts: Vec<&str> = stripped.splitn(2, '_').collect();

    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stream_table_name() {
        assert_eq!(
            parse_stream_table_name("stream_google_calendar"),
            Some(("google".to_string(), "calendar".to_string()))
        );

        assert_eq!(
            parse_stream_table_name("stream_ios_healthkit"),
            Some(("ios".to_string(), "healthkit".to_string()))
        );

        assert_eq!(
            parse_stream_table_name("stream_google_gmail"),
            Some(("google".to_string(), "gmail".to_string()))
        );

        // Invalid formats
        assert_eq!(parse_stream_table_name("google_calendar"), None);
        assert_eq!(parse_stream_table_name("stream_google"), None);
    }
}
