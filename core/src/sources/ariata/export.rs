//! App Chat Export Stream
//!
//! Exports chat messages from app.chat_sessions to object storage (MinIO/S3)
//! using cursor-based incremental sync (updated_at timestamp).
//!
//! ## Naming Architecture
//!
//! This export follows the three-tier naming convention:
//! - **Stream Name**: `app_export` (registered in elt.streams)
//! - **Stream Table**: `stream_ariata_ai_chat` (object storage destination)
//! - **Ontology Table**: `knowledge_ai_conversation` (final transformed data)
//!
//! ## Data Flow
//!
//! 1. Source: `app.chat_sessions` (operational JSONB storage)
//! 2. Export: Write to `stream_ariata_ai_chat` in S3 as JSONL
//! 3. Transform: `ChatConversationTransform` reads S3, writes to `knowledge_ai_conversation`
//!
//! The stream name `app_export` is normalized to `stream_ariata_ai_chat` by the
//! transform registry for routing transform jobs.

use crate::error::Result;
use crate::sources::base::{SyncMode, SyncResult};
use crate::sources::stream::Stream;
use crate::storage::stream_writer::StreamWriter;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Value as JsonValue};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// Epoch timestamp used as default cursor for initial sync
const EPOCH: i64 = 0; // Unix epoch (1970-01-01 00:00:00 UTC)

pub struct AppChatExportStream {
    db: PgPool,
    source_id: Uuid,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl AppChatExportStream {
    pub fn new(db: PgPool, source_id: Uuid, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self {
            db,
            source_id,
            stream_writer,
        }
    }
}

#[async_trait]
impl Stream for AppChatExportStream {
    /// Export chat messages from app.chat_sessions to object storage (MinIO/S3)
    ///
    /// Uses cursor-based incremental sync:
    /// - Cursor is the last updated_at timestamp from app.chat_sessions
    /// - Queries sessions modified since cursor
    /// - Extracts messages from JSONB array
    /// - Writes to object storage as encrypted JSONL via StreamWriter
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
        let mut collected_records = Vec::new(); // For direct transform

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
                let message_id = format!(
                    "{}_{}_{}",
                    session_id,
                    msg_timestamp.timestamp_millis(),
                    msg_idx
                );

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

                // Build record for object storage
                let record = json!({
                    "source_id": self.source_id,
                    "conversation_id": session_id,
                    "message_id": message_id,
                    "role": role,
                    "content": content,
                    "model": model,
                    "provider": provider,
                    "timestamp": msg_timestamp.to_rfc3339(),
                    "metadata": {}
                });

                // Write to StreamWriter buffer (buffered, no S3 flush)
                // For direct transform: keep copy in memory
                collected_records.push(record.clone());

                {
                    let mut writer = self.stream_writer.lock().await;
                    writer.write_record(
                        self.source_id,
                        "app_export",
                        record,
                        Some(msg_timestamp),
                    )?;
                }

                records_written += 1;
                latest_timestamp = latest_timestamp.max(msg_timestamp);
            }

            // Track latest session timestamp
            latest_timestamp = latest_timestamp.max(session_updated_at);
        }

        // NOTE: We do NOT flush to S3 here anymore!
        // Records remain in StreamWriter buffer for archival job to handle.
        // The direct transform path uses collected_records immediately.

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
            "App chat export completed (records ready for direct transform)"
        );

        Ok(SyncResult {
            records_fetched: records_written,
            records_written,
            records_failed,
            next_cursor,
            started_at,
            completed_at,
            // NEW: Include records for direct transform (hot path)
            records: if collected_records.is_empty() {
                None
            } else {
                Some(collected_records)
            },
            // archive_job_id will be set by sync_job after creating archive job
            archive_job_id: None,
        })
    }

    fn table_name(&self) -> &str {
        "stream_ariata_ai_chat"
    }

    fn stream_name(&self) -> &str {
        "app_export"
    }

    fn source_name(&self) -> &str {
        "ariata"
    }

    async fn load_config(&mut self, _db: &PgPool, _source_id: Uuid) -> Result<()> {
        // No external config needed for internal export
        Ok(())
    }
}
