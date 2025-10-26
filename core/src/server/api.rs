//! REST API handlers for source management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
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

/// Trigger a sync for a source
pub async fn sync_source_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
) -> Response {
    match crate::api::sync_source(state.db.pool(), source_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Sync completed successfully"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": e.to_string(),
                "message": "Direct sync not implemented. Use scheduler for automatic syncs."
            })),
        )
            .into_response(),
    }
}

/// Get sync history for a source
pub async fn get_sync_history_handler(
    State(state): State<AppState>,
    Path(source_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    match crate::api::get_sync_history(state.db.pool(), source_id, pagination.limit).await {
        Ok(logs) => (StatusCode::OK, Json(logs)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}
