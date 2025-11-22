//! macOS source registration for the catalog

use crate::registry::{AuthType, RegisteredSource, SourceRegistry, RegisteredStream};
use serde_json::json;

/// macOS source registration
pub struct MacSource;

impl SourceRegistry for MacSource {
    fn descriptor() -> RegisteredSource {
        RegisteredSource {
            name: "mac",
            display_name: "macOS",
            description: "Personal data from macOS devices (App usage, Browser history, iMessage)",
            auth_type: AuthType::Device,
            oauth_config: None,
            icon: Some("ri:macbook-line"),
            streams: vec![
                // Apps stream
                RegisteredStream::new("apps")
                    .display_name("Application Usage")
                    .description("Active applications, window titles, and usage duration")
                    .table_name("stream_mac_apps")
                    .target_ontologies(vec!["activity_app_usage"])
                    .config_schema(apps_config_schema())
                    .config_example(apps_config_example())
                    .supports_incremental(false)
                    .supports_full_refresh(false)  // Push-based
                    .default_cron_schedule("0 */5 * * * *")  // Every 5 minutes (6-field: sec min hour day month dow)
                    .build(),

                // Browser stream
                RegisteredStream::new("browser")
                    .display_name("Browser History")
                    .description("URLs visited, page titles, and visit durations from Safari, Chrome, Firefox")
                    .table_name("stream_mac_browser")
                    .target_ontologies(vec!["activity_web_browsing"])
                    .config_schema(browser_config_schema())
                    .config_example(browser_config_example())
                    .supports_incremental(false)
                    .supports_full_refresh(false)  // Push-based
                    .default_cron_schedule("0 */5 * * * *")  // Every 5 minutes (6-field: sec min hour day month dow)
                    .build(),

                // iMessage stream
                RegisteredStream::new("imessage")
                    .display_name("iMessage")
                    .description("Message history including SMS and iMessage conversations")
                    .table_name("stream_mac_imessage")
                    .target_ontologies(vec!["social_message"])
                    .config_schema(imessage_config_schema())
                    .config_example(imessage_config_example())
                    .supports_incremental(false)
                    .supports_full_refresh(false)  // Push-based
                    .default_cron_schedule("0 */5 * * * *")  // Every 5 minutes (6-field: sec min hour day month dow)
                    .build(),
            ],
        }
    }
}

/// JSON schema for App usage configuration
fn apps_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "track_window_titles": {
                "type": "boolean",
                "default": true,
                "description": "Include window titles (may contain sensitive information)"
            },
            "sampling_interval_seconds": {
                "type": "integer",
                "default": 60,
                "minimum": 10,
                "description": "How often to sample active applications"
            },
            "excluded_apps": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of app bundle IDs to exclude from tracking"
            }
        }
    })
}

fn apps_config_example() -> serde_json::Value {
    json!({
        "track_window_titles": true,
        "sampling_interval_seconds": 60,
        "excluded_apps": ["com.apple.Keychain"]
    })
}

/// JSON schema for Browser history configuration
fn browser_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "browsers": {
                "type": "array",
                "items": {
                    "type": "string",
                    "enum": ["safari", "chrome", "firefox", "edge"]
                },
                "default": ["safari", "chrome"],
                "description": "Which browsers to track"
            },
            "exclude_domains": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Domains to exclude from tracking (e.g., banking sites)"
            },
            "track_incognito": {
                "type": "boolean",
                "default": false,
                "description": "Track private/incognito browsing sessions"
            }
        }
    })
}

fn browser_config_example() -> serde_json::Value {
    json!({
        "browsers": ["safari", "chrome"],
        "exclude_domains": ["bank.com", "private-site.com"],
        "track_incognito": false
    })
}

/// JSON schema for iMessage configuration
fn imessage_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "include_sms": {
                "type": "boolean",
                "default": true,
                "description": "Include SMS messages (requires phone)"
            },
            "include_attachments": {
                "type": "boolean",
                "default": false,
                "description": "Track attachment metadata"
            },
            "excluded_contacts": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Phone numbers or emails to exclude"
            }
        }
    })
}

fn imessage_config_example() -> serde_json::Value {
    json!({
        "include_sms": true,
        "include_attachments": false,
        "excluded_contacts": []
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_descriptor() {
        let desc = MacSource::descriptor();
        assert_eq!(desc.name, "mac");
        assert_eq!(desc.auth_type, AuthType::Device);
        assert_eq!(desc.streams.len(), 3);
    }

    #[test]
    fn test_apps_stream() {
        let desc = MacSource::descriptor();
        let apps = desc
            .streams
            .iter()
            .find(|s| s.name == "apps")
            .expect("Apps stream not found");

        assert_eq!(apps.display_name, "Application Usage");
        assert_eq!(apps.table_name, "stream_mac_apps");
        assert!(!apps.supports_incremental);
    }

    #[test]
    fn test_browser_stream() {
        let desc = MacSource::descriptor();
        let browser = desc
            .streams
            .iter()
            .find(|s| s.name == "browser")
            .expect("Browser stream not found");

        assert_eq!(browser.display_name, "Browser History");
        assert_eq!(browser.table_name, "stream_mac_browser");
    }

    #[test]
    fn test_imessage_stream() {
        let desc = MacSource::descriptor();
        let imessage = desc
            .streams
            .iter()
            .find(|s| s.name == "imessage")
            .expect("iMessage stream not found");

        assert_eq!(imessage.display_name, "iMessage");
        assert_eq!(imessage.table_name, "stream_mac_imessage");
    }
}
