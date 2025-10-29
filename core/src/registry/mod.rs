//! Source and stream registry for catalog/discovery
//!
//! This module provides a compile-time registry of all available sources and streams.
//! Frontends and CLIs can query this registry to discover:
//! - What sources are available (Google, Strava, Notion, iOS, Mac)
//! - What streams each source provides (Calendar, Gmail, Activities, etc.)
//! - What configuration options each stream accepts
//! - What database schema each stream uses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication type required for a source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Metadata describing a data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDescriptor {
    /// Unique identifier (e.g., "google", "strava")
    pub name: &'static str,

    /// Human-readable display name
    pub display_name: &'static str,

    /// Description of what this source provides
    pub description: &'static str,

    /// Authentication type required
    pub auth_type: AuthType,

    /// Available streams for this source
    pub streams: Vec<StreamDescriptor>,

    /// OAuth-specific configuration (if applicable)
    pub oauth_config: Option<OAuthConfig>,

    /// Iconify icon name for UI display (e.g., "ri:google-fill")
    pub icon: Option<&'static str>,
}

/// OAuth configuration details for a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth scopes required
    pub scopes: Vec<&'static str>,

    /// Authorization URL pattern
    pub auth_url: &'static str,

    /// Token URL pattern
    pub token_url: &'static str,
}

/// Metadata describing a data stream within a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDescriptor {
    /// Stream identifier (e.g., "calendar", "gmail")
    pub name: &'static str,

    /// Human-readable display name
    pub display_name: &'static str,

    /// Description of what this stream provides
    pub description: &'static str,

    /// Database table name (e.g., "stream_google_calendar")
    pub table_name: &'static str,

    /// JSON schema for configuration (serialized as JSON)
    pub config_schema: serde_json::Value,

    /// Example configuration
    pub config_example: serde_json::Value,

    /// Whether this stream supports incremental sync
    pub supports_incremental: bool,

    /// Whether this stream supports full refresh
    pub supports_full_refresh: bool,

    /// Default cron schedule for this stream (e.g., "0 */6 * * *")
    pub default_cron_schedule: Option<&'static str>,
}

impl StreamDescriptor {
    /// Create a new stream descriptor builder
    pub fn new(name: &'static str) -> StreamDescriptorBuilder {
        StreamDescriptorBuilder {
            name,
            display_name: name,
            description: "",
            table_name: "",
            config_schema: serde_json::json!({}),
            config_example: serde_json::json!({}),
            supports_incremental: true,
            supports_full_refresh: true,
            default_cron_schedule: None,
        }
    }
}

/// Builder for StreamDescriptor
pub struct StreamDescriptorBuilder {
    name: &'static str,
    display_name: &'static str,
    description: &'static str,
    table_name: &'static str,
    config_schema: serde_json::Value,
    config_example: serde_json::Value,
    supports_incremental: bool,
    supports_full_refresh: bool,
    default_cron_schedule: Option<&'static str>,
}

impl StreamDescriptorBuilder {
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

    pub fn build(self) -> StreamDescriptor {
        StreamDescriptor {
            name: self.name,
            display_name: self.display_name,
            description: self.description,
            table_name: self.table_name,
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
    fn descriptor() -> SourceDescriptor;
}

/// Global registry of all sources
pub struct Registry {
    sources: HashMap<String, SourceDescriptor>,
}

impl Registry {
    /// Create a new empty registry
    fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Register a source
    fn register(&mut self, descriptor: SourceDescriptor) {
        self.sources.insert(descriptor.name.to_string(), descriptor);
    }

    /// Get all registered sources
    pub fn list_sources(&self) -> Vec<&SourceDescriptor> {
        self.sources.values().collect()
    }

    /// Get a specific source by name
    pub fn get_source(&self, name: &str) -> Option<&SourceDescriptor> {
        self.sources.get(name)
    }

    /// Get a specific stream from a source
    pub fn get_stream(&self, source_name: &str, stream_name: &str) -> Option<&StreamDescriptor> {
        self.sources
            .get(source_name)
            .and_then(|source| {
                source.streams.iter().find(|s| s.name == stream_name)
            })
    }

    /// List all streams across all sources
    pub fn list_all_streams(&self) -> Vec<(&str, &StreamDescriptor)> {
        self.sources
            .values()
            .flat_map(|source| {
                source.streams.iter().map(move |stream| (source.name, stream))
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

    // Register OAuth sources
    registry.register(crate::sources::google::registry::GoogleSource::descriptor());
    registry.register(crate::sources::notion::registry::NotionSource::descriptor());

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
pub fn list_sources() -> Vec<&'static SourceDescriptor> {
    registry().list_sources()
}

/// Get information about a specific source
pub fn get_source(name: &str) -> Option<&'static SourceDescriptor> {
    registry().get_source(name)
}

/// Get information about a specific stream
pub fn get_stream(source_name: &str, stream_name: &str) -> Option<&'static StreamDescriptor> {
    registry().get_stream(source_name, stream_name)
}

/// List all streams across all sources
pub fn list_all_streams() -> Vec<(&'static str, &'static StreamDescriptor)> {
    registry().list_all_streams()
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