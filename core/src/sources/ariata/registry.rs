//! Ariata Source Registry
//!
//! Defines the source descriptor for the internal Ariata web application source.

use crate::registry::{AuthType, RegisteredSource, SourceRegistry, RegisteredStream};
use serde_json::json;

/// Ariata source registration
pub struct AriataSource;

impl SourceRegistry for AriataSource {
    fn descriptor() -> RegisteredSource {
        RegisteredSource {
            name: "ariata",
            display_name: "Ariata",
            description: "Internal operational data from Ariata web application",
            auth_type: AuthType::None, // Internal source, no external auth needed
            oauth_config: None,
            icon: Some("ri:app-store-fill"),
            streams: vec![RegisteredStream::new("app_export")
                .display_name("Chat Export")
                .description("Exports chat sessions from app.chat_sessions to ELT pipeline")
                .table_name("stream_ariata_ai_chat")
                .config_schema(json!({}))
                .config_example(json!({}))
                .supports_incremental(true)
                .supports_full_refresh(true) // Allow full refresh for recovery scenarios
                .default_cron_schedule("0 */5 * * * *") // Every 5 minutes (6-field: sec min hour day month dow)
                .build()],
        }
    }
}
