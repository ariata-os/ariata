//! Google source registration for the catalog

use crate::registry::{AuthType, OAuthConfig, SourceDescriptor, SourceRegistry, StreamDescriptor};
use serde_json::json;

/// Google source registration
pub struct GoogleSource;

impl SourceRegistry for GoogleSource {
    fn descriptor() -> SourceDescriptor {
        SourceDescriptor {
            name: "google",
            display_name: "Google",
            description: "Sync data from Google Workspace services (Calendar, Gmail, Drive)",
            auth_type: AuthType::OAuth2,
            oauth_config: Some(OAuthConfig {
                scopes: vec![
                    "https://www.googleapis.com/auth/calendar.readonly",
                    "https://www.googleapis.com/auth/gmail.readonly",
                ],
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
                token_url: "https://oauth2.googleapis.com/token",
            }),
            icon: Some("ri:google-fill"),
            streams: vec![
                // Calendar stream
                StreamDescriptor::new("calendar")
                    .display_name("Google Calendar")
                    .description("Sync calendar events with attendees, locations, and conference details")
                    .table_name("stream_google_calendar")
                    .config_schema(calendar_config_schema())
                    .config_example(calendar_config_example())
                    .supports_incremental(true)
                    .supports_full_refresh(true)
                    .default_cron_schedule("0 */6 * * *")  // Every 6 hours
                    .build(),

                // Gmail stream
                StreamDescriptor::new("gmail")
                    .display_name("Gmail")
                    .description("Sync email messages and threads with full metadata")
                    .table_name("stream_google_gmail")
                    .config_schema(gmail_config_schema())
                    .config_example(gmail_config_example())
                    .supports_incremental(true)
                    .supports_full_refresh(true)
                    .default_cron_schedule("*/15 * * * *")  // Every 15 minutes
                    .build(),
            ],
        }
    }
}

/// JSON schema for GoogleCalendarConfig
fn calendar_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "calendar_ids": {
                "type": "array",
                "items": { "type": "string" },
                "default": ["primary"],
                "description": "List of calendar IDs to sync (use 'primary' for main calendar)"
            },
            "sync_strategy": {
                "type": "object",
                "description": "Strategy for determining what data to sync during full refresh operations",
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["type", "days_back"],
                        "properties": {
                            "type": { "const": "time_window" },
                            "days_back": {
                                "type": "integer",
                                "default": 365,
                                "minimum": 1,
                                "maximum": 3650,
                                "description": "Number of days to look back from now"
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
                                "description": "Optional limit on number of records to prevent runaway syncs"
                            }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type", "start_date", "end_date"],
                        "properties": {
                            "type": { "const": "date_range" },
                            "start_date": {
                                "type": "string",
                                "format": "date-time",
                                "description": "Start of date range (ISO 8601)"
                            },
                            "end_date": {
                                "type": "string",
                                "format": "date-time",
                                "description": "End of date range (ISO 8601)"
                            }
                        }
                    }
                ]
            },
            "include_declined": {
                "type": "boolean",
                "default": false,
                "description": "Include events where you declined the invitation"
            },
            "include_cancelled": {
                "type": "boolean",
                "default": false,
                "description": "Include cancelled events"
            },
            "max_events_per_sync": {
                "type": "integer",
                "default": 500,
                "minimum": 1,
                "maximum": 2500,
                "description": "Maximum number of events to fetch per sync operation"
            }
        }
    })
}

/// Example configuration for Google Calendar
fn calendar_config_example() -> serde_json::Value {
    json!({
        "calendar_ids": ["primary", "work@company.com"],
        "sync_strategy": {
            "type": "time_window",
            "days_back": 365
        },
        "include_declined": false,
        "include_cancelled": false,
        "max_events_per_sync": 500
    })
}

/// JSON schema for GoogleGmailConfig
fn gmail_config_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "label_ids": {
                "type": "array",
                "items": { "type": "string" },
                "default": ["INBOX", "SENT"],
                "description": "Gmail labels to sync (e.g., INBOX, SENT, DRAFT)"
            },
            "include_spam_trash": {
                "type": "boolean",
                "default": false,
                "description": "Include spam and trash folders"
            },
            "sync_mode": {
                "type": "string",
                "enum": ["messages", "threads"],
                "default": "messages",
                "description": "Sync individual messages or conversation threads"
            },
            "fetch_body": {
                "type": "boolean",
                "default": true,
                "description": "Fetch full message body content"
            },
            "sync_strategy": {
                "type": "object",
                "description": "Strategy for determining what data to sync during full refresh operations",
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["type", "days_back"],
                        "properties": {
                            "type": { "const": "time_window" },
                            "days_back": {
                                "type": "integer",
                                "default": 365,
                                "minimum": 1,
                                "maximum": 3650,
                                "description": "Number of days to look back from now"
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
                                "description": "Optional limit on number of records"
                            }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["type", "start_date", "end_date"],
                        "properties": {
                            "type": { "const": "date_range" },
                            "start_date": {
                                "type": "string",
                                "format": "date-time",
                                "description": "Start of date range (ISO 8601)"
                            },
                            "end_date": {
                                "type": "string",
                                "format": "date-time",
                                "description": "End of date range (ISO 8601)"
                            }
                        }
                    }
                ]
            },
            "max_messages_per_sync": {
                "type": "integer",
                "default": 500,
                "minimum": 1,
                "maximum": 1000,
                "description": "Maximum number of messages to fetch per sync"
            },
            "query": {
                "type": "string",
                "description": "Gmail search query filter (optional, uses Gmail search syntax)"
            }
        }
    })
}

/// Example configuration for Gmail
fn gmail_config_example() -> serde_json::Value {
    json!({
        "label_ids": ["INBOX", "SENT"],
        "include_spam_trash": false,
        "sync_mode": "messages",
        "fetch_body": true,
        "sync_strategy": {
            "type": "time_window",
            "days_back": 365
        },
        "max_messages_per_sync": 500,
        "query": null
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_descriptor() {
        let desc = GoogleSource::descriptor();
        assert_eq!(desc.name, "google");
        assert_eq!(desc.auth_type, AuthType::OAuth2);
        assert!(desc.oauth_config.is_some());
        assert_eq!(desc.streams.len(), 2);
    }

    #[test]
    fn test_calendar_stream() {
        let desc = GoogleSource::descriptor();
        let calendar = desc.streams.iter().find(|s| s.name == "calendar");
        assert!(calendar.is_some());

        let cal = calendar.unwrap();
        assert_eq!(cal.table_name, "stream_google_calendar");
        assert!(cal.supports_incremental);
        assert!(cal.supports_full_refresh);
    }

    #[test]
    fn test_gmail_stream() {
        let desc = GoogleSource::descriptor();
        let gmail = desc.streams.iter().find(|s| s.name == "gmail");
        assert!(gmail.is_some());

        let gm = gmail.unwrap();
        assert_eq!(gm.table_name, "stream_google_gmail");
        assert!(gm.supports_incremental);
    }

    #[test]
    fn test_config_schemas_valid() {
        // Ensure schemas are valid JSON
        let cal_schema = calendar_config_schema();
        assert_eq!(cal_schema["type"], "object");
        assert!(cal_schema["properties"].is_object());

        let gmail_schema = gmail_config_schema();
        assert_eq!(gmail_schema["type"], "object");
        assert!(gmail_schema["properties"].is_object());
    }
}