//! App Chat Export Stream
//!
//! Exports chat messages from app.chat_sessions to elt.stream_ariata_ai_chat
//! using cursor-based incremental sync (updated_at timestamp).

use async_trait::async_trait;
use crate::error::Result;
use crate::sources::base::{SyncMode, SyncResult};
use crate::sources::stream::Stream;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use uuid::Uuid;

// Epoch timestamp used as default cursor for initial sync
const EPOCH: i64 = 0; // Unix epoch (1970-01-01 00:00:00 UTC)

pub struct AppChatExportStream {
    db: PgPool,
    source_id: Uuid,
}

impl AppChatExportStream {
    pub fn new(db: PgPool, source_id: Uuid) -> Self {
        Self { db, source_id }
    }
}

#[async_trait]
impl Stream for AppChatExportStream {
    /// Export chat messages from app.chat_sessions to elt.stream_ariata_ai_chat
    ///
    /// Uses cursor-based incremental sync:
    /// - Cursor is the last updated_at timestamp from app.chat_sessions
    /// - Queries sessions modified since cursor
    /// - Extracts messages from JSONB array
    /// - Inserts into stream table with UPSERT
    async fn sync(&self, sync_mode: SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();

        // Get cursor (last exported timestamp)
        let cursor = match sync_mode {
            SyncMode::Incremental { cursor } => cursor,
            SyncMode::FullRefresh => None,
        };

        let last_exported: DateTime<Utc> = cursor
            .as_ref()
            .and_then(|c| DateTime::parse_from_rfc3339(c).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc.timestamp_opt(EPOCH, 0).unwrap());

        tracing::info!(
            source_id = %self.source_id,
            last_exported = %last_exported,
            "Starting app chat export"
        );

        // Query sessions with new/updated messages since cursor
        // Using regular query instead of query! macro to avoid compile-time schema check
        let sessions = sqlx::query(
            r#"
            SELECT id, messages, updated_at
            FROM app.chat_sessions
            WHERE updated_at > $1
            ORDER BY updated_at ASC
            "#,
        )
        .bind(last_exported)
        .fetch_all(&self.db)
        .await?;

        let mut records_written = 0;
        let mut records_failed = 0;
        let mut latest_timestamp = last_exported;

        // Start transaction to ensure atomic writes
        let mut tx = self.db.begin().await?;

        for session_row in sessions {
            let session_id: Uuid = session_row.get("id");
            let messages_json: JsonValue = session_row.get("messages");
            let session_updated_at: DateTime<Utc> = session_row.get("updated_at");

            let messages: Vec<JsonValue> = match serde_json::from_value(messages_json) {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::error!(
                        session_id = %session_id,
                        error = %e,
                        "Failed to parse messages JSONB, skipping session"
                    );
                    records_failed += 1;
                    continue; // Skip this session entirely
                }
            };

            for (msg_idx, message) in messages.iter().enumerate() {
                // Parse message timestamp
                let msg_timestamp: DateTime<Utc> = message
                    .get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                // Only export messages newer than cursor
                if msg_timestamp <= last_exported {
                    continue;
                }

                // Generate deterministic message_id with index to prevent collisions
                // Format: {session_id}_{timestamp_ms}_{index}
                let message_id = format!("{}_{}_{}",session_id, msg_timestamp.timestamp_millis(), msg_idx);

                // Extract message fields
                let role = message
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or("user");
                let content = message
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let model = message.get("model").and_then(|m| m.as_str());
                let provider = message
                    .get("provider")
                    .and_then(|p| p.as_str())
                    .unwrap_or("anthropic");

                // Insert into stream table (UPSERT) within transaction
                sqlx::query(
                    r#"
                    INSERT INTO elt.stream_ariata_ai_chat (
                        source_id, conversation_id, message_id, role, content,
                        model, provider, timestamp, metadata
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (source_id, message_id) DO UPDATE
                    SET content = EXCLUDED.content,
                        model = EXCLUDED.model,
                        provider = EXCLUDED.provider,
                        updated_at = NOW()
                    "#,
                )
                .bind(self.source_id)
                .bind(session_id)
                .bind(&message_id)
                .bind(role)
                .bind(content)
                .bind(model)
                .bind(provider)
                .bind(msg_timestamp)
                .bind(serde_json::json!({}))
                .execute(&mut *tx)
                .await?;

                records_written += 1;
                latest_timestamp = latest_timestamp.max(msg_timestamp);
            }

            // Track latest session timestamp
            latest_timestamp = latest_timestamp.max(session_updated_at);
        }

        // Commit transaction - all writes are atomic
        tx.commit().await?;

        let completed_at = Utc::now();
        let next_cursor = if records_written > 0 {
            Some(latest_timestamp.to_rfc3339())
        } else {
            cursor
        };

        tracing::info!(
            source_id = %self.source_id,
            records_written,
            records_failed,
            next_cursor = ?next_cursor,
            duration_ms = (completed_at - started_at).num_milliseconds(),
            "App chat export completed"
        );

        Ok(SyncResult {
            records_fetched: records_written,
            records_written,
            records_failed,
            next_cursor,
            started_at,
            completed_at,
        })
    }

    fn table_name(&self) -> &str {
        "stream_ariata_ai_chat"
    }

    fn stream_name(&self) -> &str {
        "app_export"
    }

    fn source_name(&self) -> &str {
        "ariata_app"
    }

    async fn load_config(&mut self, _db: &PgPool, _source_id: Uuid) -> Result<()> {
        // No external config needed for internal export
        Ok(())
    }
}
