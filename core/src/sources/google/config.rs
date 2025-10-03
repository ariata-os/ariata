//! Configuration for Google sources

use serde::{Deserialize, Serialize};

/// Sync direction for calendar events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    Past,
    Future,
    Both,
}

impl Default for SyncDirection {
    fn default() -> Self {
        Self::Past
    }
}

/// Configuration for Google Calendar sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleCalendarConfig {
    /// Calendar IDs to sync (default: ["primary"])
    #[serde(default = "default_calendar_ids")]
    pub calendar_ids: Vec<String>,

    /// Number of days to sync (default: 90)
    #[serde(default = "default_sync_window_days")]
    pub sync_window_days: u32,

    /// Direction to sync events (past, future, or both)
    #[serde(default)]
    pub sync_direction: SyncDirection,

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
            sync_window_days: default_sync_window_days(),
            sync_direction: SyncDirection::default(),
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

    /// Calculate time bounds based on configuration
    pub fn calculate_time_bounds(&self) -> (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>) {
        use chrono::{Duration, Utc};
        let now = Utc::now();
        let window = Duration::days(self.sync_window_days as i64);

        match self.sync_direction {
            SyncDirection::Past => {
                (Some(now - window), Some(now))
            },
            SyncDirection::Future => {
                (Some(now), Some(now + window))
            },
            SyncDirection::Both => {
                (Some(now - window), Some(now + window))
            }
        }
    }
}

fn default_calendar_ids() -> Vec<String> {
    vec!["primary".to_string()]
}

fn default_sync_window_days() -> u32 {
    90
}

fn default_max_events() -> u32 {
    500
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_default_config() {
        let config = GoogleCalendarConfig::default();
        assert_eq!(config.calendar_ids, vec!["primary"]);
        assert_eq!(config.sync_window_days, 90);
        assert!(!config.include_declined);
        assert_eq!(config.max_events_per_sync, 500);
    }

    #[test]
    fn test_time_bounds_calculation() {
        let config = GoogleCalendarConfig {
            sync_window_days: 30,
            sync_direction: SyncDirection::Past,
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
        assert_eq!(config.sync_window_days, deserialized.sync_window_days);
    }
}