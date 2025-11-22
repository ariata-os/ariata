//! Source and stream registry for catalog/discovery
//!
//! This module provides a compile-time registry of all available sources and streams.
//! The registry is initialized once at startup and contains static metadata about what
//! data sources and streams are available in the system.
//!
//! **Terminology:**
//! - **RegisteredSource**: A source type in the registry (e.g., "Google", "Notion")
//! - **SourceConnection**: A user's connected account instance (stored in `source_connections` table with auth tokens)
//! - **RegisteredStream**: A stream type that a source offers (e.g., "Calendar", "Gmail")
//! - **StreamConnection**: A user's enabled stream with configuration (stored in `stream_connections` table)
//!
//! Frontends and CLIs query this registry (via the catalog API) to discover:
//! - What sources are available (Google, Notion, iOS, Mac, Ariata)
//! - What streams each source provides (Calendar, Gmail, Pages, HealthKit, etc.)
//! - What configuration options each stream accepts
//! - What database schema each stream uses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Authentication type required for a source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../apps/web/src/lib/types/")]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// OAuth 2.0 authentication
    OAuth2,
    /// API key authentication
    ApiKey,
    /// Device-based (no external auth needed)
    Device,
    /// No authentication required
    None,
}

/// A registered source type (e.g., "Google", "Notion")
/// This defines what sources CAN be connected.
/// For actual user connections, see api::SourceConnection.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../apps/web/src/lib/types/")]
pub struct RegisteredSource {
    /// Unique identifier (e.g., "google", "notion", "ios")
    pub name: &'static str,

    /// Human-readable display name
    pub display_name: &'static str,

    /// Description of what this source provides
    pub description: &'static str,

    /// Authentication type required
    pub auth_type: AuthType,

    /// Available streams for this source
    pub streams: Vec<RegisteredStream>,

    /// OAuth-specific configuration (if applicable)
    pub oauth_config: Option<OAuthConfig>,

    /// Iconify icon name for UI display (e.g., "ri:google-fill")
    pub icon: Option<&'static str>,
}

/// OAuth configuration details for a source
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../apps/web/src/lib/types/")]
pub struct OAuthConfig {
    /// OAuth scopes required
    pub scopes: Vec<&'static str>,

    /// Authorization URL pattern
    pub auth_url: &'static str,

    /// Token URL pattern
    pub token_url: &'static str,
}

/// A registered stream type (e.g., "Calendar", "Gmail")
/// This defines what streams a source offers.
/// For user stream state, see api::StreamConnection.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../apps/web/src/lib/types/")]
pub struct RegisteredStream {
    /// Stream identifier (e.g., "calendar", "gmail")
    pub name: &'static str,

    /// Human-readable display name
    pub display_name: &'static str,

    /// Description of what this stream provides
    pub description: &'static str,

    /// Database table name (e.g., "stream_google_calendar")
    pub table_name: &'static str,

    /// Target ontology tables this stream feeds into
    /// e.g., ["praxis_calendar"] for Google Calendar
    /// e.g., ["health_heart_rate", "health_sleep", "health_workout"] for HealthKit
    pub target_ontologies: Vec<&'static str>,

    /// JSON schema for configuration (serialized as JSON)
    #[ts(type = "any")]
    pub config_schema: serde_json::Value,

    /// Example configuration
    #[ts(type = "any")]
    pub config_example: serde_json::Value,

    /// Whether this stream supports incremental sync
    pub supports_incremental: bool,

    /// Whether this stream supports full refresh
    pub supports_full_refresh: bool,

    /// Default cron schedule for this stream in 6-field format: sec min hour day month dow (e.g., "0 0 \*/6 * * *")
    pub default_cron_schedule: Option<&'static str>,
}

impl RegisteredStream {
    /// Create a new stream descriptor builder
    pub fn new(name: &'static str) -> StreamBuilder {
        StreamBuilder {
            name,
            display_name: name,
            description: "",
            table_name: "",
            target_ontologies: vec![],
            config_schema: serde_json::json!({}),
            config_example: serde_json::json!({}),
            supports_incremental: true,
            supports_full_refresh: true,
            default_cron_schedule: None,
        }
    }
}

/// Builder for RegisteredStream
pub struct StreamBuilder {
    name: &'static str,
    display_name: &'static str,
    description: &'static str,
    table_name: &'static str,
    target_ontologies: Vec<&'static str>,
    config_schema: serde_json::Value,
    config_example: serde_json::Value,
    supports_incremental: bool,
    supports_full_refresh: bool,
    default_cron_schedule: Option<&'static str>,
}

impl StreamBuilder {
    pub fn display_name(mut self, name: &'static str) -> Self {
        self.display_name = name;
        self
    }

    pub fn description(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    pub fn table_name(mut self, name: &'static str) -> Self {
        self.table_name = name;
        self
    }

    pub fn config_schema(mut self, schema: serde_json::Value) -> Self {
        self.config_schema = schema;
        self
    }

    pub fn config_example(mut self, example: serde_json::Value) -> Self {
        self.config_example = example;
        self
    }

    pub fn supports_incremental(mut self, supports: bool) -> Self {
        self.supports_incremental = supports;
        self
    }

    pub fn supports_full_refresh(mut self, supports: bool) -> Self {
        self.supports_full_refresh = supports;
        self
    }

    pub fn default_cron_schedule(mut self, schedule: &'static str) -> Self {
        self.default_cron_schedule = Some(schedule);
        self
    }

    /// Set target ontology tables this stream feeds into
    pub fn target_ontologies(mut self, ontologies: Vec<&'static str>) -> Self {
        self.target_ontologies = ontologies;
        self
    }

    pub fn build(self) -> RegisteredStream {
        RegisteredStream {
            name: self.name,
            display_name: self.display_name,
            description: self.description,
            table_name: self.table_name,
            target_ontologies: self.target_ontologies,
            config_schema: self.config_schema,
            config_example: self.config_example,
            supports_incremental: self.supports_incremental,
            supports_full_refresh: self.supports_full_refresh,
            default_cron_schedule: self.default_cron_schedule,
        }
    }
}

/// Trait for sources to register themselves in the catalog
pub trait SourceRegistry {
    /// Get the source descriptor
    fn descriptor() -> RegisteredSource;
}

/// Global registry of all sources
pub struct Registry {
    sources: HashMap<String, RegisteredSource>,
}

impl Registry {
    /// Create a new empty registry
    fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Register a source
    fn register(&mut self, descriptor: RegisteredSource) {
        self.sources.insert(descriptor.name.to_string(), descriptor);
    }

    /// Get all registered sources
    pub fn list_sources(&self) -> Vec<&RegisteredSource> {
        self.sources.values().collect()
    }

    /// Get a specific source by name
    pub fn get_source(&self, name: &str) -> Option<&RegisteredSource> {
        self.sources.get(name)
    }

    /// Get a specific stream from a source
    pub fn get_stream(&self, source_name: &str, stream_name: &str) -> Option<&RegisteredStream> {
        self.sources
            .get(source_name)
            .and_then(|source| source.streams.iter().find(|s| s.name == stream_name))
    }

    /// List all streams across all sources
    pub fn list_all_streams(&self) -> Vec<(&str, &RegisteredStream)> {
        self.sources
            .values()
            .flat_map(|source| {
                source
                    .streams
                    .iter()
                    .map(move |stream| (source.name, stream))
            })
            .collect()
    }
}

// Global registry instance (initialized via lazy_static)
use std::sync::OnceLock;

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Initialize the global registry (called once at startup)
fn init_registry() -> Registry {
    let mut registry = Registry::new();

    // Register internal sources
    registry.register(crate::sources::ariata::registry::AriataSource::descriptor());

    // Register OAuth sources
    registry.register(crate::sources::google::registry::GoogleSource::descriptor());
    registry.register(crate::sources::notion::registry::NotionSource::descriptor());
    registry.register(crate::sources::plaid::registry::PlaidSource::descriptor());

    // Register device sources
    registry.register(crate::sources::ios::registry::IosSource::descriptor());
    registry.register(crate::sources::mac::registry::MacSource::descriptor());

    registry
}

/// Get the global registry
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(init_registry)
}

/// List all available sources
pub fn list_sources() -> Vec<&'static RegisteredSource> {
    registry().list_sources()
}

/// Get information about a specific source
pub fn get_source(name: &str) -> Option<&'static RegisteredSource> {
    registry().get_source(name)
}

/// Get information about a specific stream
pub fn get_stream(source_name: &str, stream_name: &str) -> Option<&'static RegisteredStream> {
    registry().get_stream(source_name, stream_name)
}

/// List all streams across all sources
pub fn list_all_streams() -> Vec<(&'static str, &'static RegisteredStream)> {
    registry().list_all_streams()
}

/// Get a stream by its table name (e.g., "stream_google_calendar")
///
/// Returns the source name and stream reference if found.
pub fn get_stream_by_table_name(table_name: &str) -> Option<(&'static str, &'static RegisteredStream)> {
    registry()
        .list_all_streams()
        .into_iter()
        .find(|(_, stream)| stream.table_name == table_name)
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
///
/// ## Usage
///
/// If the name already starts with "stream_", it is returned as-is.
/// Otherwise, short names are expanded to their full stream table names.
pub fn normalize_stream_name(name: &str) -> String {
    if name.starts_with("stream_") {
        return name.to_string();
    }

    // Try to find by short name in registry
    for (_source_name, stream) in list_all_streams() {
        if stream.name == name {
            return stream.table_name.to_string();
        }
    }

    // Legacy fallback mappings for backwards compatibility
    match name {
        "app_export" => "stream_ariata_ai_chat",
        "gmail" => "stream_google_gmail",
        "calendar" => "stream_google_calendar",
        "pages" => "stream_notion_pages",
        "microphone" => "stream_ios_microphone",
        "healthkit" => "stream_ios_healthkit",
        "location" => "stream_ios_location",
        "apps" => "stream_mac_apps",
        "browser" => "stream_mac_browser",
        "imessage" | "messages" => "stream_mac_imessage",
        _ => name, // Return as-is if unknown
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let sources = list_sources();
        assert!(!sources.is_empty(), "Registry should have sources");
    }

    #[test]
    fn test_get_source() {
        let google = get_source("google");
        assert!(google.is_some(), "Should find Google source");

        if let Some(g) = google {
            assert_eq!(g.name, "google");
            assert!(!g.streams.is_empty(), "Google should have streams");
        }
    }

    /// Export TypeScript types for frontend use
    #[test]
    fn export_typescript_types() {
        AuthType::export().expect("Failed to export AuthType");
        RegisteredSource::export().expect("Failed to export RegisteredSource");
        OAuthConfig::export().expect("Failed to export OAuthConfig");
        RegisteredStream::export().expect("Failed to export RegisteredStream");
    }

    #[test]
    fn test_get_stream() {
        let calendar = get_stream("google", "calendar");
        assert!(calendar.is_some(), "Should find Google Calendar stream");

        if let Some(cal) = calendar {
            assert_eq!(cal.name, "calendar");
            assert_eq!(cal.table_name, "stream_google_calendar");
        }
    }

    #[test]
    fn test_list_all_streams() {
        let streams = list_all_streams();
        assert!(!streams.is_empty(), "Should have streams registered");

        // Check that stream names are properly formatted
        for (source, stream) in streams {
            assert!(stream.table_name.starts_with("stream_"));
            assert!(stream.table_name.contains(source));
        }
    }
}
