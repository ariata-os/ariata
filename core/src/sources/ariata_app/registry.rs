//! Ariata App Source Registry
//!
//! Defines the source descriptor for the internal Ariata web application source.

use crate::registry::{AuthType, SourceDescriptor, SourceRegistry, StreamDescriptor};
use serde_json::json;

/// Ariata App source registration
pub struct AriataAppSource;

impl SourceRegistry for AriataAppSource {
    fn descriptor() -> SourceDescriptor {
        SourceDescriptor {
            name: "ariata_app",
            display_name: "Ariata App",
            description: "Internal operational data from Ariata web application",
            auth_type: AuthType::None, // Internal source, no external auth needed
            oauth_config: None,
            icon: Some("ri:app-store-fill"),
            streams: vec![StreamDescriptor::new("app_export")
                .display_name("Chat Export")
                .description("Exports chat sessions from app.chat_sessions to ELT pipeline")
                .table_name("stream_ariata_ai_chat")
                .config_schema(json!({}))
                .config_example(json!({}))
                .supports_incremental(true)
                .supports_full_refresh(true) // Allow full refresh for recovery scenarios
                .default_cron_schedule("*/5 * * * *") // Every 5 minutes (5-field standard cron)
                .build()],
        }
    }
}
