//! HTTP server for data ingestion

mod ingest;
mod oauth;

use axum::{
    Router,
    routing::{get, post},
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
        .route("/health", get(health))
        .route("/ingest", post(ingest::ingest))  // New unified ingestion endpoint
        .route("/oauth/authorize/:provider", get(oauth::authorize))
        .route("/oauth/callback", get(oauth::callback))
        .with_state(state);

    let addr = format!("{}:{}", host, port);
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