//! Transform routing registry
//!
//! Centralized mapping of stream names to their transform source/target tables.
//!
//! This is the SINGLE SOURCE OF TRUTH for stream→ontology mappings.
//! DO NOT duplicate these mappings elsewhere in the codebase.

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
            target_tables: vec!["activity_calendar_entry"],
            domain: "activity",
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
        "speech_transcription" => Ok(TransformRoute {
            source_table: "speech_transcription",
            target_tables: vec!["semantic_inferences"],
            domain: "semantic",
            transform_stage: "semantic_parsing",
        }),
        _ => Err(Error::InvalidInput(format!(
            "Unknown stream for transform: '{}'. Valid streams: stream_ios_microphone, stream_google_gmail, stream_google_calendar, stream_notion_pages, stream_ariata_ai_chat, speech_transcription",
            stream_name
        ))),
    }
}

/// Get all available transform stream names
pub fn list_transform_streams() -> Vec<&'static str> {
    vec![
        "stream_ios_microphone",
        "stream_google_gmail",
        "stream_google_calendar",
        "stream_notion_pages",
        "stream_ariata_ai_chat",
        "speech_transcription",
    ]
}

/// Normalize stream name from short form to full table name
///
/// Converts short stream names (e.g., "gmail") to full table names (e.g., "stream_google_gmail").
/// If the name already starts with "stream_", it is returned as-is.
///
/// This centralizes the mapping logic to avoid duplication across the codebase.
pub fn normalize_stream_name(name: &str) -> String {
    if name.starts_with("stream_") {
        name.to_string()
    } else {
        match name {
            "ai_chat" => "stream_ariata_ai_chat",
            "gmail" => "stream_google_gmail",
            "calendar" => "stream_google_calendar",
            "pages" => "stream_notion_pages",
            "microphone" => "stream_ios_microphone",
            _ => name, // Return as-is if unknown
        }
        .to_string()
    }
}
