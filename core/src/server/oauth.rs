//! OAuth routes for authentication flows using OAuth proxy

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// OAuth proxy client would be used here for production
// For now, using placeholder types
#[derive(Debug)]
struct OAuthProxyClient;

impl OAuthProxyClient {
    fn new() -> Self { Self }

    fn get_google_auth_url(&self, _state: Option<&str>) -> Result<String, String> {
        Err("OAuth proxy not implemented".to_string())
    }

    fn get_notion_auth_url(&self, _state: Option<&str>) -> Result<String, String> {
        Err("OAuth proxy not implemented".to_string())
    }

    fn get_strava_auth_url(&self, _state: Option<&str>) -> Result<String, String> {
        Err("OAuth proxy not implemented".to_string())
    }

    async fn exchange_code(&self, provider: &str, _code: &str) -> Result<OAuthTokens, String> {
        Ok(OAuthTokens {
            provider: provider.to_string(),
            access_token: "placeholder".to_string(),
            refresh_token: None,
            expires_in: None,
        })
    }

    async fn store_tokens(&self, _db: &sqlx::PgPool, _source_name: &str, _tokens: &OAuthTokens) -> Result<Uuid, String> {
        Ok(Uuid::new_v4())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuthTokens {
    provider: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}
use super::ingest::AppState;

/// OAuth authorization request parameters
#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    #[allow(dead_code)]
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
}

/// OAuth callback parameters from the proxy
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub provider: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// OAuth success response
#[derive(Debug, Serialize)]
struct OAuthResponse {
    success: bool,
    provider: String,
    source_id: Option<String>,
    message: String,
}

/// Initiate OAuth authorization flow through the proxy
pub async fn authorize(
    Path(provider): Path<String>,
    Query(params): Query<AuthorizeParams>,
    State(_state): State<AppState>,
) -> Response {
    // Create OAuth proxy client
    let proxy_client = OAuthProxyClient::new();

    // Generate authorization URL based on provider
    let auth_url = match provider.as_str() {
        "google" => proxy_client.get_google_auth_url(params.state.as_deref()),
        "notion" => proxy_client.get_notion_auth_url(params.state.as_deref()),
        "strava" => proxy_client.get_strava_auth_url(params.state.as_deref()),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Unknown provider: {}", provider)
                }))
            ).into_response();
        }
    };

    // Handle error from auth URL generation
    let auth_url = match auth_url {
        Ok(url) => url,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": e
                }))
            ).into_response();
        }
    };

    tracing::info!("Redirecting to OAuth proxy for {}: {}", provider, auth_url);

    // Redirect to OAuth proxy
    Redirect::to(&auth_url).into_response()
}

/// Handle OAuth callback from the proxy
pub async fn callback(
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
) -> Response {
    // Check for errors first
    if let Some(error_msg) = params.error {
        tracing::error!("OAuth callback error: {}", error_msg);
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("OAuth failed: {}", error_msg)
            }))
        ).into_response();
    }

    // Extract tokens from callback
    let tokens = match extract_tokens_from_params(&params) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to extract tokens: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid callback parameters: {}", e)
                }))
            ).into_response();
        }
    };

    // Create OAuth proxy client
    let proxy_client = OAuthProxyClient::new();

    // Determine source name
    let source_name = format!("{} Account",
        tokens.provider.chars().next().unwrap_or('?').to_uppercase().to_string()
        + &tokens.provider[1..]);

    // Store tokens in database
    match proxy_client.store_tokens(state.db.pool(), &source_name, &tokens).await {
        Ok(source_id) => {
            tracing::info!(
                "OAuth successful for {}: stored as source {} ({})",
                tokens.provider, source_id, source_name
            );

            // Return success response
            (
                StatusCode::OK,
                Json(OAuthResponse {
                    success: true,
                    provider: tokens.provider.clone(),
                    source_id: Some(source_id.to_string()),
                    message: format!("Successfully connected to {}", tokens.provider),
                })
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to store OAuth tokens: {}", e);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to store credentials: {}", e)
                }))
            ).into_response()
        }
    }
}

/// Extract tokens from callback parameters
fn extract_tokens_from_params(params: &CallbackParams) -> Result<OAuthTokens, String> {
    let access_token = params.access_token.as_ref()
        .ok_or("Missing access_token")?
        .clone();

    let provider = params.provider.as_ref()
        .ok_or("Missing provider")?
        .clone();

    Ok(OAuthTokens {
        access_token,
        refresh_token: params.refresh_token.clone(),
        expires_in: params.expires_in,
        provider,
    })
}

/// Connect a Google Calendar source
pub async fn connect_google(
    State(state): State<AppState>,
) -> Response {
    // Check if we already have a Google source
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sources WHERE type = 'google' AND is_active = true"
    )
    .fetch_one(state.db.pool())
    .await;

    match existing {
        Ok(count) if count > 0 => {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Google Calendar already connected",
                    "action": "use_existing"
                }))
            ).into_response()
        }
        _ => {
            // Redirect to OAuth flow
            authorize(
                Path("google".to_string()),
                Query(AuthorizeParams {
                    redirect_uri: None,
                    state: Some(Uuid::new_v4().to_string()),
                }),
                State(state)
            ).await
        }
    }
}