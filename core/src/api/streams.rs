//! Stream management and configuration API

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::{Error, Result};
use super::sources::get_source;

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
    pub cron_schedule: Option<String>,
}

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

    // Trigger initial sync in the background
    // This provides immediate feedback and validates the connection works
    let db_clone = db.clone();
    let stream_name_clone = stream_name.to_string();
    tokio::spawn(async move {
        match crate::api::jobs::trigger_stream_sync(&db_clone, source_id, &stream_name_clone, None).await {
            Ok(response) => {
                tracing::info!(
                    "Initial sync job created for {}: job_id={}, status={}",
                    stream_name_clone,
                    response.job_id,
                    response.status
                );
            }
            Err(e) => {
                tracing::error!("Failed to create initial sync job for {}: {}", stream_name_clone, e);
            }
        }
    });

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

/// Enable default streams for a newly created source (internal helper)
pub async fn enable_default_streams(db: &PgPool, source_id: Uuid, source_type: &str) -> Result<()> {
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
