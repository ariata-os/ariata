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

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
}

/// List all sources
pub async fn list_sources_handler(State(state): State<AppState>) -> Response {
    match crate::api::list_sources(state.db.pool()).await {
        Ok(sources) => (StatusCode::OK, Json(sources)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Get a specific source by ID
pub async fn get_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::get_source(state.db.pool(), source_id).await {
        Ok(source) => (StatusCode::OK, Json(source)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Pause a source
pub async fn pause_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::pause_source(state.db.pool(), source_id).await {
        Ok(source) => (StatusCode::OK, Json(source)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Resume a source
pub async fn resume_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::resume_source(state.db.pool(), source_id).await {
        Ok(source) => (StatusCode::OK, Json(source)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Delete a source by ID
pub async fn delete_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::delete_source(state.db.pool(), source_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Source deleted successfully"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
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
    match crate::api::enable_stream(state.db.pool(), source_id, &stream_name, request.config).await
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
    let sync_mode = request.and_then(|r| r.sync_mode.map(|m| match m.as_str() {
        "full_refresh" => crate::sources::base::SyncMode::FullRefresh,
        _ => crate::sources::base::SyncMode::incremental(None),
    }));

    // Use the new async job-based sync
    match crate::api::trigger_stream_sync(state.db.pool(), source_id, &stream_name, sync_mode).await {
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
// Jobs API
// ============================================================================

/// Get job status by ID
pub async fn get_job_handler(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> Response {
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
    pub status: Option<String>,  // Comma-separated list
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
