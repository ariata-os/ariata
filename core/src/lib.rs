//! Ariata - Open Source Personal Data Ecosystem
//!
//! High-performance data pipeline for personal data collection, storage, and analysis.

pub mod api;
pub mod client;
pub mod database;
pub mod error;
pub mod oauth;
pub mod pipeline;
pub mod registry;
pub mod scheduler;
pub mod server;
pub mod sources;
pub mod storage;

// Re-export main types
pub use client::{Ariata, AriataBuilder};
pub use error::{Error, Result};

// Re-export scheduler types
pub use scheduler::Scheduler;

// Re-export OAuth types (for CLI)
pub use oauth::OAuthManager;

// Re-export library API functions
pub use api::{
    // Generic source management
    list_sources, get_source, delete_source, get_source_status, sync_source, get_sync_history,

    // Registry/catalog
    list_available_sources, get_source_info, get_stream_info, list_all_streams,

    // Types
    Source, SourceStatus, SyncLog,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
