//! virtues-api - Open Source API Proxy with Budget Enforcement
//!
//! A "prepaid arcade card" model for AI API access:
//! - Authenticate the device bearer, look up its entitlement in Postgres
//! - Enforce per-call cap, daily ceiling, and wallet balance on each charge
//! - Forward requests upstream (Vercel AI Gateway, Exa, Places, Unsplash)
//!
//! Budget state lives entirely in the `accounts` table (credited by atlas on
//! subscription renewal + top-up), with the append-only `ledger` as the source
//! of truth. There is no RAM-budget mode and no Atlas hydration — the Postgres
//! pool is required at boot.
//!
//! This code is open source so users can verify we don't log their data.

mod auth;
mod bearer_auth;
mod blocklist;
mod catalog;
mod config;
mod db;
mod dev_seed;
mod entitlement;
mod providers;
mod proxy;
mod routes;
mod sweeper;

use anyhow::{Context, Result};
use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::blocklist::Blocklist;
use crate::config::Config;

/// Shared application state
pub struct AppState {
    pub config: Arc<Config>,
    pub http_client: reqwest::Client,
    /// Postgres pool backing the accounts/ledger/device_keys/blocklist tables.
    /// The only budget store — required at boot (`VIRTUES_API_DATABASE_URL`).
    pub db: sqlx::PgPool,
    /// Behavioral abuse blocklist (in-memory hot path, DB-snapshotted).
    pub blocklist: Blocklist,
    /// Live model facts from the gateway (prices, context windows, existence),
    /// refreshed hourly. The ONLY source of model pricing — see `catalog.rs`.
    pub catalog: catalog::Catalog,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install the ring CryptoProvider as the process-wide default. Without this,
    // `reqwest::Client` panics at first use ("No provider set") because rustls 0.23
    // requires the provider installed before any TLS work. Mirrors atlas + virtues-core.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls ring CryptoProvider");

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "virtues_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from root .env file (shared across all apps)
    // Try workspace root first, then fall back to current directory
    let root_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(".env"));

    if let Some(env_path) = root_env {
        if env_path.exists() {
            dotenvy::from_path(&env_path).ok();
            tracing::debug!("Loaded .env from {}", env_path.display());
        }
    }
    // Also load local .env if present (for overrides)
    dotenvy::dotenv().ok();
    let config = Arc::new(Config::from_env()?);

    tracing::info!("Starting virtues-api on port {}", config.port);

    // Log AI Gateway configuration
    tracing::info!(
        "Vercel AI Gateway: url={}",
        config.ai_gateway_url
    );

    // Log external services configuration. (Plaid/OAuth providers read their
    // own env in routes/oauth.rs, so they're not surfaced here.)
    tracing::info!(
        // Web search runs on the AI Gateway key, not a search vendor key —
        // there is no separate search credential to report any more.
        "External services: GooglePlaces={}",
        config.google_api_key.is_some(),
    );

    // Build HTTP client for embeddings and other direct API calls
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 min for long completions
        .build()?;

    // DB connect + migrate. Required — the accounts/ledger schema is the only
    // budget store (there is no RAM-budget fallback), so fail fast at boot
    // rather than silently 503'ing every metered call.
    let db_url = std::env::var("VIRTUES_API_DATABASE_URL")
        .context("VIRTUES_API_DATABASE_URL is required")?;
    let db = db::connect_and_migrate(&db_url).await?;

    // Behavioral blocklist: in-memory hot path, snapshotted from the table.
    let blocklist = Blocklist::from_env();
    blocklist.load_snapshot(&db).await;
    blocklist.spawn_pruner();

    // Background housekeeping: reclaim long-expired accounts.
    sweeper::spawn(db.clone());

    // Dev-only: fund a known account + device key so a local virtues-api
    // accepts calls without atlas. Gated to ENVIRONMENT=dev so it can never
    // fire in production.
    let is_dev = std::env::var("ENVIRONMENT").map(|v| v == "dev").unwrap_or(false);
    if is_dev {
        dev_seed::seed_dev_account(&db).await?;
    }

    // Build shared state
    // Model facts (prices, context windows, which ids actually exist) come from
    // the gateway, never from a table we maintain. Fetched now, refreshed
    // hourly. A failed fetch is logged, not fatal — see catalog.rs.
    let catalog = catalog::spawn(http_client.clone(), config.ai_gateway_url.clone()).await;

    let state = Arc::new(AppState {
        config,
        http_client,
        db,
        blocklist,
        catalog,
    });

    // Build router
    let app = Router::new()
        // Health check (no auth required)
        .route("/health", get(routes::health::health_check))
        .route("/ready", get(routes::health::readiness_check))
        // Internal atlas → virtues-api surface (device register + credit)
        .merge(routes::internal::router())
        // OAuth proxy (google/notion/strava/plaid) — folded in from the Node
        // oauth-proxy (WS-4). Mounted at root: /{provider}/start|callback|...
        .merge(routes::oauth::router())
        // Bearer-auth account endpoints (whoami + usage)
        .merge(routes::bearer_test::router())
        // Bearer-auth + entitlement charge: Places, Exa, Unsplash, AI.
        // AI (`/v1/ai/*`) covers both streaming and non-streaming chat; the
        // charge fires from a callback once the upstream usage is known.
        .merge(routes::places::router())
        .merge(routes::search::router())
        .merge(routes::unsplash::router())
        // Plaid bank-data proxy (/v1/services/plaid/*): keeps the master Plaid
        // secret server-side; the box sends only per-user access_tokens.
        .merge(routes::plaid::router())
        .merge(routes::ai::router())
        // Middleware
        .layer(TraceLayer::new_for_http())
        // F2: no CORS. virtues-api is a server-to-server sidecar (the home box
        // calls it with a Bearer or X-Internal-Secret header). No browser
        // should ever talk to it. The previous `allow_origin: Any +
        // allow_headers: Any` made any public web page a potential confused
        // deputy if a credential ever leaked into a browser context.
        .with_state(state.clone());

    // Start server
    let addr = format!("0.0.0.0:{}", state.config.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("virtues-api listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
