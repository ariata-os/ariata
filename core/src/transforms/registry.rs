//! Transform routing registry
//!
//! Centralized mapping of stream names to their transform source/target tables.
//!
//! This is the SINGLE SOURCE OF TRUTH for stream→ontology mappings.
//! DO NOT duplicate these mappings elsewhere in the codebase.
//!
//! ## Transform Stage Taxonomy
//!
//! Transforms are organized by ELT stage:
//!
//! ### 1. **Ingest**: Stream → Primitive (in `/sources/{provider}/`)
//! - Purpose: Normalize raw provider data to standard ontology schema
//! - Example: `stream_ios_location` → `location_point`
//! - Provider-specific: Yes (Gmail API ≠ Outlook API)
//!
//! ### 2. **Enrich**: Primitive → Semantic Primitive (in `/transforms/enrich/{domain}/`)
//! - Purpose: Derive semantic meaning from raw primitives
//! - Example: `location_point` → `location_visit` (clustering)
//! - Provider-agnostic: Yes (works with any GPS data)
//!
//! ### 3. **Aggregate**: Multi-Primitive → Semantic (in `/transforms/aggregate/{domain}/`)
//! - Purpose: Synthesize insights from multiple primitive types
//! - Example: `calendar` + `location` → `contextualized_event`
//! - Cross-domain: Often
//!
//! ### 4. **Narrative**: Cross-Domain → Narrative (in `/transforms/narrative/`)
//! - Purpose: Generate narrative prose from multiple domains
//! - Example: all primitives → `narrative_chunks` (day/week)
//! - Agent-driven: Yes (LLM-based synthesis)

use crate::error::{Error, Result};

/// Transform route configuration
#[derive(Debug, Clone)]
pub struct TransformRoute {
    pub source_table: &'static str,
    pub target_tables: Vec<&'static str>,
    pub domain: &'static str,
    pub transform_stage: &'static str,
}

/// Get the transform route for a given stream name
pub fn get_transform_route(stream_name: &str) -> Result<TransformRoute> {
    match stream_name {
        "stream_ios_microphone" => Ok(TransformRoute {
            source_table: "stream_ios_microphone",
            target_tables: vec!["speech_transcription"],
            domain: "speech",
            transform_stage: "transcription",
        }),
        "stream_google_gmail" => Ok(TransformRoute {
            source_table: "stream_google_gmail",
            target_tables: vec!["social_email"],
            domain: "social",
            transform_stage: "email_normalization",
        }),
        "stream_google_calendar" => Ok(TransformRoute {
            source_table: "stream_google_calendar",
            target_tables: vec!["praxis_calendar"],
            domain: "praxis",
            transform_stage: "calendar_normalization",
        }),
        "stream_notion_pages" => Ok(TransformRoute {
            source_table: "stream_notion_pages",
            target_tables: vec!["knowledge_document"],
            domain: "knowledge",
            transform_stage: "document_extraction",
        }),
        "stream_ariata_ai_chat" => Ok(TransformRoute {
            source_table: "stream_ariata_ai_chat",
            target_tables: vec!["knowledge_ai_conversation"],
            domain: "knowledge",
            transform_stage: "conversation_structuring",
        }),
        "stream_ios_healthkit" => Ok(TransformRoute {
            source_table: "stream_ios_healthkit",
            target_tables: vec![
                "health_heart_rate",
                "health_hrv",
                "health_steps",
                "health_sleep",
                "health_workout",
            ],
            domain: "health",
            transform_stage: "health_metrics_normalization",
        }),
        "stream_ios_location" => Ok(TransformRoute {
            source_table: "stream_ios_location",
            target_tables: vec!["location_point"],
            domain: "location",
            transform_stage: "location_normalization",
        }),
        "stream_mac_apps" => Ok(TransformRoute {
            source_table: "stream_mac_apps",
            target_tables: vec!["activity_app_usage"],
            domain: "activity",
            transform_stage: "app_usage_normalization",
        }),
        "stream_mac_browser" => Ok(TransformRoute {
            source_table: "stream_mac_browser",
            target_tables: vec!["activity_web_browsing"],
            domain: "activity",
            transform_stage: "web_browsing_normalization",
        }),
        "stream_mac_imessage" => Ok(TransformRoute {
            source_table: "stream_mac_imessage",
            target_tables: vec!["social_message"],
            domain: "social",
            transform_stage: "message_normalization",
        }),
        "location_point" => Ok(TransformRoute {
            source_table: "location_point",
            target_tables: vec!["location_visit"],
            domain: "location",
            transform_stage: "visit_clustering",
        }),
        "speech_transcription" => Ok(TransformRoute {
            source_table: "speech_transcription",
            target_tables: vec!["semantic_inferences"],
            domain: "semantic",
            transform_stage: "semantic_parsing",
        }),
        _ => Err(Error::InvalidInput(format!(
            "Unknown stream for transform: '{}'. Valid streams: stream_ios_microphone, stream_google_gmail, stream_google_calendar, stream_notion_pages, stream_ariata_ai_chat, stream_ios_healthkit, stream_ios_location, stream_mac_apps, stream_mac_browser, stream_mac_imessage, location_point, speech_transcription",
            stream_name
        ))),
    }
}

/// Get all available transform stream names
pub fn list_transform_streams() -> Vec<&'static str> {
    vec![
        "stream_ios_microphone",
        "stream_ios_healthkit",
        "stream_ios_location",
        "stream_mac_apps",
        "stream_mac_browser",
        "stream_mac_imessage",
        "location_point",
        "stream_google_gmail",
        "stream_google_calendar",
        "stream_notion_pages",
        "stream_ariata_ai_chat",
        "speech_transcription",
    ]
}

/// Normalize stream name from short form to full table name
///
/// ## Naming Convention
///
/// The system uses a three-tier naming architecture:
/// 1. **Stream Name** (registered in data.streams) - e.g., "app_export", "gmail"
/// 2. **Stream Table** (object storage) - e.g., "stream_ariata_ai_chat", "stream_google_gmail"
/// 3. **Ontology Table** (data schema) - e.g., "knowledge_ai_conversation", "social_email"
///
/// This function maps stream names (tier 1) to stream tables (tier 2).
/// The transform registry then maps stream tables to ontology tables.
///
/// ## Provider Semantics
///
/// - `ariata` - Internal system source and provider label
/// - Stream: `app_export` - Exports chat sessions from app.chat_sessions
/// - Table: `stream_ariata_ai_chat` - Raw exported data in object storage
/// - Ontology: `knowledge_ai_conversation` - Normalized conversation records
///
/// ## Usage
///
/// If the name already starts with "stream_", it is returned as-is.
/// Otherwise, short names are expanded to their full stream table names.
pub fn normalize_stream_name(name: &str) -> String {
    if name.starts_with("stream_") {
        name.to_string()
    } else {
        match name {
            // Internal Ariata streams
            "app_export" => "stream_ariata_ai_chat",

            // External provider streams
            "gmail" => "stream_google_gmail",
            "calendar" => "stream_google_calendar",
            "pages" => "stream_notion_pages",

            // iOS device streams
            "microphone" => "stream_ios_microphone",
            "healthkit" => "stream_ios_healthkit",
            "location" => "stream_ios_location",

            // macOS device streams
            "apps" => "stream_mac_apps",
            "browser" => "stream_mac_browser",
            "imessage" => "stream_mac_imessage",
            "messages" => "stream_mac_imessage",

            _ => name, // Return as-is if unknown
        }
        .to_string()
    }
}
