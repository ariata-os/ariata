//! HTTP server for data ingestion and API

mod api;
mod ingest;

use axum::{
    extract::DefaultBodyLimit,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};

use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

use self::ingest::AppState;
use crate::error::{Error, Result};
use crate::mcp::{http::add_mcp_routes, AriataMcpServer};
use crate::storage::encryption;
use crate::storage::stream_writer::StreamWriter;
use crate::Ariata;

/// Run the HTTP ingestion server with integrated scheduler
pub async fn run(client: Ariata, host: &str, port: u16) -> Result<()> {
    // Validate required environment variables early
    validate_environment()?;

    // Initialize StreamWriter (simple in-memory buffer)
    let stream_writer = StreamWriter::new();
    let stream_writer_arc = Arc::new(Mutex::new(stream_writer));

    // Start the scheduler in the background
    let db_pool = client.database.pool().clone();
    let storage = (*client.storage).clone();
    let scheduler_stream_writer = stream_writer_arc.clone();
    let _scheduler_handle = tokio::spawn(async move {
        match crate::Scheduler::new(db_pool, storage, scheduler_stream_writer).await {
            Ok(sched) => {
                if let Err(e) = sched.start().await {
                    tracing::warn!("Failed to start scheduler: {}", e);
                } else {
                    tracing::info!("Scheduler started successfully");

                    // Schedule location visit clustering job
                    if let Err(e) = sched.schedule_location_clustering_job().await {
                        tracing::warn!("Failed to schedule location clustering job: {}", e);
                    }

                    // Keep scheduler alive - it will be dropped when the server shuts down
                    // The JobScheduler runs background tasks that need to stay active
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create scheduler: {}", e);
            }
        }
    });

    // Create app state with database, storage, and stream writer
    let state = AppState {
        db: client.database.clone(),
        storage: client.storage.clone(),
        stream_writer: stream_writer_arc.clone(),
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
        // Device pairing endpoints
        .route(
            "/api/devices/pairing/initiate",
            post(api::initiate_device_pairing_handler),
        )
        .route(
            "/api/devices/pairing/complete",
            post(api::complete_device_pairing_handler),
        )
        .route(
            "/api/devices/pairing/:source_id",
            get(api::check_pairing_status_handler),
        )
        .route(
            "/api/devices/pending-pairings",
            get(api::list_pending_pairings_handler),
        )
        .route("/api/devices/verify", post(api::verify_device_handler))
        .route("/api/devices/health", get(api::device_health_check_handler))
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
            "/api/sources/:id/streams/:name/disable",
            post(api::disable_stream_handler),
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
        .route(
            "/api/sources/:id/streams/:name/jobs",
            get(api::get_stream_jobs_handler),
        )
        .route(
            "/api/sources/:id/transforms/:name",
            post(api::trigger_transform_handler),
        )
        // Catalog/Registry API
        .route(
            "/api/catalog/sources",
            get(api::list_catalog_sources_handler),
        )
        // Ontologies API
        .route(
            "/api/ontologies/available",
            get(api::list_available_ontologies_handler),
        )
        .route(
            "/api/ontologies/overview",
            get(api::get_ontologies_overview_handler),
        )
        // Jobs API
        .route("/api/jobs/:id", get(api::get_job_handler))
        .route("/api/jobs", get(api::query_jobs_handler))
        .route("/api/jobs/:id/cancel", post(api::cancel_job_handler))
        // Profile API
        .route("/api/profile", get(api::get_profile_handler))
        .route("/api/profile", put(api::update_profile_handler))
        // Assistant Profile API
        .route(
            "/api/assistant-profile",
            get(api::get_assistant_profile_handler),
        )
        .route(
            "/api/assistant-profile",
            put(api::update_assistant_profile_handler),
        )
        // Tools API
        .route("/api/tools", get(api::list_tools_handler))
        .route("/api/tools/:id", get(api::get_tool_handler))
        .route("/api/tools/:id", put(api::update_tool_handler))
        // Models API
        .route("/api/models", get(api::list_models_handler))
        .route("/api/models/:id", get(api::get_model_handler))
        // Agents API
        .route("/api/agents", get(api::list_agents_handler))
        .route("/api/agents/:id", get(api::get_agent_handler))
        // Actions API - Tasks
        .route("/api/actions/tasks", get(api::list_tasks_handler))
        .route("/api/actions/tasks", post(api::create_task_handler))
        .route("/api/actions/tasks/:id", get(api::get_task_handler))
        .route("/api/actions/tasks/:id", put(api::update_task_handler))
        .route("/api/actions/tasks/:id", delete(api::delete_task_handler))
        // Actions API - Initiatives
        .route("/api/actions/initiatives", get(api::list_initiatives_handler))
        .route("/api/actions/initiatives", post(api::create_initiative_handler))
        .route("/api/actions/initiatives/:id", get(api::get_initiative_handler))
        .route("/api/actions/initiatives/:id", put(api::update_initiative_handler))
        .route("/api/actions/initiatives/:id", delete(api::delete_initiative_handler))
        // Actions API - Aspirations
        .route("/api/actions/aspirations", get(api::list_aspirations_handler))
        .route("/api/actions/aspirations", post(api::create_aspiration_handler))
        .route("/api/actions/aspirations/:id", get(api::get_aspiration_handler))
        .route("/api/actions/aspirations/:id", put(api::update_aspiration_handler))
        .route("/api/actions/aspirations/:id", delete(api::delete_aspiration_handler))
        // Actions API - Tags
        .route("/api/actions/tags", get(api::list_tags_handler))
        // Axiology API - Temperaments
        .route("/api/axiology/temperaments", get(api::list_temperaments_handler))
        .route("/api/axiology/temperaments", post(api::create_temperament_handler))
        .route("/api/axiology/temperaments/:id", get(api::get_temperament_handler))
        .route("/api/axiology/temperaments/:id", put(api::update_temperament_handler))
        .route("/api/axiology/temperaments/:id", delete(api::delete_temperament_handler))
        // Axiology API - Virtues
        .route("/api/axiology/virtues", get(api::list_virtues_handler))
        .route("/api/axiology/virtues", post(api::create_virtue_handler))
        .route("/api/axiology/virtues/:id", get(api::get_virtue_handler))
        .route("/api/axiology/virtues/:id", put(api::update_virtue_handler))
        .route("/api/axiology/virtues/:id", delete(api::delete_virtue_handler))
        // Axiology API - Vices
        .route("/api/axiology/vices", get(api::list_vices_handler))
        .route("/api/axiology/vices", post(api::create_vice_handler))
        .route("/api/axiology/vices/:id", get(api::get_vice_handler))
        .route("/api/axiology/vices/:id", put(api::update_vice_handler))
        .route("/api/axiology/vices/:id", delete(api::delete_vice_handler))
        // Axiology API - Values
        .route("/api/axiology/values", get(api::list_values_handler))
        .route("/api/axiology/values", post(api::create_value_handler))
        .route("/api/axiology/values/:id", get(api::get_value_handler))
        .route("/api/axiology/values/:id", put(api::update_value_handler))
        .route("/api/axiology/values/:id", delete(api::delete_value_handler))
        .with_state(state.clone())
        .layer(DefaultBodyLimit::disable()) // Disable default 2MB limit
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024)); // Set 20MB limit for audio files

    // Add MCP routes to the same server
    let mcp_server = AriataMcpServer::new(client.database.pool().clone());
    let app = add_mcp_routes(app, mcp_server);

    tracing::info!("MCP endpoint enabled at /mcp");

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);

    // Run the server (this blocks forever until shutdown)
    axum::serve(listener, app).await?;

    // Note: No flush needed on shutdown - StreamWriter is in-memory only now.
    // Archive jobs handle S3 uploads asynchronously.
    tracing::info!("Server shutting down gracefully");

    // Note: scheduler runs in background and will stop when the process exits
    // The handle is dropped here, but the task continues running

    Ok(())
}

/// Validate required environment variables at startup
fn validate_environment() -> Result<()> {
    // Check encryption key
    let master_key = env::var("STREAM_ENCRYPTION_MASTER_KEY")
        .map_err(|_| Error::Configuration(
            "STREAM_ENCRYPTION_MASTER_KEY environment variable is required. Generate with: openssl rand -hex 32".into()
        ))?;

    // Validate key format (should be 64 hex characters = 32 bytes)
    if master_key.len() != 64 {
        return Err(Error::Configuration(format!(
            "STREAM_ENCRYPTION_MASTER_KEY must be 64 hex characters (32 bytes), got {} characters",
            master_key.len()
        )));
    }

    // Try to parse to ensure it's valid hex
    encryption::parse_master_key_hex(&master_key).map_err(|e| {
        Error::Configuration(format!(
            "Invalid STREAM_ENCRYPTION_MASTER_KEY format: {}",
            e
        ))
    })?;

    // Check S3 config if S3_BUCKET is set
    if let Ok(bucket) = env::var("S3_BUCKET") {
        if bucket.is_empty() {
            return Err(Error::Configuration("S3_BUCKET is set but empty".into()));
        }

        // Require credentials when using S3
        if env::var("S3_ACCESS_KEY").is_err() {
            return Err(Error::Configuration(
                "S3_ACCESS_KEY is required when S3_BUCKET is set".into(),
            ));
        }

        if env::var("S3_SECRET_KEY").is_err() {
            return Err(Error::Configuration(
                "S3_SECRET_KEY is required when S3_BUCKET is set".into(),
            ));
        }

        tracing::debug!("S3 configuration validated for bucket: {}", bucket);
    }

    tracing::debug!("Environment validation passed");
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
