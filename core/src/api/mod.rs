//! Library-level API functions for programmatic access
//!
//! This module provides the SDK/API layer that can be used by:
//! - CLI applications
//! - HTTP servers
//! - Python wrappers and other bindings
//!
//! The API is organized into submodules for clarity:
//! - `types` - Shared types
//! - `sources` - Source CRUD operations
//! - `oauth` - OAuth flows and authentication
//! - `device_pairing` - Device registration and pairing
//! - `streams` - Stream management and configuration
//! - `jobs` - Async job tracking and management
//! - `registry` - Catalog/registry queries

pub mod device_pairing;
pub mod jobs;
pub mod oauth;
pub mod registry;
pub mod sources;
pub mod streams;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use streams::StreamInfo;
pub use types::{Source, SourceStatus};

// Re-export all functions for convenience
pub use device_pairing::{
    check_pairing_status, complete_device_pairing, initiate_device_pairing, list_pending_pairings,
    update_last_seen, validate_device_token, verify_device, DeviceInfo, DeviceVerified,
    PairingCompleted, PairingInitiated, PairingStatus, PendingPairing,
};
pub use oauth::{
    create_source, handle_oauth_callback, initiate_oauth_flow, register_device,
    CreateSourceRequest, OAuthAuthorizeRequest, OAuthAuthorizeResponse, OAuthCallbackParams,
    RegisterDeviceRequest,
};
pub use registry::{
    get_source_info, get_stream_descriptor, list_all_streams, list_available_sources,
};
pub use sources::{delete_source, get_source, get_source_status, list_sources, pause_source, resume_source};
pub use streams::{
    disable_stream, enable_stream, get_stream_info, list_source_streams, update_stream_config,
    update_stream_schedule, EnableStreamRequest, UpdateStreamConfigRequest,
    UpdateStreamScheduleRequest,
};
pub use jobs::{
    cancel_job, get_job_history, get_job_status, query_jobs, trigger_stream_sync, CreateJobResponse,
    QueryJobsRequest,
};
