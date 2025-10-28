//! Sync execution and history API

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::sources::{base::SyncLogger, StreamFactory};
use chrono::Utc;

use super::types::SyncLog;

/// Trigger a sync for a specific stream
pub async fn sync_stream(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    sync_mode: Option<crate::sources::base::SyncMode>,
) -> Result<SyncLog> {
    let started_at = Utc::now();
    let logger = SyncLogger::new(db.clone());

    // Create factory
    let factory = StreamFactory::new(db.clone());

    // Create stream instance
    let mut stream = factory.create_stream(source_id, stream_name).await?;

    // Load configuration
    stream.load_config(db, source_id).await?;

    // Determine sync mode (default to incremental if not specified)
    let mode = sync_mode.unwrap_or_else(|| crate::sources::base::SyncMode::incremental(None));

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
