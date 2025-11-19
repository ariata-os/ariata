//! Ariata - Open Source Personal Data Ecosystem
//!
//! High-performance data pipeline for personal data collection, storage, and analysis.

pub mod api;
pub mod cli;
pub mod client;
pub mod database;
pub mod error;
pub mod jobs;
pub mod llm;
pub mod mcp;
pub mod registry;
pub mod scheduler;
pub mod seeding;
pub mod server;
pub mod setup;
pub mod sources;
pub mod storage;
pub mod transcription;
pub mod transforms;

// Re-export main types
pub use client::{Ariata, AriataBuilder};
pub use error::{Error, Result};

// Re-export OAuth types
pub use sources::base::TokenManager;

// Re-export Scheduler
pub use scheduler::Scheduler;

// Re-export sync types
pub use sources::base::SyncMode;

// Re-export library API functions
pub use api::{
    // Device pairing
    check_pairing_status,
    complete_device_pairing,
    create_source,
    delete_source,
    disable_stream,
    enable_stream,
    get_source,
    get_source_info,
    get_source_status,
    get_stream_descriptor,
    get_stream_info,
    handle_oauth_callback,
    initiate_device_pairing,
    // OAuth & source registration
    initiate_oauth_flow,
    list_all_streams,

    // Registry/catalog
    list_available_sources,
    list_pending_pairings,
    // Stream management
    list_source_streams,
    // Generic source management
    list_sources,
    register_device,

    update_last_seen,
    update_stream_config,
    update_stream_schedule,
    validate_device_token,
    CreateSourceRequest,
    DeviceInfo,
    EnableStreamRequest,
    OAuthAuthorizeResponse,
    OAuthCallbackParams,
    PairingCompleted,
    PairingInitiated,
    PairingStatus,
    PendingPairing,
    RegisterDeviceRequest,

    // Types
    SourceConnection,
    SourceConnectionStatus,
    StreamConnection,
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
