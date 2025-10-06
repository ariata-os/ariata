//! Sync logging for observability and audit trail
//!
//! The SyncLogger writes sync operation results to the database for:
//! - Observability: Monitor sync health, throughput, error rates
//! - Debugging: Trace sync failures weeks/months later
//! - Compliance: Audit trail of all data movement
//! - Analytics: Trend analysis on sync performance

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::Result;
use super::sync_mode::{SyncMode, SyncResult};

/// Helper for persisting sync results to the database
pub struct SyncLogger {
    db: PgPool,
}

impl SyncLogger {
    /// Create a new sync logger
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log a successful sync operation
    #[tracing::instrument(skip(self, result), fields(source_id = %source_id, mode = ?mode))]
    pub async fn log_success(
        &self,
        source_id: Uuid,
        mode: &SyncMode,
        result: &SyncResult,
    ) -> Result<Uuid> {
        let sync_mode_str = match mode {
            SyncMode::FullRefresh => "full_refresh",
            SyncMode::Incremental { .. } => "incremental",
        };

        let cursor_before = match mode {
            SyncMode::Incremental { cursor } => cursor.as_deref(),
            SyncMode::FullRefresh => None,
        };

        let log_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO sync_logs (
                source_id,
                sync_mode,
                started_at,
                completed_at,
                duration_ms,
                status,
                records_fetched,
                records_written,
                records_failed,
                sync_cursor_before,
                sync_cursor_after
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id
            "#
        )
        .bind(source_id)
        .bind(sync_mode_str)
        .bind(result.started_at)
        .bind(result.completed_at)
        .bind(result.duration_ms() as i32)
        .bind("success")
        .bind(result.records_fetched as i32)
        .bind(result.records_written as i32)
        .bind(result.records_failed as i32)
        .bind(cursor_before)
        .bind(result.next_cursor.as_deref())
        .fetch_one(&self.db)
        .await?;

        tracing::info!(
            log_id = %log_id,
            records_fetched = result.records_fetched,
            records_written = result.records_written,
            duration_ms = result.duration_ms(),
            "Sync completed successfully"
        );

        Ok(log_id)
    }

    /// Log a failed sync operation
    #[tracing::instrument(skip(self, error), fields(source_id = %source_id, mode = ?mode))]
    pub async fn log_failure(
        &self,
        source_id: Uuid,
        mode: &SyncMode,
        started_at: DateTime<Utc>,
        error: &crate::error::Error,
    ) -> Result<Uuid> {
        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds();

        let sync_mode_str = match mode {
            SyncMode::FullRefresh => "full_refresh",
            SyncMode::Incremental { .. } => "incremental",
        };

        let cursor_before = match mode {
            SyncMode::Incremental { cursor } => cursor.as_deref(),
            SyncMode::FullRefresh => None,
        };

        // Classify the error for monitoring
        let error_class = classify_error(error);
        let error_message = error.to_string();

        let log_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO sync_logs (
                source_id,
                sync_mode,
                started_at,
                completed_at,
                duration_ms,
                status,
                records_fetched,
                records_written,
                records_failed,
                error_message,
                error_class,
                sync_cursor_before
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id
            "#
        )
        .bind(source_id)
        .bind(sync_mode_str)
        .bind(started_at)
        .bind(completed_at)
        .bind(duration_ms as i32)
        .bind("failed")
        .bind(0_i32)
        .bind(0_i32)
        .bind(0_i32)
        .bind(&error_message)
        .bind(error_class)
        .bind(cursor_before)
        .fetch_one(&self.db)
        .await?;

        tracing::error!(
            log_id = %log_id,
            error_class = error_class,
            duration_ms = duration_ms,
            error = %error,
            "Sync failed"
        );

        Ok(log_id)
    }

    /// Log a partial sync (some records succeeded, some failed)
    #[tracing::instrument(skip(self, result), fields(source_id = %source_id, mode = ?mode))]
    pub async fn log_partial(
        &self,
        source_id: Uuid,
        mode: &SyncMode,
        result: &SyncResult,
        error_message: Option<&str>,
    ) -> Result<Uuid> {
        let sync_mode_str = match mode {
            SyncMode::FullRefresh => "full_refresh",
            SyncMode::Incremental { .. } => "incremental",
        };

        let cursor_before = match mode {
            SyncMode::Incremental { cursor } => cursor.as_deref(),
            SyncMode::FullRefresh => None,
        };

        let log_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO sync_logs (
                source_id,
                sync_mode,
                started_at,
                completed_at,
                duration_ms,
                status,
                records_fetched,
                records_written,
                records_failed,
                error_message,
                sync_cursor_before,
                sync_cursor_after
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id
            "#
        )
        .bind(source_id)
        .bind(sync_mode_str)
        .bind(result.started_at)
        .bind(result.completed_at)
        .bind(result.duration_ms() as i32)
        .bind("partial")
        .bind(result.records_fetched as i32)
        .bind(result.records_written as i32)
        .bind(result.records_failed as i32)
        .bind(error_message)
        .bind(cursor_before)
        .bind(result.next_cursor.as_deref())
        .fetch_one(&self.db)
        .await?;

        tracing::warn!(
            log_id = %log_id,
            records_written = result.records_written,
            records_failed = result.records_failed,
            success_rate = result.success_rate(),
            "Partial sync completed"
        );

        Ok(log_id)
    }

    /// Get recent sync logs for a source
    pub async fn get_recent_logs(
        &self,
        source_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SyncLog>> {
        let logs = sqlx::query_as::<_, SyncLog>(
            r#"
            SELECT
                id, source_id, sync_mode, started_at, completed_at, duration_ms,
                status, records_fetched, records_written, records_failed,
                error_message, error_class, sync_cursor_before, sync_cursor_after,
                created_at
            FROM sync_logs
            WHERE source_id = $1
            ORDER BY started_at DESC
            LIMIT $2
            "#
        )
        .bind(source_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }
}

/// Sync log record from the database
#[derive(Debug, sqlx::FromRow)]
pub struct SyncLog {
    pub id: Uuid,
    pub source_id: Uuid,
    pub sync_mode: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub status: String,
    pub records_fetched: i32,
    pub records_written: i32,
    pub records_failed: i32,
    pub error_message: Option<String>,
    pub error_class: Option<String>,
    pub sync_cursor_before: Option<String>,
    pub sync_cursor_after: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Classify errors for monitoring and alerting
fn classify_error(error: &crate::error::Error) -> &'static str {
    use crate::error::Error;

    match error {
        Error::Http(msg) => {
            // Try to classify based on error message patterns
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
        Error::Source(_) => "sync_token_error",  // Source-specific errors are usually sync token issues
        Error::Database(_) => "database_error",
        Error::Storage(_) => "storage_error",
        Error::Authentication(_) | Error::Unauthorized(_) => "auth_error",
        Error::Serialization(_) => "serialization_error",
        Error::Configuration(_) => "config_error",
        _ => "unknown_error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        use crate::error::Error;

        // Auth errors
        assert_eq!(classify_error(&Error::Http("401 Unauthorized".to_string())), "auth_error");
        assert_eq!(classify_error(&Error::Authentication("Token expired".to_string())), "auth_error");

        // Rate limit
        assert_eq!(classify_error(&Error::Http("429 Too Many Requests".to_string())), "rate_limit");
        assert_eq!(classify_error(&Error::Http("Rate limit exceeded".to_string())), "rate_limit");

        // Sync token errors
        assert_eq!(classify_error(&Error::Source("Sync token invalid".to_string())), "sync_token_error");
        assert_eq!(classify_error(&Error::Http("410 Sync token is no longer valid".to_string())), "sync_token_error");

        // Server errors
        assert_eq!(classify_error(&Error::Http("500 Internal Server Error".to_string())), "server_error");
        assert_eq!(classify_error(&Error::Http("503 Service Unavailable".to_string())), "server_error");

        // Client errors
        assert_eq!(classify_error(&Error::Http("400 Bad Request".to_string())), "client_error");
        assert_eq!(classify_error(&Error::Http("404 Not Found".to_string())), "client_error");
    }
}
