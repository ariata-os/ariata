//! Plaid source registration for the catalog

use crate::registry::{AuthType, OAuthConfig, RegisteredSource, RegisteredStream, SourceRegistry};
use serde_json::json;

/// Plaid source registration
pub struct PlaidSource;

impl SourceRegistry for PlaidSource {
    fn descriptor() -> RegisteredSource {
        RegisteredSource {
            name: "plaid",
            display_name: "Plaid",
            description: "Connect your bank accounts and credit cards to sync transactions and balances",
            auth_type: AuthType::OAuth2,
            oauth_config: Some(OAuthConfig {
                scopes: vec!["transactions", "auth"],
                auth_url: "https://cdn.plaid.com/link/v2/stable/link.html",
                token_url: "https://production.plaid.com/link/token/exchange",
            }),
            icon: Some("simple-icons:plaid"),
            streams: vec![
                // Transactions stream
                RegisteredStream::new("transactions")
                    .display_name("Transactions")
                    .description(
                        "Sync bank transactions, account balances, and financial data",
                    )
                    .table_name("stream_plaid_transactions")
                    .target_ontologies(vec![]) // Transform not yet implemented
                    .config_schema(transactions_config_schema())
                    .config_example(transactions_config_example())
                    .supports_incremental(true)
                    .supports_full_refresh(true)
                    .default_cron_schedule("0 0 */6 * * *") // Every 6 hours (6-field: sec min hour day month dow)
                    .build(),
            ],
        }
    }
}

/// JSON schema for Plaid transactions configuration
fn transactions_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "account_ids": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of account IDs to sync (leave empty to sync all connected accounts)"
            },
            "include_pending": {
                "type": "boolean",
                "default": true,
                "description": "Include pending transactions"
            },
            "sync_strategy": {
                "type": "object",
                "description": "Strategy for sync operations",
                "default": {
                    "type": "time_window",
                    "days_back": 90
                },
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["type"],
                        "properties": {
                            "type": { "const": "time_window" },
                            "days_back": {
                                "type": "integer",
                                "default": 90,
                                "description": "Number of days of history to sync"
                            }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type"],
                        "properties": {
                            "type": { "const": "full_history" },
                            "max_records": {
                                "type": "integer",
                                "nullable": true,
                                "description": "Optional limit on number of transactions"
                            }
                        }
                    }
                ]
            },
            "max_transactions_per_sync": {
                "type": "integer",
                "default": 500,
                "minimum": 1,
                "maximum": 500,
                "description": "Maximum number of transactions to fetch per sync (Plaid limit is 500)"
            }
        }
    })
}

/// Example configuration for Plaid transactions
fn transactions_config_example() -> serde_json::Value {
    json!({
        "account_ids": [],
        "include_pending": true,
        "sync_strategy": {
            "type": "time_window",
            "days_back": 90
        },
        "max_transactions_per_sync": 500
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plaid_descriptor() {
        let desc = PlaidSource::descriptor();
        assert_eq!(desc.name, "plaid");
        assert_eq!(desc.auth_type, AuthType::OAuth2);
        assert!(desc.oauth_config.is_some());
        assert_eq!(desc.streams.len(), 1);
    }

    #[test]
    fn test_transactions_stream() {
        let desc = PlaidSource::descriptor();
        let transactions = desc.streams.iter().find(|s| s.name == "transactions");
        assert!(transactions.is_some());

        let tx = transactions.unwrap();
        assert_eq!(tx.table_name, "stream_plaid_transactions");
        assert!(tx.supports_incremental);
        assert!(tx.supports_full_refresh);
    }

    #[test]
    fn test_config_schemas_valid() {
        // Ensure schemas are valid JSON
        let tx_schema = transactions_config_schema();
        assert_eq!(tx_schema["type"], "object");
        assert!(tx_schema["properties"].is_object());
    }
}
