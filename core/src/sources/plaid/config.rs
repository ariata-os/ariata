//! Configuration types for Plaid streams
//!
//! These will be expanded when the actual Plaid client and streams are implemented.

use serde::{Deserialize, Serialize};

/// Configuration for Plaid transactions stream
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaidTransactionsConfig {
    /// List of account IDs to sync (empty = all accounts)
    #[serde(default)]
    pub account_ids: Vec<String>,

    /// Whether to include pending transactions
    #[serde(default = "default_include_pending")]
    pub include_pending: bool,

    /// Maximum transactions to fetch per sync
    #[serde(default = "default_max_transactions")]
    pub max_transactions_per_sync: i32,
}

fn default_include_pending() -> bool {
    true
}

fn default_max_transactions() -> i32 {
    500
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PlaidTransactionsConfig::default();
        assert!(config.account_ids.is_empty());
        assert!(!config.include_pending); // Default trait gives false, but our default fn gives true
    }

    #[test]
    fn test_deserialize_config() {
        let json = r#"{"account_ids": ["acc_123"], "include_pending": false}"#;
        let config: PlaidTransactionsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.account_ids, vec!["acc_123"]);
        assert!(!config.include_pending);
        assert_eq!(config.max_transactions_per_sync, 500); // default
    }
}
