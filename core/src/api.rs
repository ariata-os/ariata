//! Library-level API functions for programmatic access
//!
//! These functions provide a simple, library-first interface for OAuth flows
//! and data synchronization, suitable for use from Python wrappers or other bindings.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::env;

use crate::{
    error::{Error, Result},
    sources::google::GoogleCalendarSync,
    oauth::token_manager::{TokenManager, OAuthProxyConfig},
};

/// Generate a Google OAuth authorization URL
///
/// This creates the URL users should visit to authorize access to their Google account.
/// Returns the authorization URL that should be opened in a browser.
///
/// # Arguments
/// * `redirect_uri` - The URI Google will redirect to after authorization (optional, defaults to auth.ariata.com)
///
/// # Example
/// ```
/// let auth_url = ariata::generate_google_oauth_url(None).await?;
/// println!("Visit this URL to authorize: {}", auth_url);
/// ```
pub async fn generate_google_oauth_url(redirect_uri: Option<&str>) -> Result<String> {
    let client_id = env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| Error::Other("GOOGLE_CLIENT_ID not set".to_string()))?;

    let redirect = redirect_uri.unwrap_or("https://auth.ariata.com/google/callback");

    // Google OAuth2 authorization endpoint with required parameters
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
        ?client_id={}\
        &redirect_uri={}\
        &response_type=code\
        &scope=https://www.googleapis.com/auth/calendar.readonly\
        &access_type=offline\
        &prompt=consent",
        client_id,
        urlencoding::encode(redirect)
    );

    Ok(auth_url)
}

/// Exchange a Google OAuth authorization code for tokens and create a source
///
/// Takes the authorization code from the OAuth callback and exchanges it for
/// access and refresh tokens, then stores them in the database as a new source.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `code` - Authorization code from Google OAuth callback
///
/// # Returns
/// The UUID of the newly created source
///
/// # Example
/// ```
/// let source_id = ariata::exchange_google_oauth_code(&db, "auth_code_here").await?;
/// println!("Created Google source: {}", source_id);
/// ```
pub async fn exchange_google_oauth_code(db: &PgPool, code: &str) -> Result<Uuid> {
    let client_id = env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| Error::Other("GOOGLE_CLIENT_ID not set".to_string()))?;
    let client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .map_err(|_| Error::Other("GOOGLE_CLIENT_SECRET not set".to_string()))?;

    // Exchange code for tokens via OAuth proxy
    let oauth_proxy_url = env::var("OAUTH_PROXY_URL")
        .unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/google/token", oauth_proxy_url))
        .json(&json!({
            "code": code,
            "client_id": client_id,
            "client_secret": client_secret,
            "redirect_uri": "https://auth.ariata.com/google/callback"
        }))
        .send()
        .await
        .map_err(|e| Error::Network(format!("Failed to exchange code: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::Other(format!("OAuth exchange failed: {}", error_text)));
    }

    let token_response: serde_json::Value = response.json().await
        .map_err(|e| Error::Other(format!("Failed to parse token response: {}", e)))?;

    let access_token = token_response["access_token"].as_str()
        .ok_or_else(|| Error::Other("Missing access token".to_string()))?;
    let refresh_token = token_response["refresh_token"].as_str();
    let expires_in = token_response["expires_in"].as_i64().unwrap_or(3600);

    // Create a new source in the database
    let source_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::seconds(expires_in);

    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, access_token, refresh_token, token_expires_at, is_active)
        VALUES ($1, 'google', 'Google Account', $2, $3, $4, true)
        ON CONFLICT (id) DO UPDATE SET
            access_token = $2,
            refresh_token = COALESCE($3, sources.refresh_token),
            token_expires_at = $4,
            updated_at = NOW()
        "#
    )
    .bind(source_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(expires_at)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to create source: {}", e)))?;

    Ok(source_id)
}

/// Sync Google Calendar data for a source
///
/// Performs a full or incremental sync of Google Calendar events for the given source.
/// This will fetch events from the past 30 days by default.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the Google source to sync
///
/// # Returns
/// A summary of the sync operation including number of events synced
///
/// # Example
/// ```
/// let stats = ariata::sync_google_calendar(&db, source_id).await?;
/// println!("Synced {} events", stats.events_count);
/// ```
pub async fn sync_google_calendar(db: &PgPool, source_id: Uuid) -> Result<SyncStats> {
    // Initialize the token manager
    let oauth_proxy_url = env::var("OAUTH_PROXY_URL")
        .unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    let proxy_config = OAuthProxyConfig {
        base_url: oauth_proxy_url,
    };

    let token_manager = std::sync::Arc::new(TokenManager::with_config(
        db.clone(),
        proxy_config,
    ));

    // Create the sync client
    let sync = GoogleCalendarSync::new(
        source_id,
        db.clone(),
        token_manager,
    );

    // Perform the sync (automatically syncs past 30 days of events)
    let stats = sync.sync().await?;

    Ok(SyncStats {
        events_count: stats.upserted,
        start_date: Utc::now() - Duration::days(30),
        end_date: Utc::now(),
    })
}

/// Create a Google source using a refresh token
///
/// This is a convenience function for testing or when you already have a refresh token.
/// It creates a source in the database with the provided refresh token.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `refresh_token` - Google OAuth refresh token
///
/// # Returns
/// The UUID of the newly created source
///
/// # Example
/// ```
/// let source_id = ariata::create_google_source_with_refresh_token(
///     &db,
///     "1//refresh_token_here"
/// ).await?;
/// ```
pub async fn create_google_source_with_refresh_token(
    db: &PgPool,
    refresh_token: &str
) -> Result<Uuid> {
    let source_id = Uuid::new_v4();

    // Get initial access token using the refresh token
    let oauth_proxy_url = env::var("OAUTH_PROXY_URL")
        .unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    let client_id = env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| Error::Other("GOOGLE_CLIENT_ID not set".to_string()))?;
    let client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .map_err(|_| Error::Other("GOOGLE_CLIENT_SECRET not set".to_string()))?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/google/refresh", oauth_proxy_url))
        .json(&json!({
            "refresh_token": refresh_token,
            "client_id": client_id,
            "client_secret": client_secret
        }))
        .send()
        .await
        .map_err(|e| Error::Network(format!("Failed to refresh token: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::Other(format!("Token refresh failed: {}", error_text)));
    }

    let token_response: serde_json::Value = response.json().await
        .map_err(|e| Error::Other(format!("Failed to parse token response: {}", e)))?;

    let access_token = token_response["access_token"].as_str()
        .ok_or_else(|| Error::Other("Missing access token".to_string()))?;
    let expires_in = token_response["expires_in"].as_i64().unwrap_or(3600);
    let expires_at = Utc::now() + Duration::seconds(expires_in);

    // Create the source in the database
    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, access_token, refresh_token, token_expires_at, is_active)
        VALUES ($1, 'google', 'Google Account (from refresh token)', $2, $3, $4, true)
        "#
    )
    .bind(source_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(expires_at)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to create source: {}", e)))?;

    Ok(source_id)
}

/// Summary statistics for a sync operation
#[derive(Debug)]
pub struct SyncStats {
    pub events_count: usize,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}