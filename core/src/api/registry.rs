//! Catalog and registry API - query available sources and streams

/// List all available sources in the catalog
pub fn list_available_sources() -> Vec<&'static crate::registry::SourceDescriptor> {
    crate::registry::list_sources()
}

/// Get information about a specific source
pub fn get_source_info(name: &str) -> Option<&'static crate::registry::SourceDescriptor> {
    crate::registry::get_source(name)
}

/// Get descriptor for a specific stream from the registry
pub fn get_stream_descriptor(
    source_name: &str,
    stream_name: &str,
) -> Option<&'static crate::registry::StreamDescriptor> {
    crate::registry::get_stream(source_name, stream_name)
}

/// List all streams across all sources
pub fn list_all_streams() -> Vec<(&'static str, &'static crate::registry::StreamDescriptor)> {
    crate::registry::list_all_streams()
}
