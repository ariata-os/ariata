//! Configuration for Google sources

use serde::{Deserialize, Serialize};

use crate::sources::base::SyncStrategy;

/// Configuration for Google Calendar sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleCalendarConfig {
    /// Calendar IDs to sync (default: ["primary"])
    #[serde(default = "default_calendar_ids")]
    pub calendar_ids: Vec<String>,

    /// Strategy for full sync operations (default: 365 days lookback)
    #[serde(default)]
    pub sync_strategy: SyncStrategy,

    /// Include events where the user declined (default: false)
    #[serde(default)]
    pub include_declined: bool,

    /// Include cancelled events (default: false)
    #[serde(default)]
    pub include_cancelled: bool,

    /// Maximum number of events per sync batch (default: 500)
    #[serde(default = "default_max_events")]
    pub max_events_per_sync: u32,
}

impl Default for GoogleCalendarConfig {
    fn default() -> Self {
        Self {
            calendar_ids: default_calendar_ids(),
            sync_strategy: SyncStrategy::default(),
            include_declined: false,
            include_cancelled: false,
            max_events_per_sync: default_max_events(),
        }
    }
}

impl GoogleCalendarConfig {
    /// Create a config from JSON value (from database)
    pub fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// Convert to JSON value (for database storage)
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// Calculate time bounds based on sync strategy
    pub fn calculate_time_bounds(&self) -> (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>) {
        self.sync_strategy.calculate_time_bounds()
    }
}

fn default_calendar_ids() -> Vec<String> {
    vec!["primary".to_string()]
}

fn default_max_events() -> u32 {
    500
}

/// Sync mode for Gmail
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GmailSyncMode {
    Messages,  // Sync individual messages
    Threads,   // Sync conversation threads
}

impl Default for GmailSyncMode {
    fn default() -> Self {
        Self::Messages
    }
}

/// Configuration for Google Gmail sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleGmailConfig {
    /// Label IDs to sync (default: [] which syncs all mail)
    /// Leave empty to sync all mail, or specify labels like ["INBOX", "SENT"] to filter
    #[serde(default = "default_label_ids")]
    pub label_ids: Vec<String>,

    /// Include spam and trash folders (default: false)
    #[serde(default)]
    pub include_spam_trash: bool,

    /// Sync mode: messages or threads (default: messages)
    #[serde(default)]
    pub sync_mode: GmailSyncMode,

    /// Fetch full message body (default: true)
    #[serde(default = "default_fetch_body")]
    pub fetch_body: bool,

    /// Strategy for full sync operations (default: 365 days lookback)
    #[serde(default)]
    pub sync_strategy: SyncStrategy,

    /// Maximum number of messages per sync batch (default: 500)
    #[serde(default = "default_max_messages")]
    pub max_messages_per_sync: u32,

    /// Query filter for messages (Gmail search syntax)
    pub query: Option<String>,
}

impl Default for GoogleGmailConfig {
    fn default() -> Self {
        Self {
            label_ids: default_label_ids(),
            include_spam_trash: false,
            sync_mode: GmailSyncMode::default(),
            fetch_body: default_fetch_body(),
            sync_strategy: SyncStrategy::default(),
            max_messages_per_sync: default_max_messages(),
            query: None,
        }
    }
}

impl GoogleGmailConfig {
    /// Create a config from JSON value (from database)
    pub fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// Convert to JSON value (for database storage)
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// Build the query string for Gmail API based on sync strategy
    pub fn build_query(&self) -> String {
        let mut parts = Vec::new();

        // Add time bounds from strategy
        let (min_time, max_time) = self.sync_strategy.calculate_time_bounds();

        if let Some(min) = min_time {
            let after = min.format("%Y/%m/%d").to_string();
            parts.push(format!("after:{after}"));
        }

        if let Some(max) = max_time {
            let before = max.format("%Y/%m/%d").to_string();
            parts.push(format!("before:{before}"));
        }

        // Add custom query if provided
        if let Some(ref q) = self.query {
            parts.push(q.clone());
        }

        parts.join(" ")
    }
}

fn default_label_ids() -> Vec<String> {
    // Empty vec means no label filter - sync all mail within time window
    vec![]
}

fn default_fetch_body() -> bool {
    true
}

fn default_max_messages() -> u32 {
    500
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_default_config() {
        let config = GoogleCalendarConfig::default();
        assert_eq!(config.calendar_ids, vec!["primary"]);
        assert!(!config.include_declined);
        assert_eq!(config.max_events_per_sync, 500);
    }

    #[test]
    fn test_time_bounds_calculation() {
        let config = GoogleCalendarConfig {
            sync_strategy: SyncStrategy::TimeWindow { days_back: 30 },
            ..Default::default()
        };

        let (min_time, max_time) = config.calculate_time_bounds();
        assert!(min_time.is_some());
        assert!(max_time.is_some());

        let now = Utc::now();
        let min = min_time.unwrap();
        let max = max_time.unwrap();

        // Should be approximately 30 days in the past
        let diff = now - min;
        assert!(diff.num_days() >= 29 && diff.num_days() <= 31);

        // Max should be approximately now
        let diff = max - now;
        assert!(diff.num_seconds().abs() < 60);
    }

    #[test]
    fn test_json_serialization() {
        let config = GoogleCalendarConfig::default();
        let json = config.to_json();
        let deserialized = GoogleCalendarConfig::from_json(&json).unwrap();

        assert_eq!(config.calendar_ids, deserialized.calendar_ids);
    }

    #[test]
    fn test_gmail_default_config() {
        let config = GoogleGmailConfig::default();
        assert_eq!(config.label_ids, Vec::<String>::new()); // Empty = sync all mail
        assert!(!config.include_spam_trash);
        assert!(config.fetch_body);
        assert_eq!(config.max_messages_per_sync, 500);
    }

    #[test]
    fn test_gmail_query_builder() {
        let config = GoogleGmailConfig {
            sync_strategy: SyncStrategy::TimeWindow { days_back: 7 },
            query: Some("has:attachment".to_string()),
            ..Default::default()
        };

        let query = config.build_query();
        assert!(query.contains("after:"));
        assert!(query.contains("has:attachment"));
    }

    #[test]
    fn test_gmail_json_serialization() {
        let config = GoogleGmailConfig::default();
        let json = config.to_json();
        let deserialized = GoogleGmailConfig::from_json(&json).unwrap();

        assert_eq!(config.label_ids, deserialized.label_ids);
        assert_eq!(config.fetch_body, deserialized.fetch_body);
    }
}