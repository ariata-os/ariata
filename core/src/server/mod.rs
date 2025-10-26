//! HTTP server for data ingestion and API

mod api;
mod ingest;

use axum::{
    Router,
    routing::{get, post, delete},
    Json,
    response::IntoResponse,
};

use crate::Ariata;
use crate::error::Result;
use self::ingest::AppState;

/// Run the HTTP ingestion server
pub async fn run(client: Ariata, host: &str, port: u16) -> Result<()> {
    // Create app state with database and storage
    let state = AppState {
        db: client.database.clone(),
        storage: client.storage.clone(),
    };

    let app = Router::new()
        // Health check
        .route("/health", get(health))

        // Data ingestion
        .route("/ingest", post(ingest::ingest))

        // Source management API
        .route("/api/sources", get(api::list_sources_handler))
        .route("/api/sources/:id", get(api::get_source_handler))
        .route("/api/sources/:id", delete(api::delete_source_handler))
        .route("/api/sources/:id/status", get(api::get_source_status_handler))
        .route("/api/sources/:id/sync", post(api::sync_source_handler))
        .route("/api/sources/:id/history", get(api::get_sync_history_handler))

        .with_state(state);

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}