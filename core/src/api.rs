//! Library-level API functions for programmatic access
//!
//! These functions provide a simple, library-first interface for OAuth flows
//! and data synchronization, suitable for use from Python wrappers or other bindings.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

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
    pub last_sync_at: Option<DateTime<Utc>>,
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
        SELECT id, type as source_type, name, is_active, last_sync_at,
               error_message, created_at, updated_at
        FROM sources
        ORDER BY created_at DESC
        "#
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
        SELECT id, type as source_type, name, is_active, last_sync_at,
               error_message, created_at, updated_at
        FROM sources
        WHERE id = $1
        "#
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

/// Trigger a sync for any source by ID
///
/// Note: Direct sync triggering is not implemented at the API level.
/// Use the scheduler for automatic periodic syncs, or call source-specific
/// sync implementations directly from their modules (e.g., GoogleCalendarSync::new().sync()).
///
/// # Arguments
/// * `_db` - Database connection pool
/// * `source_id` - UUID of the source to sync
///
/// # Returns
/// Currently returns an error - sync should be triggered via scheduler
pub async fn sync_source(_db: &PgPool, source_id: Uuid) -> Result<()> {
    Err(Error::Other(format!(
        "Direct sync not implemented. Use scheduler for automatic syncs. Source ID: {source_id}"
    )))
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
        "#
    )
    .bind(source_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to get sync history: {e}")))?;

    Ok(logs)
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

/// Get information about a specific stream
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
/// let calendar = ariata::get_stream_info("google", "calendar").unwrap();
/// println!("Table: {}", calendar.table_name);
/// println!("Config schema: {}", calendar.config_schema);
/// ```
pub fn get_stream_info(source_name: &str, stream_name: &str) -> Option<&'static crate::registry::StreamDescriptor> {
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