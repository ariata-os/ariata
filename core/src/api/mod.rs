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
//! - `streams` - Stream management and configuration
//! - `sync` - Sync execution and history
//! - `registry` - Catalog/registry queries

pub mod oauth;
pub mod registry;
pub mod sources;
pub mod streams;
pub mod sync;
pub mod types;

// Re-export commonly used types
pub use streams::StreamInfo;
pub use types::{Source, SourceStatus, SyncLog};

// Re-export all functions for convenience
pub use oauth::{
    create_source, handle_oauth_callback, initiate_oauth_flow, register_device,
    CreateSourceRequest, OAuthAuthorizeRequest, OAuthAuthorizeResponse, OAuthCallbackParams,
    RegisterDeviceRequest,
};
pub use registry::{
    get_source_info, get_stream_descriptor, list_all_streams, list_available_sources,
};
pub use sources::{delete_source, get_source, get_source_status, list_sources};
pub use streams::{
    disable_stream, enable_stream, get_stream_info, list_source_streams, update_stream_config,
    update_stream_schedule, EnableStreamRequest, UpdateStreamConfigRequest,
    UpdateStreamScheduleRequest,
};
pub use sync::{get_stream_sync_history, get_sync_history, sync_stream};
