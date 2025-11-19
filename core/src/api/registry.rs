//! Catalog API - query available sources and streams from the registry
//!
//! **Terminology:**
//! - **Catalog**: User-facing API concept (HTTP endpoint: `/api/catalog/sources`)
//! - **Registry**: Internal compile-time data structure containing source/stream metadata
//! - **Provider**: The type/name of a source (e.g., "google", "notion", "ios")
//!
//! The catalog exposes the registry's contents via HTTP endpoints, allowing frontends
//! to discover what sources and streams are available before configuring them.

/// List all available sources in the catalog
pub fn list_available_sources() -> Vec<&'static crate::registry::RegisteredSource> {
    crate::registry::list_sources()
}

/// Get information about a specific source
pub fn get_source_info(name: &str) -> Option<&'static crate::registry::RegisteredSource> {
    crate::registry::get_source(name)
}

/// Get descriptor for a specific stream from the registry
pub fn get_stream_descriptor(
    source_name: &str,
    stream_name: &str,
) -> Option<&'static crate::registry::RegisteredStream> {
    crate::registry::get_stream(source_name, stream_name)
}

/// List all streams across all sources
pub fn list_all_streams() -> Vec<(&'static str, &'static crate::registry::RegisteredStream)> {
    crate::registry::list_all_streams()
}
