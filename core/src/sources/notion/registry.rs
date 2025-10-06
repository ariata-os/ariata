//! Notion source registration for the catalog

use crate::registry::{AuthType, OAuthConfig, SourceDescriptor, SourceRegistry, StreamDescriptor};
use serde_json::json;

/// Notion source registration
pub struct NotionSource;

impl SourceRegistry for NotionSource {
    fn descriptor() -> SourceDescriptor {
        SourceDescriptor {
            name: "notion",
            display_name: "Notion",
            description: "Sync pages, databases, and blocks from Notion workspaces",
            auth_type: AuthType::OAuth2,
            oauth_config: Some(OAuthConfig {
                scopes: vec![
                    "read_content",
                ],
                auth_url: "https://api.notion.com/v1/oauth/authorize",
                token_url: "https://api.notion.com/v1/oauth/token",
            }),
            streams: vec![
                // Pages stream
                StreamDescriptor::new("pages")
                    .display_name("Notion Pages")
                    .description("Sync pages and their content from Notion databases and workspaces")
                    .table_name("stream_notion_pages")
                    .config_schema(pages_config_schema())
                    .config_example(pages_config_example())
                    .supports_incremental(true)
                    .supports_full_refresh(true)
                    .build(),
            ],
        }
    }
}

/// JSON schema for Notion pages configuration
fn pages_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "database_ids": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of database IDs to sync (leave empty to sync all accessible pages)"
            },
            "include_archived": {
                "type": "boolean",
                "default": false,
                "description": "Include archived pages"
            },
            "sync_page_content": {
                "type": "boolean",
                "default": true,
                "description": "Sync full page content (blocks and text)"
            },
            "sync_properties": {
                "type": "boolean",
                "default": true,
                "description": "Sync page properties and metadata"
            },
            "max_pages_per_sync": {
                "type": "integer",
                "default": 100,
                "minimum": 1,
                "maximum": 500,
                "description": "Maximum number of pages to fetch per sync"
            }
        }
    })
}

/// Example configuration for Notion pages
fn pages_config_example() -> serde_json::Value {
    json!({
        "database_ids": [],
        "include_archived": false,
        "sync_page_content": true,
        "sync_properties": true,
        "max_pages_per_sync": 100
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notion_descriptor() {
        let desc = NotionSource::descriptor();
        assert_eq!(desc.name, "notion");
        assert_eq!(desc.auth_type, AuthType::OAuth2);
        assert!(desc.oauth_config.is_some());
        assert_eq!(desc.streams.len(), 1);
    }

    #[test]
    fn test_pages_stream() {
        let desc = NotionSource::descriptor();
        let pages = desc.streams.iter().find(|s| s.name == "pages");
        assert!(pages.is_some());

        let p = pages.unwrap();
        assert_eq!(p.table_name, "stream_notion_pages");
        assert!(p.supports_incremental);
        assert!(p.supports_full_refresh);
    }
}
