//! Source management API - CRUD operations for data sources

use sqlx::PgPool;
use uuid::Uuid;

use super::types::{SourceConnection, SourceConnectionStatus};
use crate::error::{Error, Result};

/// List all configured sources
///
/// Returns all sources in the database with stream counts and last sync time.
pub async fn list_sources(db: &PgPool) -> Result<Vec<SourceConnection>> {
    let sources = sqlx::query_as::<_, SourceConnection>(
        r#"
        SELECT
            s.id,
            s.source,
            s.name,
            s.auth_type,
            s.is_active,
            s.is_internal,
            s.error_message,
            s.created_at,
            s.updated_at,
            MAX(st.last_sync_at) as last_sync_at,
            COALESCE(COUNT(DISTINCT CASE WHEN st.is_enabled THEN st.stream_name END), 0) as enabled_streams_count,
            COALESCE(COUNT(DISTINCT st.stream_name), 0) as total_streams_count
        FROM source_connections s
        LEFT JOIN stream_connections st ON s.id = st.source_connection_id
        WHERE NOT (s.auth_type = 'device' AND s.pairing_status IS NULL)
        GROUP BY s.id, s.source, s.name, s.auth_type, s.is_active, s.is_internal, s.error_message, s.created_at, s.updated_at
        ORDER BY s.created_at DESC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to list sources: {e}")))?;

    Ok(sources)
}

/// Get a specific source by ID
pub async fn get_source(db: &PgPool, source_id: Uuid) -> Result<SourceConnection> {
    let source = sqlx::query_as::<_, SourceConnection>(
        r#"
        SELECT
            s.id,
            s.source,
            s.name,
            s.auth_type,
            s.is_active,
            s.is_internal,
            s.error_message,
            s.created_at,
            s.updated_at,
            MAX(st.last_sync_at) as last_sync_at,
            COALESCE(COUNT(DISTINCT CASE WHEN st.is_enabled THEN st.stream_name END), 0) as enabled_streams_count,
            COALESCE(COUNT(DISTINCT st.stream_name), 0) as total_streams_count
        FROM source_connections s
        LEFT JOIN stream_connections st ON s.id = st.source_connection_id
        WHERE s.id = $1
        GROUP BY s.id, s.source, s.name, s.auth_type, s.is_active, s.is_internal, s.error_message, s.created_at, s.updated_at
        "#,
    )
    .bind(source_id)
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to get source: {e}")))?;

    Ok(source)
}

/// Pause a source by setting is_active to false
///
/// This prevents scheduled syncs from running but keeps the source configured.
pub async fn pause_source(db: &PgPool, source_id: Uuid) -> Result<SourceConnection> {
    sqlx::query("UPDATE source_connections SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(source_id)
        .execute(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to pause source: {e}")))?;

    get_source(db, source_id).await
}

/// Resume a source by setting is_active to true
///
/// This re-enables scheduled syncs for the source.
pub async fn resume_source(db: &PgPool, source_id: Uuid) -> Result<SourceConnection> {
    sqlx::query("UPDATE source_connections SET is_active = true, updated_at = NOW() WHERE id = $1")
        .bind(source_id)
        .execute(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to resume source: {e}")))?;

    get_source(db, source_id).await
}

/// Delete a source by ID
///
/// This will cascade delete all associated data in stream tables.
pub async fn delete_source(db: &PgPool, source_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM source_connections WHERE id = $1")
        .bind(source_id)
        .execute(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to delete source: {e}")))?;

    Ok(())
}

/// Get source status with sync statistics
///
/// Returns detailed status including sync history and success rates.
pub async fn get_source_status(db: &PgPool, source_id: Uuid) -> Result<SourceConnectionStatus> {
    let status = sqlx::query_as::<_, SourceConnectionStatus>(
        r#"
        SELECT
            s.id,
            s.name,
            s.source,
            s.is_active,
            s.is_internal,
            s.last_sync_at,
            s.error_message,
            COUNT(sl.id) as total_syncs,
            COALESCE(SUM(CASE WHEN sl.status = 'success' THEN 1 ELSE 0 END), 0) as successful_syncs,
            COALESCE(SUM(CASE WHEN sl.status = 'failed' THEN 1 ELSE 0 END), 0) as failed_syncs,
            (SELECT status FROM sync_logs WHERE source_connection_id = s.id ORDER BY started_at DESC LIMIT 1) as last_sync_status,
            (SELECT duration_ms FROM sync_logs WHERE source_connection_id = s.id ORDER BY started_at DESC LIMIT 1) as last_sync_duration_ms
        FROM source_connections s
        LEFT JOIN sync_logs sl ON s.id = sl.source_connection_id
        WHERE s.id = $1
        GROUP BY s.id, s.name, s.source, s.is_active, s.is_internal, s.last_sync_at, s.error_message
        "#
    )
    .bind(source_id)
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to get source status: {e}")))?;

    Ok(status)
}
