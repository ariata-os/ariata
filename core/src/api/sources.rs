//! Source management API - CRUD operations for data sources

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};
use super::types::{Source, SourceStatus};

/// List all configured sources
///
/// Returns all sources in the database, regardless of type (OAuth, device, etc.)
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

