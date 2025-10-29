//! HTTP server for data ingestion and API

mod api;
mod ingest;

use axum::{
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};

use self::ingest::AppState;
use crate::error::Result;
use crate::Ariata;

/// Run the HTTP ingestion server with integrated scheduler
pub async fn run(client: Ariata, host: &str, port: u16) -> Result<()> {
    // Start the scheduler in the background
    let db_pool = client.database.pool().clone();
    let _scheduler_handle = tokio::spawn(async move {
        match crate::Scheduler::new(db_pool).await {
            Ok(sched) => {
                if let Err(e) = sched.start().await {
                    tracing::warn!("Failed to start scheduler: {}", e);
                } else {
                    tracing::info!("Scheduler started successfully");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create scheduler: {}", e);
            }
        }
    });

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
        // OAuth flow
        .route(
            "/api/sources/:provider/authorize",
            post(api::oauth_authorize_handler),
        )
        .route("/api/sources/callback", get(api::oauth_callback_handler))
        // Source management API
        .route("/api/sources", get(api::list_sources_handler))
        .route("/api/sources", post(api::create_source_handler))
        .route(
            "/api/sources/register-device",
            post(api::register_device_handler),
        )
        .route("/api/sources/:id", get(api::get_source_handler))
        .route("/api/sources/:id", delete(api::delete_source_handler))
        .route("/api/sources/:id/pause", post(api::pause_source_handler))
        .route("/api/sources/:id/resume", post(api::resume_source_handler))
        .route(
            "/api/sources/:id/status",
            get(api::get_source_status_handler),
        )
        // Stream management API
        .route("/api/sources/:id/streams", get(api::list_streams_handler))
        .route(
            "/api/sources/:id/streams/:name",
            get(api::get_stream_handler),
        )
        .route(
            "/api/sources/:id/streams/:name/enable",
            post(api::enable_stream_handler),
        )
        .route(
            "/api/sources/:id/streams/:name",
            delete(api::disable_stream_handler),
        )
        .route(
            "/api/sources/:id/streams/:name/config",
            put(api::update_stream_config_handler),
        )
        .route(
            "/api/sources/:id/streams/:name/schedule",
            put(api::update_stream_schedule_handler),
        )
        .route(
            "/api/sources/:id/streams/:name/sync",
            post(api::sync_stream_handler),
        )
        // Catalog/Registry API
        .route("/api/catalog/sources", get(api::list_catalog_sources_handler))
        // Jobs API
        .route("/api/jobs/:id", get(api::get_job_handler))
        .route("/api/jobs", get(api::query_jobs_handler))
        .route("/api/jobs/:id/cancel", post(api::cancel_job_handler))
        .with_state(state);

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);

    // Run the server (this blocks forever until shutdown)
    axum::serve(listener, app).await?;

    // Note: scheduler runs in background and will stop when the process exits
    // The handle is dropped here, but the task continues running

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
