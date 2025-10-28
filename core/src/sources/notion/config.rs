//! Configuration for Notion sources

use serde::{Deserialize, Serialize};

/// Configuration for Notion Pages sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionPagesConfig {
    /// Maximum number of pages per sync batch (default: 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,

    /// Include archived pages (default: false)
    #[serde(default)]
    pub include_archived: bool,

    /// Filter by specific database IDs (empty = all pages)
    #[serde(default)]
    pub database_ids: Vec<String>,

    /// Number of days for initial sync lookback (default: 365)
    #[serde(default = "default_initial_sync_window_days")]
    pub initial_sync_window_days: u32,
}

impl Default for NotionPagesConfig {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
            include_archived: false,
            database_ids: vec![],
            initial_sync_window_days: default_initial_sync_window_days(),
        }
    }
}

impl NotionPagesConfig {
    /// Create a config from JSON value (from database)
    pub fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// Convert to JSON value (for database storage)
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }
}

fn default_page_size() -> u32 {
    100
}

fn default_initial_sync_window_days() -> u32 {
    365
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NotionPagesConfig::default();
        assert_eq!(config.page_size, 100);
        assert!(!config.include_archived);
        assert_eq!(config.database_ids.len(), 0);
        assert_eq!(config.initial_sync_window_days, 365);
    }

    #[test]
    fn test_json_serialization() {
        let config = NotionPagesConfig::default();
        let json = config.to_json();
        let deserialized = NotionPagesConfig::from_json(&json).unwrap();

        assert_eq!(config.page_size, deserialized.page_size);
        assert_eq!(config.include_archived, deserialized.include_archived);
        assert_eq!(config.initial_sync_window_days, deserialized.initial_sync_window_days);
    }
}
