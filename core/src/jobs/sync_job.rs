//! Sync job execution logic

use crate::error::Result;
use crate::jobs::models::Job;
use crate::sources::{base::SyncMode, StreamFactory};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;

/// Execute a sync job
///
/// This function is called by the job executor to perform the actual sync work.
/// It updates the job status in the database as it progresses.
pub async fn execute_sync_job(db: &PgPool, job: &Job) -> Result<()> {
    let source_id = job
        .source_id
        .ok_or_else(|| crate::Error::InvalidInput("Sync job missing source_id".to_string()))?;

    let stream_name = job.stream_name.as_ref().ok_or_else(|| {
        crate::Error::InvalidInput("Sync job missing stream_name".to_string())
    })?;

    let sync_mode_str = job.sync_mode.as_ref().ok_or_else(|| {
        crate::Error::InvalidInput("Sync job missing sync_mode".to_string())
    })?;

    // Convert sync mode string to SyncMode enum
    let sync_mode = match sync_mode_str.as_str() {
        "full_refresh" => SyncMode::FullRefresh,
        "incremental" => SyncMode::incremental(None),
        _ => {
            return Err(crate::Error::InvalidInput(format!(
                "Invalid sync mode: {}",
                sync_mode_str
            )))
        }
    };

    // Extract cursor from metadata if present
    let cursor_before = job.metadata
        .get("cursor_before")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Create factory and stream instance
    let factory = StreamFactory::new(db.clone());
    let mut stream = factory.create_stream(source_id, stream_name).await?;

    // Load configuration
    stream.load_config(db, source_id).await?;

    // Execute sync
    let result = stream.sync(sync_mode.clone()).await;

    match result {
        Ok(sync_result) => {
            // Build metadata with detailed sync info
            let metadata = json!({
                "cursor_before": cursor_before,
                "cursor_after": sync_result.next_cursor,
                "records_fetched": sync_result.records_fetched,
                "records_written": sync_result.records_written,
                "records_failed": sync_result.records_failed,
                "duration_ms": sync_result.duration_ms()
            });

            // Update the streams table with last sync timestamp
            sqlx::query(
                r#"
                UPDATE elt.streams
                SET last_sync_at = $1, updated_at = NOW()
                WHERE source_id = $2 AND stream_name = $3
                "#,
            )
            .bind(sync_result.completed_at)
            .bind(source_id)
            .bind(stream_name)
            .execute(db)
            .await?;

            // Update job with final stats and metadata
            sqlx::query(
                r#"
                UPDATE elt.jobs
                SET status = 'succeeded',
                    completed_at = NOW(),
                    records_processed = $1,
                    metadata = $2
                WHERE id = $3
                "#,
            )
            .bind(sync_result.records_written as i64)
            .bind(metadata)
            .bind(job.id)
            .execute(db)
            .await?;

            tracing::info!(
                job_id = %job.id,
                stream_name = %stream_name,
                records_fetched = sync_result.records_fetched,
                records_written = sync_result.records_written,
                duration_ms = sync_result.duration_ms(),
                "Sync job completed successfully"
            );

            Ok(())
        }
        Err(e) => {
            // Classify error for monitoring
            let error_class = classify_sync_error(&e);

            // Build metadata with error details
            let metadata = json!({
                "cursor_before": cursor_before,
                "error_class": error_class
            });

            // Update job with error
            sqlx::query(
                r#"
                UPDATE elt.jobs
                SET status = 'failed',
                    completed_at = NOW(),
                    error_message = $1,
                    error_class = $2,
                    metadata = $3
                WHERE id = $4
                "#,
            )
            .bind(e.to_string())
            .bind(error_class)
            .bind(metadata)
            .bind(job.id)
            .execute(db)
            .await?;

            tracing::error!(
                job_id = %job.id,
                stream_name = %stream_name,
                error_class = error_class,
                error = %e,
                "Sync job failed"
            );

            Err(e)
        }
    }
}

/// Classify errors for monitoring and alerting
fn classify_sync_error(error: &crate::error::Error) -> &'static str {
    use crate::error::Error;

    match error {
        Error::Http(msg) => {
            let msg_lower = msg.to_lowercase();
            if msg_lower.contains("401") || msg_lower.contains("unauthorized") {
                "auth_error"
            } else if msg_lower.contains("429") || msg_lower.contains("rate limit") {
                "rate_limit"
            } else if msg_lower.contains("sync token") {
                "sync_token_error"
            } else if msg_lower.contains("5") && (msg_lower.contains("500") || msg_lower.contains("503")) {
                "server_error"
            } else if msg_lower.contains("4") && (msg_lower.contains("400") || msg_lower.contains("404")) {
                "client_error"
            } else {
                "network_error"
            }
        }
        Error::Source(_) => "sync_token_error",
        Error::Database(_) => "database_error",
        Error::Storage(_) => "storage_error",
        Error::Authentication(_) | Error::Unauthorized(_) => "auth_error",
        Error::Serialization(_) => "serialization_error",
        Error::Configuration(_) => "config_error",
        _ => "unknown_error",
    }
}
