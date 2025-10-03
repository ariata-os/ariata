//! OAuth routes for authentication flows

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::oauth::OAuthManager;
use super::ingest::AppState;

/// OAuth authorization request parameters
#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    #[allow(dead_code)]
    pub redirect_uri: Option<String>,
}

/// OAuth callback parameters
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub code: String,
    #[allow(dead_code)]
    pub state: String,
    pub provider: Option<String>,
}

/// OAuth success response
#[derive(Debug, Serialize)]
struct OAuthResponse {
    success: bool,
    provider: String,
    message: String,
}

/// Initiate OAuth authorization flow
pub async fn authorize(
    Path(provider): Path<String>,
    Query(_params): Query<AuthorizeParams>,
    State(state): State<AppState>,
) -> Response {
    // Create OAuth manager
    let mut oauth = OAuthManager::new(state.db.clone());

    // Configure providers (in production, load from config)
    configure_providers(&mut oauth);

    // Generate authorization URL
    match oauth.get_auth_url(&provider) {
        Ok((auth_url, csrf_token)) => {
            // In production, store CSRF token in session/database
            tracing::info!(
                "OAuth flow initiated for {} with CSRF: {}",
                provider,
                csrf_token.secret()
            );

            // Redirect to provider's auth page
            Redirect::to(&auth_url).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to generate auth URL for {}: {}", provider, e);

            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Failed to initiate OAuth: {}", e)
                }))
            ).into_response()
        }
    }
}

/// Handle OAuth callback
pub async fn callback(
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
) -> Response {
    let provider = params.provider.unwrap_or_else(|| {
        // Try to determine provider from state parameter
        // In production, decode state to get provider
        "unknown".to_string()
    });

    // Create OAuth manager
    let mut oauth = OAuthManager::new(state.db.clone());
    configure_providers(&mut oauth);

    // Exchange code for tokens
    match oauth.exchange_code(&provider, params.code).await {
        Ok(credentials) => {
            tracing::info!(
                "OAuth successful for {}: got {} token",
                provider,
                if credentials.refresh_token.is_some() { "refresh" } else { "access" }
            );

            // Return success page or redirect
            (
                StatusCode::OK,
                Json(OAuthResponse {
                    success: true,
                    provider: provider.clone(),
                    message: format!("Successfully connected to {}", provider),
                })
            ).into_response()
        }
        Err(e) => {
            tracing::error!("OAuth callback failed for {}: {}", provider, e);

            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("OAuth failed: {}", e)
                }))
            ).into_response()
        }
    }
}

/// Configure OAuth providers
fn configure_providers(oauth: &mut OAuthManager) {
    // Load from environment variables or config
    if let (Ok(client_id), Ok(client_secret)) = (
        std::env::var("GOOGLE_CLIENT_ID"),
        std::env::var("GOOGLE_CLIENT_SECRET"),
    ) {
        oauth.configure_google(client_id, client_secret);
    }

    if let (Ok(client_id), Ok(client_secret)) = (
        std::env::var("STRAVA_CLIENT_ID"),
        std::env::var("STRAVA_CLIENT_SECRET"),
    ) {
        oauth.configure_strava(client_id, client_secret);
    }
}