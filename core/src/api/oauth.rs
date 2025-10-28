//! OAuth flow and source registration API

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::sources::get_source;
use super::types::Source;
use crate::error::{Error, Result};
use crate::oauth::TokenManager;

/// Request parameters for initiating OAuth authorization
#[derive(Debug, serde::Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
}

/// Response containing OAuth authorization URL
#[derive(Debug, serde::Serialize)]
pub struct OAuthAuthorizeResponse {
    pub authorization_url: String,
    pub state: String,
}

/// OAuth callback query parameters
#[derive(Debug, serde::Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub provider: String,
    pub state: Option<String>,
}

/// Request for creating a source manually
#[derive(Debug, serde::Deserialize)]
pub struct CreateSourceRequest {
    #[serde(rename = "type")]
    pub source_type: String,
    pub name: String,
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub device_id: Option<String>,
}

/// Request for registering a device as a source
#[derive(Debug, serde::Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_type: String,
    pub device_id: String,
    pub name: String,
}

/// Initiate OAuth authorization flow
pub async fn initiate_oauth_flow(
    provider: &str,
    redirect_uri: Option<String>,
    state: Option<String>,
) -> Result<OAuthAuthorizeResponse> {
    let descriptor = crate::registry::get_source(provider)
        .ok_or_else(|| Error::Other(format!("Unknown provider: {provider}")))?;

    if descriptor.auth_type != crate::registry::AuthType::OAuth2 {
        return Err(Error::Other(format!(
            "Provider {provider} does not use OAuth2 authentication"
        )));
    }

    let oauth_config = descriptor
        .oauth_config
        .as_ref()
        .ok_or_else(|| Error::Other(format!("No OAuth config for provider: {provider}")))?;

    let state = state.unwrap_or_else(|| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:x}", rng.gen::<u128>())
    });

    let proxy_url =
        std::env::var("OAUTH_PROXY_URL").unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    let redirect_uri = redirect_uri.unwrap_or_else(|| format!("{proxy_url}/callback"));
    let scopes = oauth_config.scopes.join(" ");

    let authorization_url = format!(
        "{proxy_url}/{provider}/auth?return_url={}&state={}&scope={}",
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state),
        urlencoding::encode(&scopes)
    );

    Ok(OAuthAuthorizeResponse {
        authorization_url,
        state,
    })
}

/// Handle OAuth callback and create source
pub async fn handle_oauth_callback(
    db: &PgPool,
    code: &str,
    provider: &str,
    _state: Option<String>,
) -> Result<Source> {
    let descriptor = crate::registry::get_source(provider)
        .ok_or_else(|| Error::Other(format!("Unknown provider: {provider}")))?;

    // Exchange code for tokens via OAuth proxy
    let proxy_url =
        std::env::var("OAUTH_PROXY_URL").unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    let client = reqwest::Client::new();

    let response = client
        .post(&format!("{}/{}/token", proxy_url, provider))
        .json(&serde_json::json!({
            "code": code,
        }))
        .send()
        .await
        .map_err(|e| Error::Http(format!("Failed to exchange code for tokens: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::Http(format!(
            "Token exchange failed: {} {}",
            status, error_text
        )));
    }

    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_in: Option<i64>,
    }

    let token_data: TokenResponse = response
        .json()
        .await
        .map_err(|e| Error::Http(format!("Failed to parse token response: {e}")))?;

    let source_name = format!("{} Account", descriptor.display_name);
    let token_manager = std::sync::Arc::new(TokenManager::new(db.clone()));

    let source_id = token_manager
        .store_initial_tokens(
            provider,
            &source_name,
            token_data.access_token,
            token_data.refresh_token,
            token_data.expires_in,
        )
        .await?;

    super::streams::enable_default_streams(db, source_id, provider).await?;

    get_source(db, source_id).await
}

/// Create a source manually (for testing or direct token input)
pub async fn create_source(db: &PgPool, request: CreateSourceRequest) -> Result<Source> {
    let descriptor = crate::registry::get_source(&request.source_type)
        .ok_or_else(|| Error::Other(format!("Unknown source type: {}", request.source_type)))?;

    let source_id = Uuid::new_v4();

    match descriptor.auth_type {
        crate::registry::AuthType::OAuth2 => {
            let refresh_token = request.refresh_token.clone().ok_or_else(|| {
                Error::Other("refresh_token required for OAuth2 sources".to_string())
            })?;

            let access_token = request.access_token.clone().ok_or_else(|| {
                Error::Other("access_token required for OAuth2 sources".to_string())
            })?;

            let expires_in = request
                .token_expires_at
                .map(|expires_at| (expires_at - Utc::now()).num_seconds());

            let token_manager = std::sync::Arc::new(TokenManager::new(db.clone()));
            token_manager
                .store_initial_tokens(
                    &request.source_type,
                    &request.name,
                    access_token,
                    Some(refresh_token),
                    expires_in,
                )
                .await?;
        }
        crate::registry::AuthType::Device => {}
        _ => {
            return Err(Error::Other(format!(
                "Source type {} not yet supported for manual creation",
                request.source_type
            )));
        }
    }

    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, true, NOW(), NOW())
        "#,
    )
    .bind(source_id)
    .bind(&request.source_type)
    .bind(&request.name)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to create source: {e}")))?;

    super::streams::enable_default_streams(db, source_id, &request.source_type).await?;

    get_source(db, source_id).await
}

/// Register a device as a source
pub async fn register_device(db: &PgPool, request: RegisterDeviceRequest) -> Result<Source> {
    let descriptor = crate::registry::get_source(&request.device_type)
        .ok_or_else(|| Error::Other(format!("Unknown device type: {}", request.device_type)))?;

    if descriptor.auth_type != crate::registry::AuthType::Device {
        return Err(Error::Other(format!(
            "Source type {} is not a device source",
            request.device_type
        )));
    }

    let source_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, true, NOW(), NOW())
        "#,
    )
    .bind(source_id)
    .bind(&request.device_type)
    .bind(&request.name)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to register device: {e}")))?;

    super::streams::enable_default_streams(db, source_id, &request.device_type).await?;

    get_source(db, source_id).await
}
