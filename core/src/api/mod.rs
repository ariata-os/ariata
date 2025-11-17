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
//! - `ontologies` - Ontology table queries
//! - `actions` - Temporal pursuits management (tasks/initiatives/aspirations)
//! - `axiology` - Value system management (values/telos/virtues/vices/habits/temperaments/preferences)
//! - `rate_limit` - API usage tracking and rate limiting
//! - `models` - LLM model configurations
//! - `agents` - AI agent configurations

pub mod actions;
pub mod agents;
pub mod rate_limit;
pub mod assistant_profile;
pub mod axiology;
pub mod device_pairing;
pub mod jobs;
pub mod models;
pub mod oauth;
pub mod ontologies;
pub mod profile;
pub mod registry;
pub mod sources;
pub mod streams;
pub mod tools;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use streams::StreamInfo;
pub use types::{Source, SourceStatus};

// Re-export all functions for convenience
pub use actions::{
    create_aspiration, create_initiative, create_task, delete_aspiration, delete_initiative,
    delete_task, get_aspiration, get_initiative, get_task, list_aspirations, list_initiatives,
    list_tags, list_tasks, update_aspiration, update_initiative, update_task, Aspiration,
    CreateAspirationRequest, CreateTaskRequest, Initiative, Task, UpdateAspirationRequest,
    UpdateTaskRequest,
};
pub use assistant_profile::{
    get_assistant_name, get_assistant_profile, get_pinned_tools, update_assistant_profile,
    UpdateAssistantProfileRequest,
};
pub use axiology::{
    create_habit, create_preference, create_telos, create_temperament, create_value, create_vice,
    create_virtue, delete_habit, delete_preference, delete_telos, delete_temperament,
    delete_value, delete_vice, delete_virtue, get_habit, get_preference, get_telos,
    get_temperament, get_value, get_vice, get_virtue, list_habits, list_preferences, list_telos,
    list_temperaments, list_values, list_vices, list_virtues, update_habit, update_preference,
    update_telos, update_temperament, update_value, update_vice, update_virtue,
    CreateHabitRequest, CreatePreferenceRequest, CreateSimpleRequest, Habit, Preference,
    Temperament, Telos, UpdateHabitRequest, UpdatePreferenceRequest, UpdateSimpleRequest, Value,
    Vice, Virtue,
};
pub use device_pairing::{
    check_pairing_status, complete_device_pairing, initiate_device_pairing, list_pending_pairings,
    update_last_seen, validate_device_token, verify_device, DeviceInfo, DeviceVerified,
    PairingCompleted, PairingInitiated, PairingStatus, PendingPairing,
};
pub use jobs::{
    cancel_job, get_job_history, get_job_status, query_jobs, trigger_stream_sync,
    CreateJobResponse, QueryJobsRequest,
};
pub use oauth::{
    create_source, handle_oauth_callback, initiate_oauth_flow, register_device,
    CreateSourceRequest, OAuthAuthorizeRequest, OAuthAuthorizeResponse, OAuthCallbackParams,
    RegisterDeviceRequest,
};
pub use profile::{get_display_name, get_profile, update_profile, UpdateProfileRequest};
pub use registry::{
    get_source_info, get_stream_descriptor, list_all_streams, list_available_sources,
};
pub use sources::{
    delete_source, get_source, get_source_status, list_sources, pause_source, resume_source,
};
pub use streams::{
    disable_stream, enable_stream, get_stream_info, list_source_streams, update_stream_config,
    update_stream_schedule, EnableStreamRequest, UpdateStreamConfigRequest,
    UpdateStreamScheduleRequest,
};
pub use tools::{get_tool, list_tools, update_tool, ListToolsQuery, Tool, UpdateToolRequest};
pub use rate_limit::{
    check_rate_limit, get_usage_stats, record_usage, RateLimitError, RateLimits,
    TokenUsage, UsageStats,
};
pub use models::{get_model, list_models, ModelInfo};
pub use agents::{get_agent, list_agents, AgentInfo};
