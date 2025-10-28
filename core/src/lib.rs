//! Ariata - Open Source Personal Data Ecosystem
//!
//! High-performance data pipeline for personal data collection, storage, and analysis.

pub mod api;
pub mod client;
pub mod database;
pub mod error;
pub mod oauth;
pub mod registry;
pub mod scheduler;
pub mod server;
pub mod setup;
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
    create_source,
    delete_source,
    disable_stream,
    enable_stream,
    get_source,
    get_source_info,
    get_source_status,
    get_stream_descriptor,
    get_stream_info,
    get_stream_sync_history,

    get_sync_history,
    handle_oauth_callback,
    // OAuth & source registration
    initiate_oauth_flow,
    list_all_streams,

    // Registry/catalog
    list_available_sources,
    // Stream management
    list_source_streams,
    // Generic source management
    list_sources,
    register_device,
    // Stream sync (new pattern - use this!)
    sync_stream,

    update_stream_config,
    update_stream_schedule,
    CreateSourceRequest,
    EnableStreamRequest,
    OAuthAuthorizeResponse,
    OAuthCallbackParams,
    RegisterDeviceRequest,

    // Types
    Source,
    SourceStatus,
    StreamInfo,
    SyncLog,
    UpdateStreamConfigRequest,
    UpdateStreamScheduleRequest,
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
