//! Library-level API functions for programmatic access
//!
//! These functions provide a simple, library-first interface for OAuth flows
//! and data synchronization, suitable for use from Python wrappers or other bindings.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

// ============================================================================
// Source Management API (Generic - works with any source)
// ============================================================================

/// Represents a configured data source
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Source {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub source_type: String,
    pub name: String,
    pub is_active: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Source status with sync statistics
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SourceStatus {
    pub id: Uuid,
    pub name: String,
    pub source_type: String,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub total_syncs: i64,
    pub successful_syncs: i64,
    pub failed_syncs: i64,
    pub last_sync_status: Option<String>,
    pub last_sync_duration_ms: Option<i32>,
}

/// Sync log entry
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SyncLog {
    pub id: Uuid,
    pub source_id: Uuid,
    pub sync_mode: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub status: String,
    pub records_fetched: Option<i32>,
    pub records_written: Option<i32>,
    pub records_failed: Option<i32>,
    pub error_message: Option<String>,
}

/// List all configured sources
///
/// Returns all sources in the database, regardless of type (OAuth, device, etc.)
///
/// # Example
/// ```
/// let sources = ariata::list_sources(&db).await?;
/// for source in sources {
///     println!("{} - {} ({})", source.id, source.name, source.source_type);
/// }
/// ```
pub async fn list_sources(db: &PgPool) -> Result<Vec<Source>> {
    let sources = sqlx::query_as::<_, Source>(
        r#"
        SELECT id, type as source_type, name, is_active,
               error_message, created_at, updated_at
        FROM sources
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to list sources: {e}")))?;

    Ok(sources)
}

/// Get a specific source by ID
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
///
/// # Example
/// ```
/// let source = ariata::get_source(&db, source_id).await?;
/// println!("Source: {} ({})", source.name, source.source_type);
/// ```
pub async fn get_source(db: &PgPool, source_id: Uuid) -> Result<Source> {
    let source = sqlx::query_as::<_, Source>(
        r#"
        SELECT id, type as source_type, name, is_active,
               error_message, created_at, updated_at
        FROM sources
        WHERE id = $1
        "#,
    )
    .bind(source_id)
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to get source: {e}")))?;

    Ok(source)
}

/// Delete a source by ID
///
/// This will cascade delete all associated data in stream tables.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source to delete
///
/// # Example
/// ```
/// ariata::delete_source(&db, source_id).await?;
/// println!("Source deleted");
/// ```
pub async fn delete_source(db: &PgPool, source_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM sources WHERE id = $1")
        .bind(source_id)
        .execute(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to delete source: {e}")))?;

    Ok(())
}

/// Get source status with sync statistics
///
/// Returns detailed status including sync history and success rates.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
///
/// # Example
/// ```
/// let status = ariata::get_source_status(&db, source_id).await?;
/// println!("Total syncs: {}, Success rate: {:.1}%",
///     status.total_syncs,
///     (status.successful_syncs as f64 / status.total_syncs as f64) * 100.0
/// );
/// ```
pub async fn get_source_status(db: &PgPool, source_id: Uuid) -> Result<SourceStatus> {
    let status = sqlx::query_as::<_, SourceStatus>(
        r#"
        SELECT
            s.id,
            s.name,
            s.type as source_type,
            s.is_active,
            s.last_sync_at,
            s.error_message,
            COUNT(sl.id) as total_syncs,
            COALESCE(SUM(CASE WHEN sl.status = 'success' THEN 1 ELSE 0 END), 0) as successful_syncs,
            COALESCE(SUM(CASE WHEN sl.status = 'failed' THEN 1 ELSE 0 END), 0) as failed_syncs,
            (SELECT status FROM sync_logs WHERE source_id = s.id ORDER BY started_at DESC LIMIT 1) as last_sync_status,
            (SELECT duration_ms FROM sync_logs WHERE source_id = s.id ORDER BY started_at DESC LIMIT 1) as last_sync_duration_ms
        FROM sources s
        LEFT JOIN sync_logs sl ON s.id = sl.source_id
        WHERE s.id = $1
        GROUP BY s.id, s.name, s.type, s.is_active, s.last_sync_at, s.error_message
        "#
    )
    .bind(source_id)
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to get source status: {e}")))?;

    Ok(status)
}

/// Trigger a sync for a specific stream
///
/// This synchronizes a single stream from a source using the StreamFactory.
/// The sync mode can be specified, or will auto-detect based on available sync tokens.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream to sync (e.g., "calendar", "gmail")
/// * `sync_mode` - Optional sync mode (defaults to auto-detect)
///
/// # Returns
/// A SyncLog entry with the results of the sync operation
///
/// # Example
/// ```rust
/// // Sync Google Calendar stream in incremental mode
/// let log = ariata::sync_stream(
///     &db,
///     source_id,
///     "calendar",
///     Some(SyncMode::incremental(None))
/// ).await?;
/// ```
pub async fn sync_stream(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    sync_mode: Option<crate::sources::base::SyncMode>,
) -> Result<SyncLog> {
    use crate::sources::{base::SyncLogger, StreamFactory};
    use chrono::Utc;

    let started_at = Utc::now();
    let logger = SyncLogger::new(db.clone());

    // Create factory
    let factory = StreamFactory::new(db.clone());

    // Create stream instance
    let mut stream = factory.create_stream(source_id, stream_name).await?;

    // Load configuration
    stream.load_config(db, source_id).await?;

    // Determine sync mode
    let mode = sync_mode.unwrap_or_else(|| {
        // Default: try incremental, fall back to full refresh if no token
        crate::sources::base::SyncMode::incremental(None)
    });

    // Execute sync and handle errors
    let result = match stream.sync(mode.clone()).await {
        Ok(result) => {
            // Log success to database
            let log_id = logger
                .log_success(source_id, stream_name, &mode, &result)
                .await?;

            // Convert SyncResult to SyncLog for return
            SyncLog {
                id: log_id,
                source_id,
                sync_mode: match mode {
                    crate::sources::base::SyncMode::FullRefresh => "full_refresh".to_string(),
                    crate::sources::base::SyncMode::Incremental { .. } => "incremental".to_string(),
                },
                started_at: result.started_at,
                completed_at: Some(result.completed_at),
                duration_ms: Some(
                    (result.completed_at - result.started_at).num_milliseconds() as i32
                ),
                status: if result.records_failed == 0 {
                    "success"
                } else {
                    "partial"
                }
                .to_string(),
                records_fetched: Some(result.records_fetched as i32),
                records_written: Some(result.records_written as i32),
                records_failed: Some(result.records_failed as i32),
                error_message: None,
            }
        }
        Err(e) => {
            // Log failure to database
            let log_id = logger
                .log_failure(source_id, stream_name, &mode, started_at, &e)
                .await
                .unwrap_or_else(|log_err| {
                    tracing::error!(error = %log_err, "Failed to log sync failure");
                    Uuid::new_v4()
                });

            // Convert error to SyncLog for return
            SyncLog {
                id: log_id,
                source_id,
                sync_mode: match mode {
                    crate::sources::base::SyncMode::FullRefresh => "full_refresh".to_string(),
                    crate::sources::base::SyncMode::Incremental { .. } => "incremental".to_string(),
                },
                started_at,
                completed_at: Some(Utc::now()),
                duration_ms: Some((Utc::now() - started_at).num_milliseconds() as i32),
                status: "failed".to_string(),
                records_fetched: None,
                records_written: None,
                records_failed: None,
                error_message: Some(e.to_string()),
            }
        }
    };

    Ok(result)
}

/// Get sync history for a source
///
/// Returns recent sync operations with results and timing information.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `limit` - Maximum number of logs to return
///
/// # Example
/// ```
/// let logs = ariata::get_sync_history(&db, source_id, 10).await?;
/// for log in logs {
///     println!("{}: {} - {} records in {}ms",
///         log.started_at, log.status, log.records_written.unwrap_or(0), log.duration_ms.unwrap_or(0)
///     );
/// }
/// ```
pub async fn get_sync_history(db: &PgPool, source_id: Uuid, limit: i64) -> Result<Vec<SyncLog>> {
    let logs = sqlx::query_as::<_, SyncLog>(
        r#"
        SELECT id, source_id, sync_mode, started_at, completed_at, duration_ms,
               status, records_fetched, records_written, records_failed, error_message
        FROM sync_logs
        WHERE source_id = $1
        ORDER BY started_at DESC
        LIMIT $2
        "#,
    )
    .bind(source_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to get sync history: {e}")))?;

    Ok(logs)
}

/// Get sync history for a specific stream
///
/// Returns recent sync operations for a specific stream with results and timing information.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream
/// * `limit` - Maximum number of logs to return
///
/// # Example
/// ```
/// let logs = ariata::get_stream_sync_history(&db, source_id, "calendar", 10).await?;
/// for log in logs {
///     println!("{}: {} - {} records in {}ms",
///         log.started_at, log.status, log.records_written.unwrap_or(0), log.duration_ms.unwrap_or(0)
///     );
/// }
/// ```
pub async fn get_stream_sync_history(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    limit: i64,
) -> Result<Vec<SyncLog>> {
    let logs = sqlx::query_as::<_, SyncLog>(
        r#"
        SELECT id, source_id, sync_mode, started_at, completed_at, duration_ms,
               status, records_fetched, records_written, records_failed, error_message
        FROM sync_logs
        WHERE source_id = $1 AND stream_name = $2
        ORDER BY started_at DESC
        LIMIT $3
        "#,
    )
    .bind(source_id)
    .bind(stream_name)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|e| {
        Error::Database(format!(
            "Failed to get sync history for stream {stream_name}: {e}"
        ))
    })?;

    Ok(logs)
}

// ============================================================================
// OAuth Flow & Source Registration API
// ============================================================================

/// Request parameters for initiating OAuth authorization
#[derive(Debug, serde::Deserialize)]
pub struct OAuthAuthorizeRequest {
    /// Optional callback URL (defaults to configured OAuth proxy callback)
    pub redirect_uri: Option<String>,
    /// Optional state parameter for CSRF protection
    pub state: Option<String>,
}

/// Response containing OAuth authorization URL
#[derive(Debug, serde::Serialize)]
pub struct OAuthAuthorizeResponse {
    /// URL to redirect user to for OAuth authorization
    pub authorization_url: String,
    /// State parameter (for CSRF validation)
    pub state: String,
}

/// OAuth callback query parameters
#[derive(Debug, serde::Deserialize)]
pub struct OAuthCallbackParams {
    /// Authorization code from provider
    pub code: String,
    /// Provider name (google, notion, etc.)
    pub provider: String,
    /// Optional state for CSRF validation
    pub state: Option<String>,
}

/// Request for creating a source manually
#[derive(Debug, serde::Deserialize)]
pub struct CreateSourceRequest {
    /// Source type (google, notion, ios, mac)
    #[serde(rename = "type")]
    pub source_type: String,
    /// Display name for the source
    pub name: String,
    /// OAuth refresh token (for OAuth sources)
    pub refresh_token: Option<String>,
    /// Access token (optional, for OAuth sources)
    pub access_token: Option<String>,
    /// Token expiration (optional, for OAuth sources)
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Device ID (for device sources)
    pub device_id: Option<String>,
}

/// Request for registering a device as a source
#[derive(Debug, serde::Deserialize)]
pub struct RegisterDeviceRequest {
    /// Device type (ios, mac)
    pub device_type: String,
    /// Device identifier (unique per device)
    pub device_id: String,
    /// Device name/description
    pub name: String,
}

/// Initiate OAuth authorization flow
///
/// Returns an authorization URL to redirect the user to for OAuth consent.
/// The user will be redirected back to the callback URL with an authorization code.
///
/// # Arguments
/// * `provider` - Provider name (google, notion, etc.)
/// * `redirect_uri` - Optional callback URL (defaults to OAuth proxy callback)
/// * `state` - Optional state for CSRF protection (generated if not provided)
///
/// # Returns
/// Authorization URL and state parameter
///
/// # Example
/// ```rust
/// let response = ariata::initiate_oauth_flow("google", None, None).await?;
/// println!("Redirect user to: {}", response.authorization_url);
/// ```
pub async fn initiate_oauth_flow(
    provider: &str,
    redirect_uri: Option<String>,
    state: Option<String>,
) -> Result<OAuthAuthorizeResponse> {
    // Get source descriptor to validate provider
    let descriptor = crate::registry::get_source(provider)
        .ok_or_else(|| Error::Other(format!("Unknown provider: {provider}")))?;

    // Verify it's an OAuth provider
    if descriptor.auth_type != crate::registry::AuthType::OAuth2 {
        return Err(Error::Other(format!(
            "Provider {provider} does not use OAuth2 authentication"
        )));
    }

    // Get OAuth configuration
    let oauth_config = descriptor
        .oauth_config
        .as_ref()
        .ok_or_else(|| Error::Other(format!("No OAuth config for provider: {provider}")))?;

    // Generate state if not provided (for CSRF protection)
    let state = state.unwrap_or_else(|| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:x}", rng.gen::<u128>())
    });

    // Get OAuth proxy URL from environment
    let proxy_url =
        std::env::var("OAUTH_PROXY_URL").unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    // Build authorization URL via OAuth proxy
    let redirect_uri = redirect_uri.unwrap_or_else(|| format!("{proxy_url}/callback"));

    let scopes = oauth_config.scopes.join(" ");

    // Build authorization URL
    let authorization_url = if redirect_uri.starts_with("http://localhost:") {
        // CLI mode: build direct Google OAuth URL
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| Error::Other("GOOGLE_CLIENT_ID not set in environment".to_string()))?;

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
            oauth_config.auth_url,
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&state)
        )
    } else {
        // Production: via OAuth proxy (correct route is /auth not /authorize)
        format!(
            "{proxy_url}/{provider}/auth?return_url={}&state={}&scope={}",
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&state),
            urlencoding::encode(&scopes)
        )
    };

    Ok(OAuthAuthorizeResponse {
        authorization_url,
        state,
    })
}

/// Handle OAuth callback and create source
///
/// Exchanges authorization code for tokens via OAuth proxy and creates a new source.
/// Automatically enables default streams for the source.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `code` - Authorization code from OAuth provider
/// * `provider` - Provider name (google, notion, etc.)
/// * `state` - Optional state for CSRF validation
///
/// # Returns
/// Newly created Source
///
/// # Example
/// ```rust
/// let source = ariata::handle_oauth_callback(&db, "auth_code_123", "google", None).await?;
/// println!("Created source: {} ({})", source.name, source.id);
/// ```
pub async fn handle_oauth_callback(
    db: &PgPool,
    code: &str,
    provider: &str,
    _state: Option<String>,
) -> Result<Source> {
    use crate::oauth::TokenManager;

    // Validate provider
    let descriptor = crate::registry::get_source(provider)
        .ok_or_else(|| Error::Other(format!("Unknown provider: {provider}")))?;

    // Exchange code for tokens
    let proxy_url =
        std::env::var("OAUTH_PROXY_URL").unwrap_or_else(|_| "https://auth.ariata.com".to_string());

    let client = reqwest::Client::new();

    // Use direct OAuth if:
    // 1. Explicitly set via env var, OR
    // 2. Redirect URI is localhost (CLI mode), OR
    // 3. Not using default proxy URL
    let use_direct_oauth = std::env::var("USE_DIRECT_OAUTH").is_ok()
        || std::env::var("GOOGLE_CLIENT_SECRET").is_ok()  // CLI has secret set
        || proxy_url != "https://auth.ariata.com";

    let response = if use_direct_oauth {
        // Direct OAuth exchange (CLI mode)
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| Error::Other("GOOGLE_CLIENT_ID not set".to_string()))?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| Error::Other("GOOGLE_CLIENT_SECRET not set".to_string()))?;

        client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("code", code),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
                ("redirect_uri", "http://localhost:8080"),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|e| Error::Http(format!("Failed to exchange code for tokens: {e}")))?
    } else {
        // Via OAuth proxy (production)
        client
            .post(&format!("{proxy_url}/{provider}/token"))
            .json(&serde_json::json!({
                "code": code,
            }))
            .send()
            .await
            .map_err(|e| Error::Http(format!("Failed to exchange code for tokens: {e}")))?
    };

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::Http(format!("Token exchange failed: {error_text}")));
    }

    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }

    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| Error::Http(format!("Failed to parse token response: {e}")))?;

    // Create source in database with tokens
    let source_id = Uuid::new_v4();
    let source_name = format!("{} Account", descriptor.display_name);
    let token_manager = std::sync::Arc::new(TokenManager::new(db.clone()));

    // Encrypt and store tokens
    token_manager
        .store_initial_tokens(
            provider,
            &source_name,
            token_response.access_token.clone(),
            token_response.refresh_token.clone(),
            token_response.expires_in,
        )
        .await?;

    // Insert source record
    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, true, NOW(), NOW())
        "#,
    )
    .bind(source_id)
    .bind(provider)
    .bind(&source_name)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to create source: {e}")))?;

    // Enable default streams
    enable_default_streams(db, source_id, provider).await?;

    // Return created source
    get_source(db, source_id).await
}

/// Create a source manually (for testing or direct token input)
///
/// # Arguments
/// * `db` - Database connection pool
/// * `request` - Source creation request with type, name, and credentials
///
/// # Returns
/// Newly created Source
pub async fn create_source(db: &PgPool, request: CreateSourceRequest) -> Result<Source> {
    use crate::oauth::TokenManager;

    // Validate source type exists
    let descriptor = crate::registry::get_source(&request.source_type)
        .ok_or_else(|| Error::Other(format!("Unknown source type: {}", request.source_type)))?;

    let source_id = Uuid::new_v4();

    match descriptor.auth_type {
        crate::registry::AuthType::OAuth2 => {
            // Require refresh token for OAuth sources
            let refresh_token = request.refresh_token.clone().ok_or_else(|| {
                Error::Other("refresh_token required for OAuth2 sources".to_string())
            })?;

            let access_token = request.access_token.clone().ok_or_else(|| {
                Error::Other("access_token required for OAuth2 sources".to_string())
            })?;

            // Convert token_expires_at to expires_in (seconds from now)
            let expires_in = request
                .token_expires_at
                .map(|expires_at| (expires_at - Utc::now()).num_seconds());

            // Store tokens
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
        crate::registry::AuthType::Device => {
            // Device sources don't need OAuth tokens
            // device_id will be validated when device pushes data
        }
        _ => {
            return Err(Error::Other(format!(
                "Source type {} not yet supported for manual creation",
                request.source_type
            )));
        }
    }

    // Insert source record
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

    // Enable default streams
    enable_default_streams(db, source_id, &request.source_type).await?;

    // Return created source
    get_source(db, source_id).await
}

/// Register a device as a source
///
/// # Arguments
/// * `db` - Database connection pool
/// * `request` - Device registration request
///
/// # Returns
/// Newly created Source representing the device
pub async fn register_device(db: &PgPool, request: RegisterDeviceRequest) -> Result<Source> {
    // Validate device type
    let descriptor = crate::registry::get_source(&request.device_type)
        .ok_or_else(|| Error::Other(format!("Unknown device type: {}", request.device_type)))?;

    if descriptor.auth_type != crate::registry::AuthType::Device {
        return Err(Error::Other(format!(
            "Source type {} is not a device source",
            request.device_type
        )));
    }

    let source_id = Uuid::new_v4();

    // Insert source record (device_id stored in name for now)
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

    // Enable default streams
    enable_default_streams(db, source_id, &request.device_type).await?;

    // Return created source
    get_source(db, source_id).await
}

/// Enable default streams for a newly created source
async fn enable_default_streams(db: &PgPool, source_id: Uuid, source_type: &str) -> Result<()> {
    let descriptor = crate::registry::get_source(source_type)
        .ok_or_else(|| Error::Other(format!("Unknown source type: {source_type}")))?;

    // Insert streams table entries for all available streams (disabled by default)
    for stream in &descriptor.streams {
        sqlx::query(
            r#"
            INSERT INTO streams (id, source_id, stream_name, table_name, is_enabled, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, false, '{}', NOW(), NOW())
            ON CONFLICT (source_id, stream_name) DO NOTHING
            "#
        )
        .bind(Uuid::new_v4())
        .bind(source_id)
        .bind(stream.name)
        .bind(stream.table_name)
        .execute(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to enable stream {}: {e}", stream.name)))?;
    }

    Ok(())
}

// ============================================================================
// Stream Discovery & Configuration API
// ============================================================================

/// Information about a stream, including enablement status and configuration
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct StreamInfo {
    pub stream_name: String,
    pub display_name: String,
    pub description: String,
    pub table_name: String,
    pub is_enabled: bool,
    pub cron_schedule: Option<String>,
    pub config: serde_json::Value,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub supports_incremental: bool,
    pub supports_full_refresh: bool,
    pub config_schema: serde_json::Value,
    pub config_example: serde_json::Value,
    pub default_cron_schedule: Option<String>,
}

/// Request for enabling a stream
#[derive(Debug, serde::Deserialize)]
pub struct EnableStreamRequest {
    /// Optional initial configuration (uses defaults if not provided)
    pub config: Option<serde_json::Value>,
}

/// Request for updating stream configuration
#[derive(Debug, serde::Deserialize)]
pub struct UpdateStreamConfigRequest {
    pub config: serde_json::Value,
}

/// Request for updating stream schedule
#[derive(Debug, serde::Deserialize)]
pub struct UpdateStreamScheduleRequest {
    /// Cron expression (e.g., "0 */6 * * *")
    pub cron_schedule: Option<String>,
}

/// List all streams for a source
///
/// Returns information about all available streams for the source type,
/// including which ones are enabled and their current configuration.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
///
/// # Returns
/// List of StreamInfo with enablement status and configuration
///
/// # Example
/// ```rust
/// let streams = ariata::list_source_streams(&db, source_id).await?;
/// for stream in streams {
///     println!("{}: {} (enabled: {})",
///         stream.stream_name,
///         stream.display_name,
///         stream.is_enabled
///     );
/// }
/// ```
pub async fn list_source_streams(db: &PgPool, source_id: Uuid) -> Result<Vec<StreamInfo>> {
    // Get source to determine type
    let source = get_source(db, source_id).await?;
    let source_type = source.source_type;

    // Get source descriptor from registry
    let descriptor = crate::registry::get_source(&source_type)
        .ok_or_else(|| Error::Other(format!("Unknown source type: {source_type}")))?;

    // Get enabled streams from database
    let enabled_streams: Vec<(
        String,
        bool,
        Option<String>,
        serde_json::Value,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        r#"
            SELECT stream_name, is_enabled, cron_schedule, config, last_sync_at
            FROM streams
            WHERE source_id = $1
            "#,
    )
    .bind(source_id)
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to query streams: {e}")))?;

    // Build response by combining registry metadata with database state
    let mut result = Vec::new();
    for stream_desc in &descriptor.streams {
        // Find matching database record
        let db_record = enabled_streams
            .iter()
            .find(|(name, _, _, _, _)| name == stream_desc.name);

        let (is_enabled, cron_schedule, config, last_sync_at) = if let Some(record) = db_record {
            (record.1, record.2.clone(), record.3.clone(), record.4)
        } else {
            (false, None, serde_json::json!({}), None)
        };

        result.push(StreamInfo {
            stream_name: stream_desc.name.to_string(),
            display_name: stream_desc.display_name.to_string(),
            description: stream_desc.description.to_string(),
            table_name: stream_desc.table_name.to_string(),
            is_enabled,
            cron_schedule,
            config,
            last_sync_at,
            supports_incremental: stream_desc.supports_incremental,
            supports_full_refresh: stream_desc.supports_full_refresh,
            config_schema: stream_desc.config_schema.clone(),
            config_example: stream_desc.config_example.clone(),
            default_cron_schedule: stream_desc.default_cron_schedule.map(|s| s.to_string()),
        });
    }

    Ok(result)
}

/// Get details for a specific stream
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream
///
/// # Returns
/// StreamInfo with current configuration
pub async fn get_stream_info(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
) -> Result<StreamInfo> {
    let streams = list_source_streams(db, source_id).await?;
    streams
        .into_iter()
        .find(|s| s.stream_name == stream_name)
        .ok_or_else(|| Error::Other(format!("Stream not found: {stream_name}")))
}

/// Enable a stream for a source
///
/// Creates an entry in the streams table with the provided or default configuration.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream to enable
/// * `config` - Optional configuration (uses defaults if not provided)
///
/// # Returns
/// Updated StreamInfo
pub async fn enable_stream(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    config: Option<serde_json::Value>,
) -> Result<StreamInfo> {
    // Get source to determine type
    let source = get_source(db, source_id).await?;

    // Validate stream exists in registry
    let stream_desc = crate::registry::get_stream(&source.source_type, stream_name)
        .ok_or_else(|| Error::Other(format!("Stream not found: {stream_name}")))?;

    // Use provided config or empty object (stream will load defaults)
    let config = config.unwrap_or_else(|| serde_json::json!({}));

    // Get default cron schedule from registry
    let default_schedule = stream_desc.default_cron_schedule;

    // Insert or update streams table
    sqlx::query(
        r#"
        INSERT INTO streams (id, source_id, stream_name, table_name, is_enabled, config, cron_schedule, created_at, updated_at)
        VALUES ($1, $2, $3, $4, true, $5, $6, NOW(), NOW())
        ON CONFLICT (source_id, stream_name)
        DO UPDATE SET
            is_enabled = true,
            config = EXCLUDED.config,
            cron_schedule = COALESCE(streams.cron_schedule, EXCLUDED.cron_schedule),
            updated_at = NOW()
        "#
    )
    .bind(Uuid::new_v4())
    .bind(source_id)
    .bind(stream_name)
    .bind(stream_desc.table_name)
    .bind(&config)
    .bind(default_schedule)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to enable stream: {e}")))?;

    // Return updated stream info
    get_stream_info(db, source_id, stream_name).await
}

/// Disable a stream for a source
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream to disable
pub async fn disable_stream(db: &PgPool, source_id: Uuid, stream_name: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE streams
        SET is_enabled = false, updated_at = NOW()
        WHERE source_id = $1 AND stream_name = $2
        "#,
    )
    .bind(source_id)
    .bind(stream_name)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to disable stream: {e}")))?;

    Ok(())
}

/// Update stream configuration
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream
/// * `config` - New configuration (JSONB)
///
/// # Returns
/// Updated StreamInfo
pub async fn update_stream_config(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    config: serde_json::Value,
) -> Result<StreamInfo> {
    // Validate stream exists
    get_stream_info(db, source_id, stream_name).await?;

    // Update config
    sqlx::query(
        r#"
        UPDATE streams
        SET config = $1, updated_at = NOW()
        WHERE source_id = $2 AND stream_name = $3
        "#,
    )
    .bind(&config)
    .bind(source_id)
    .bind(stream_name)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to update stream config: {e}")))?;

    // Return updated stream info
    get_stream_info(db, source_id, stream_name).await
}

/// Update stream cron schedule
///
/// # Arguments
/// * `db` - Database connection pool
/// * `source_id` - UUID of the source
/// * `stream_name` - Name of the stream
/// * `cron_schedule` - Cron expression (e.g., "0 */6 * * *") or None to disable scheduling
///
/// # Returns
/// Updated StreamInfo
pub async fn update_stream_schedule(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    cron_schedule: Option<String>,
) -> Result<StreamInfo> {
    // Validate stream exists
    get_stream_info(db, source_id, stream_name).await?;

    // Update schedule
    sqlx::query(
        r#"
        UPDATE streams
        SET cron_schedule = $1, updated_at = NOW()
        WHERE source_id = $2 AND stream_name = $3
        "#,
    )
    .bind(&cron_schedule)
    .bind(source_id)
    .bind(stream_name)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to update stream schedule: {e}")))?;

    // Return updated stream info
    get_stream_info(db, source_id, stream_name).await
}

// ============================================================================
// Catalog / Registry API
// ============================================================================

/// List all available sources in the catalog
///
/// Returns metadata about all sources that can be configured, including
/// their authentication requirements, available streams, and configuration options.
///
/// # Example
/// ```
/// let sources = ariata::list_available_sources();
/// for source in sources {
///     println!("Source: {} ({})", source.display_name, source.name);
///     println!("  Auth: {:?}", source.auth_type);
///     println!("  Streams: {}", source.streams.len());
/// }
/// ```
pub fn list_available_sources() -> Vec<&'static crate::registry::SourceDescriptor> {
    crate::registry::list_sources()
}

/// Get information about a specific source
///
/// # Arguments
/// * `name` - The source identifier (e.g., "google", "strava", "notion")
///
/// # Returns
/// Source metadata including available streams and configuration schemas, or None if not found
///
/// # Example
/// ```
/// let google = ariata::get_source_info("google").unwrap();
/// println!("Google has {} streams available", google.streams.len());
/// ```
pub fn get_source_info(name: &str) -> Option<&'static crate::registry::SourceDescriptor> {
    crate::registry::get_source(name)
}

/// Get descriptor for a specific stream from the registry
///
/// # Arguments
/// * `source_name` - The source identifier (e.g., "google")
/// * `stream_name` - The stream identifier (e.g., "calendar")
///
/// # Returns
/// Stream metadata including configuration schema and database table name, or None if not found
///
/// # Example
/// ```
/// let calendar = ariata::get_stream_descriptor("google", "calendar").unwrap();
/// println!("Table: {}", calendar.table_name);
/// println!("Config schema: {}", calendar.config_schema);
/// ```
pub fn get_stream_descriptor(
    source_name: &str,
    stream_name: &str,
) -> Option<&'static crate::registry::StreamDescriptor> {
    crate::registry::get_stream(source_name, stream_name)
}

/// List all streams across all sources
///
/// Returns a list of (source_name, stream_descriptor) tuples for all registered streams.
///
/// # Example
/// ```
/// let all_streams = ariata::list_all_streams();
/// for (source, stream) in all_streams {
///     println!("{}.{} -> {}", source, stream.name, stream.table_name);
/// }
/// ```
pub fn list_all_streams() -> Vec<(&'static str, &'static crate::registry::StreamDescriptor)> {
    crate::registry::list_all_streams()
}
