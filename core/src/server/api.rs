//! REST API handlers for source management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ingest::AppState;
use crate::error::Error;

/// Helper to convert Result to Response with proper status code
fn api_response<T: Serialize>(result: crate::error::Result<T>) -> Response {
    match result {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => error_response(e),
    }
}

/// Helper to convert Error to Response with appropriate status code
fn error_response(error: Error) -> Response {
    let (status, message) = match &error {
        Error::NotFound(_) => (StatusCode::NOT_FOUND, error.to_string()),
        Error::Unauthorized(_) => (StatusCode::UNAUTHORIZED, error.to_string()),
        Error::InvalidInput(_) => (StatusCode::BAD_REQUEST, error.to_string()),
        Error::Database(msg) if msg.contains("already has an active") => {
            (StatusCode::CONFLICT, error.to_string())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };

    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

/// Helper to create a success message response
fn success_message(message: &str) -> Response {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "message": message })),
    )
        .into_response()
}

/// List all sources
pub async fn list_sources_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_sources(state.db.pool()).await)
}

/// Get a specific source by ID
pub async fn get_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_source(state.db.pool(), source_id).await)
}

/// Pause a source
pub async fn pause_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    api_response(crate::api::pause_source(state.db.pool(), source_id).await)
}

/// Resume a source
pub async fn resume_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    api_response(crate::api::resume_source(state.db.pool(), source_id).await)
}

/// Delete a source by ID
pub async fn delete_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::delete_source(state.db.pool(), source_id).await {
        Ok(_) => success_message("Source deleted successfully"),
        Err(e) => error_response(e),
    }
}

/// Get source status with statistics
pub async fn get_source_status_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::get_source_status(state.db.pool(), source_id).await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Initiate OAuth authorization flow
pub async fn oauth_authorize_handler(
    Path(provider): Path<String>,
    Query(params): Query<crate::api::OAuthAuthorizeRequest>,
) -> Response {
    match crate::api::initiate_oauth_flow(&provider, params.redirect_uri, params.state).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Handle OAuth callback
pub async fn oauth_callback_handler(
    State(state): State<AppState>,
    Query(params): Query<crate::api::OAuthCallbackParams>,
) -> Response {
    match crate::api::handle_oauth_callback(state.db.pool(), &params).await {
        Ok(source) => (StatusCode::CREATED, Json(source)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Create a source manually
pub async fn create_source_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateSourceRequest>,
) -> Response {
    match crate::api::create_source(state.db.pool(), request).await {
        Ok(source) => (StatusCode::CREATED, Json(source)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Register a device as a source
pub async fn register_device_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::RegisterDeviceRequest>,
) -> Response {
    match crate::api::register_device(state.db.pool(), request).await {
        Ok(source) => (StatusCode::CREATED, Json(source)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// List all streams for a source
pub async fn list_streams_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::list_source_streams(state.db.pool(), source_id).await {
        Ok(streams) => (StatusCode::OK, Json(streams)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Get stream details
pub async fn get_stream_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
) -> Response {
    match crate::api::get_stream_info(state.db.pool(), source_id, &stream_name).await {
        Ok(stream) => (StatusCode::OK, Json(stream)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Enable a stream
pub async fn enable_stream_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
    Json(request): Json<crate::api::EnableStreamRequest>,
) -> Response {
    match crate::api::enable_stream(
        state.db.pool(),
        &*state.storage,
        state.stream_writer.clone(),
        source_id,
        &stream_name,
        request.config,
    )
    .await
    {
        Ok(stream) => (StatusCode::OK, Json(stream)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Disable a stream
pub async fn disable_stream_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
) -> Response {
    match crate::api::disable_stream(state.db.pool(), source_id, &stream_name).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Stream disabled successfully"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Update stream configuration
pub async fn update_stream_config_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
    Json(request): Json<crate::api::UpdateStreamConfigRequest>,
) -> Response {
    match crate::api::update_stream_config(state.db.pool(), source_id, &stream_name, request.config)
        .await
    {
        Ok(stream) => (StatusCode::OK, Json(stream)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Update stream schedule
pub async fn update_stream_schedule_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
    Json(request): Json<crate::api::UpdateStreamScheduleRequest>,
) -> Response {
    match crate::api::update_stream_schedule(
        state.db.pool(),
        source_id,
        &stream_name,
        request.cron_schedule,
    )
    .await
    {
        Ok(stream) => (StatusCode::OK, Json(stream)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Trigger a manual sync for a stream (async job-based)
pub async fn sync_stream_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
    Json(request): Json<Option<SyncStreamRequest>>,
) -> Response {
    // Parse sync mode from request (default to incremental)
    let sync_mode = request.and_then(|r| {
        r.sync_mode.map(|m| match m.as_str() {
            "full_refresh" => crate::sources::base::SyncMode::FullRefresh,
            _ => crate::sources::base::SyncMode::incremental(None),
        })
    });

    // Use the new async job-based sync
    match crate::api::trigger_stream_sync(
        state.db.pool(),
        &*state.storage,
        state.stream_writer.clone(),
        source_id,
        &stream_name,
        sync_mode,
    )
    .await
    {
        Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
        Err(e) => {
            let status = if e.to_string().contains("already has an active sync") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(serde_json::json!({
                    "error": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// Request for syncing a stream
#[derive(Debug, Deserialize)]
pub struct SyncStreamRequest {
    pub sync_mode: Option<String>,
}

/// Trigger a transform job for a stream
///
/// POST /api/sources/:source_id/transforms/:stream_name
pub async fn trigger_transform_handler(
    State(_state): State<AppState>,
    Path((_source_id, stream_name)): Path<(Uuid, String)>,
) -> Response {
    // Manual transform triggers are deprecated in the direct transform architecture.
    // Transforms are now automatically triggered after sync jobs complete.
    error_response(crate::error::Error::InvalidInput(format!(
        "Manual transform triggers are not supported. \
         Transforms are automatically triggered after sync jobs complete. \
         To transform data for '{}', run a sync job instead.",
        stream_name
    )))
}

// ============================================================================
// Catalog/Registry API
// ============================================================================

/// Simplified catalog source for frontend display
#[derive(Debug, Serialize)]
pub struct CatalogSource {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub auth_type: String,
    pub stream_count: usize,
    pub icon: Option<String>,
}

/// List all available source types from the registry
pub async fn list_catalog_sources_handler() -> Response {
    let sources = crate::registry::list_sources();

    let catalog: Vec<CatalogSource> = sources
        .iter()
        .map(|s| CatalogSource {
            name: s.name.to_string(),
            display_name: s.display_name.to_string(),
            description: s.description.to_string(),
            auth_type: format!("{:?}", s.auth_type).to_lowercase(),
            stream_count: s.streams.len(),
            icon: s.icon.map(|i| i.to_string()),
        })
        .collect();

    (StatusCode::OK, Json(catalog)).into_response()
}

// ============================================================================
// Ontologies API

/// List available ontology tables
pub async fn list_available_ontologies_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::ontologies::list_available_ontologies(state.db.pool()).await)
}

/// Get ontologies overview with record counts and samples
pub async fn get_ontologies_overview_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::ontologies::get_ontologies_overview(state.db.pool()).await)
}

// ============================================================================
// Jobs API
// ============================================================================

/// Get job status by ID
pub async fn get_job_handler(State(state): State<AppState>, Path(job_id): Path<Uuid>) -> Response {
    match crate::api::get_job_status(state.db.pool(), job_id).await {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Query jobs with filters
#[derive(Debug, Deserialize)]
pub struct QueryJobsParams {
    pub source_id: Option<Uuid>,
    pub status: Option<String>, // Comma-separated list
    pub limit: Option<i64>,
}

pub async fn query_jobs_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryJobsParams>,
) -> Response {
    // Parse comma-separated status list
    let statuses = params.status.map(|s| {
        s.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>()
    });

    let request = crate::api::QueryJobsRequest {
        source_id: params.source_id,
        status: statuses,
        limit: params.limit,
    };

    match crate::api::query_jobs(state.db.pool(), request).await {
        Ok(jobs) => (StatusCode::OK, Json(jobs)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Cancel a running job
pub async fn cancel_job_handler(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> Response {
    match crate::api::cancel_job(state.db.pool(), job_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Job cancelled successfully"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Get job history for a specific stream
#[derive(Debug, Deserialize)]
pub struct StreamJobsParams {
    pub limit: Option<i64>,
}

pub async fn get_stream_jobs_handler(
    State(state): State<AppState>,
    Path((source_id, stream_name)): Path<(Uuid, String)>,
    Query(params): Query<StreamJobsParams>,
) -> Response {
    let limit = params.limit.unwrap_or(10);

    match crate::api::get_job_history(state.db.pool(), source_id, &stream_name, limit).await {
        Ok(jobs) => (StatusCode::OK, Json(jobs)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

// ============================================================================
// Device Pairing Endpoints
// ============================================================================

/// Request to initiate device pairing
#[derive(Debug, Deserialize)]
pub struct InitiatePairingRequest {
    pub device_type: String,
    pub name: String,
}

/// Response when pairing is initiated
#[derive(Debug, Serialize)]
pub struct InitiatePairingResponse {
    pub source_id: Uuid,
    pub code: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Initiate device pairing by generating a pairing code
pub async fn initiate_device_pairing_handler(
    State(state): State<AppState>,
    Json(request): Json<InitiatePairingRequest>,
) -> Response {
    match crate::api::initiate_device_pairing(state.db.pool(), &request.device_type, &request.name)
        .await
    {
        Ok(pairing) => (
            StatusCode::OK,
            Json(InitiatePairingResponse {
                source_id: pairing.source_id,
                code: pairing.code,
                expires_at: pairing.expires_at,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Request to complete device pairing
#[derive(Debug, Deserialize)]
pub struct CompletePairingRequest {
    pub code: String,
    pub device_info: crate::DeviceInfo,
}

/// Response when pairing is completed
#[derive(Debug, Serialize)]
pub struct CompletePairingResponse {
    pub source_id: Uuid,
    pub device_token: String,
    pub available_streams: Vec<crate::registry::RegisteredStream>,
}

/// Complete device pairing with a valid pairing code
pub async fn complete_device_pairing_handler(
    State(state): State<AppState>,
    Json(request): Json<CompletePairingRequest>,
) -> Response {
    match crate::api::complete_device_pairing(state.db.pool(), &request.code, request.device_info)
        .await
    {
        Ok(completed) => (
            StatusCode::OK,
            Json(CompletePairingResponse {
                source_id: completed.source_id,
                device_token: completed.device_token,
                available_streams: completed.available_streams,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Response for pairing status
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PairingStatusResponse {
    Pending,
    Active { device_info: crate::DeviceInfo },
    Revoked,
}

/// Check the status of a device pairing
pub async fn check_pairing_status_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::check_pairing_status(state.db.pool(), source_id).await {
        Ok(status) => {
            let response = match status {
                crate::PairingStatus::Pending => PairingStatusResponse::Pending,
                crate::PairingStatus::Active(info) => {
                    PairingStatusResponse::Active { device_info: info }
                }
                crate::PairingStatus::Revoked => PairingStatusResponse::Revoked,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Response for pending pairings
#[derive(Debug, Serialize)]
pub struct PendingPairingsResponse {
    pub pairings: Vec<PendingPairingItem>,
}

#[derive(Debug, Serialize)]
pub struct PendingPairingItem {
    pub source_id: Uuid,
    pub name: String,
    pub device_type: String,
    pub code: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List all pending device pairings
pub async fn list_pending_pairings_handler(State(state): State<AppState>) -> Response {
    match crate::api::list_pending_pairings(state.db.pool()).await {
        Ok(pairings) => {
            let items = pairings
                .into_iter()
                .map(|p| PendingPairingItem {
                    source_id: p.source_id,
                    name: p.name,
                    device_type: p.device_type,
                    code: p.code,
                    expires_at: p.expires_at,
                    created_at: p.created_at,
                })
                .collect();
            (
                StatusCode::OK,
                Json(PendingPairingsResponse { pairings: items }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Health check endpoint for devices to validate their authentication
///
/// This lightweight endpoint allows devices to verify their token is still valid
/// without creating any side effects. Used for startup validation and periodic health checks.
pub async fn device_health_check_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Extract device token from Authorization header or X-Device-Token header
    let token = if let Some(value) = headers.get(axum::http::header::AUTHORIZATION) {
        let auth_str = value.to_str().unwrap_or("");
        if let Some(t) = auth_str.strip_prefix("Bearer ") {
            t.to_string()
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid Authorization header format. Expected: Bearer <token>"
                })),
            )
                .into_response();
        }
    } else if let Some(value) = headers.get("X-Device-Token") {
        value.to_str().unwrap_or("").to_string()
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Missing authentication header. Provide Authorization: Bearer <token> or X-Device-Token: <token>"
            })),
        )
            .into_response();
    };

    // Validate the device token
    match crate::api::device_pairing::validate_device_token(state.db.pool(), &token).await {
        Ok(source_id) => {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "active",
                    "source_id": source_id,
                })),
            )
                .into_response()
        }
        Err(_) => {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid or revoked device token"
                })),
            )
                .into_response()
        }
    }
}

/// Verify a device token and return configuration status
///
/// This is called by devices that already have a device_token
/// to check if streams have been configured
pub async fn verify_device_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Extract Bearer token from Authorization header
    let token = match headers.get(axum::http::header::AUTHORIZATION) {
        Some(value) => {
            let auth_str = value.to_str().unwrap_or("");
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                token
            } else {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "Missing or invalid Authorization header. Expected: Bearer <token>"
                    })),
                )
                    .into_response();
            }
        }
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing Authorization header"
                })),
            )
                .into_response();
        }
    };

    match crate::api::device_pairing::verify_device(state.db.pool(), token).await {
        Ok(verified) => {
            let response = serde_json::json!({
                "source_id": verified.source_id,
                "configuration_complete": verified.configuration_complete,
                "enabled_streams": verified.enabled_streams.iter().map(|s| {
                    serde_json::json!({
                        "stream_name": s.stream_name,
                        "display_name": s.display_name,
                        "description": s.description,
                        "is_enabled": s.is_enabled,
                        "config": s.config,
                        "supports_incremental": s.supports_incremental,
                        "default_cron_schedule": s.default_cron_schedule,
                    })
                }).collect::<Vec<_>>(),
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match e {
                crate::Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(serde_json::json!({
                    "error": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

// =============================================================================
// Profile API
// =============================================================================

/// Get user profile
pub async fn get_profile_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::get_profile(state.db.pool()).await)
}

/// Update user profile
pub async fn update_profile_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::UpdateProfileRequest>,
) -> Response {
    api_response(crate::api::update_profile(state.db.pool(), request).await)
}

// =============================================================================
// Assistant Profile API
// =============================================================================

/// Get assistant profile
pub async fn get_assistant_profile_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::get_assistant_profile(state.db.pool()).await)
}

/// Update assistant profile
pub async fn update_assistant_profile_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::UpdateAssistantProfileRequest>,
) -> Response {
    api_response(crate::api::update_assistant_profile(state.db.pool(), request).await)
}

// =============================================================================
// Axiology API - Tasks
// =============================================================================

/// List all active tasks
pub async fn list_tasks_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_tasks(state.db.pool()).await)
}

/// Get a specific task by ID
pub async fn get_task_handler(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_task(state.db.pool(), task_id).await)
}

/// Create a new task
pub async fn create_task_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateTaskRequest>,
) -> Response {
    api_response(crate::api::create_task(state.db.pool(), request).await)
}

/// Update an existing task
pub async fn update_task_handler(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateTaskRequest>,
) -> Response {
    api_response(crate::api::update_task(state.db.pool(), task_id, request).await)
}

/// Delete a task (soft delete)
pub async fn delete_task_handler(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Response {
    match crate::api::delete_task(state.db.pool(), task_id).await {
        Ok(_) => success_message("Task deleted successfully"),
        Err(e) => error_response(e),
    }
}

/// List all distinct tags across temporal pursuits
pub async fn list_tags_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_tags(state.db.pool()).await)
}

// =============================================================================
// Actions API - Initiatives
// =============================================================================

/// List all active initiatives
pub async fn list_initiatives_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_initiatives(state.db.pool()).await)
}

/// Get a specific initiative by ID
pub async fn get_initiative_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_initiative(state.db.pool(), id).await)
}

/// Create a new initiative
pub async fn create_initiative_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateTaskRequest>,
) -> Response {
    api_response(crate::api::create_initiative(state.db.pool(), request).await)
}

/// Update an existing initiative
pub async fn update_initiative_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateTaskRequest>,
) -> Response {
    api_response(crate::api::update_initiative(state.db.pool(), id, request).await)
}

/// Delete an initiative (soft delete)
pub async fn delete_initiative_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match crate::api::delete_initiative(state.db.pool(), id).await {
        Ok(_) => success_message("Initiative deleted successfully"),
        Err(e) => error_response(e),
    }
}

// =============================================================================
// Actions API - Aspirations
// =============================================================================

/// List all active aspirations
pub async fn list_aspirations_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_aspirations(state.db.pool()).await)
}

/// Get a specific aspiration by ID
pub async fn get_aspiration_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_aspiration(state.db.pool(), id).await)
}

/// Create a new aspiration
pub async fn create_aspiration_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateAspirationRequest>,
) -> Response {
    api_response(crate::api::create_aspiration(state.db.pool(), request).await)
}

/// Update an existing aspiration
pub async fn update_aspiration_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateAspirationRequest>,
) -> Response {
    api_response(crate::api::update_aspiration(state.db.pool(), id, request).await)
}

/// Delete an aspiration (soft delete)
pub async fn delete_aspiration_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match crate::api::delete_aspiration(state.db.pool(), id).await {
        Ok(_) => success_message("Aspiration deleted successfully"),
        Err(e) => error_response(e),
    }
}

// =============================================================================
// Axiology API - Temperaments
// =============================================================================

pub async fn list_temperaments_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_temperaments(state.db.pool()).await)
}

pub async fn get_temperament_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_temperament(state.db.pool(), id).await)
}

pub async fn create_temperament_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateSimpleRequest>,
) -> Response {
    api_response(crate::api::create_temperament(state.db.pool(), request).await)
}

pub async fn update_temperament_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateSimpleRequest>,
) -> Response {
    api_response(crate::api::update_temperament(state.db.pool(), id, request).await)
}

pub async fn delete_temperament_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match crate::api::delete_temperament(state.db.pool(), id).await {
        Ok(_) => success_message("Temperament deleted successfully"),
        Err(e) => error_response(e),
    }
}

// =============================================================================
// Axiology API - Virtues
// =============================================================================

pub async fn list_virtues_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_virtues(state.db.pool()).await)
}

pub async fn get_virtue_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_virtue(state.db.pool(), id).await)
}

pub async fn create_virtue_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateSimpleRequest>,
) -> Response {
    api_response(crate::api::create_virtue(state.db.pool(), request).await)
}

pub async fn update_virtue_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateSimpleRequest>,
) -> Response {
    api_response(crate::api::update_virtue(state.db.pool(), id, request).await)
}

pub async fn delete_virtue_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match crate::api::delete_virtue(state.db.pool(), id).await {
        Ok(_) => success_message("Virtue deleted successfully"),
        Err(e) => error_response(e),
    }
}

// =============================================================================
// Axiology API - Vices
// =============================================================================

pub async fn list_vices_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_vices(state.db.pool()).await)
}

pub async fn get_vice_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_vice(state.db.pool(), id).await)
}

pub async fn create_vice_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateSimpleRequest>,
) -> Response {
    api_response(crate::api::create_vice(state.db.pool(), request).await)
}

pub async fn update_vice_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateSimpleRequest>,
) -> Response {
    api_response(crate::api::update_vice(state.db.pool(), id, request).await)
}

pub async fn delete_vice_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match crate::api::delete_vice(state.db.pool(), id).await {
        Ok(_) => success_message("Vice deleted successfully"),
        Err(e) => error_response(e),
    }
}

// =============================================================================
// Axiology API - Values
// =============================================================================

pub async fn list_values_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_values(state.db.pool()).await)
}

pub async fn get_value_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    api_response(crate::api::get_value(state.db.pool(), id).await)
}

pub async fn create_value_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::api::CreateSimpleRequest>,
) -> Response {
    api_response(crate::api::create_value(state.db.pool(), request).await)
}

pub async fn update_value_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::api::UpdateSimpleRequest>,
) -> Response {
    api_response(crate::api::update_value(state.db.pool(), id, request).await)
}

pub async fn delete_value_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match crate::api::delete_value(state.db.pool(), id).await {
        Ok(_) => success_message("Value deleted successfully"),
        Err(e) => error_response(e),
    }
}

// =============================================================================
// Tools API
// =============================================================================

/// List all tools with optional filtering
pub async fn list_tools_handler(
    State(state): State<AppState>,
    Query(query): Query<crate::api::ListToolsQuery>,
) -> Response {
    api_response(crate::api::list_tools(state.db.pool(), query).await)
}

/// Get a specific tool by ID
pub async fn get_tool_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    api_response(crate::api::get_tool(state.db.pool(), id).await)
}

/// Update a tool's metadata
pub async fn update_tool_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<crate::api::UpdateToolRequest>,
) -> Response {
    api_response(crate::api::update_tool(state.db.pool(), id, request).await)
}

// =============================================================================
// Models API
// =============================================================================

/// List all available models
pub async fn list_models_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_models(state.db.pool()).await)
}

/// Get a specific model by ID
pub async fn get_model_handler(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Response {
    api_response(crate::api::get_model(state.db.pool(), &model_id).await)
}

// =============================================================================
// Agents API
// =============================================================================

/// List all available agents
pub async fn list_agents_handler(State(state): State<AppState>) -> Response {
    api_response(crate::api::list_agents(state.db.pool()).await)
}

/// Get a specific agent by ID
pub async fn get_agent_handler(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Response {
    api_response(crate::api::get_agent(state.db.pool(), &agent_id).await)
}
