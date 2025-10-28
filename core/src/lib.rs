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

// Re-export OAuth types
pub use oauth::TokenManager;

// Re-export Scheduler
pub use scheduler::Scheduler;

// Re-export library API functions
pub use api::{
    // Generic source management
    list_sources, get_source, delete_source, get_source_status, get_sync_history, get_stream_sync_history,

    // OAuth & source registration
    initiate_oauth_flow, handle_oauth_callback, create_source, register_device,
    OAuthAuthorizeResponse, CreateSourceRequest, RegisterDeviceRequest,

    // Stream management
    list_source_streams, get_stream_info, enable_stream, disable_stream,
    update_stream_config, update_stream_schedule,
    StreamInfo, EnableStreamRequest, UpdateStreamConfigRequest, UpdateStreamScheduleRequest,

    // Stream sync (new pattern - use this!)
    sync_stream,

    // Registry/catalog
    list_available_sources, get_source_info, get_stream_descriptor, list_all_streams,

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
