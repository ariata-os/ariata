//! Chat to knowledge_ai_conversation ontology transformation
//!
//! Transforms raw chat messages from stream_ariata_ai_chat into the normalized
//! knowledge_ai_conversation ontology table.
//!
//! ## Data Flow
//!
//! - **Source**: `stream_ariata_ai_chat` (JSONL in S3, written by AppChatExportStream)
//! - **Target**: `elt.knowledge_ai_conversation` (normalized ontology table)
//! - **Checkpoint**: `ariata_to_chat_conversation` (tracks S3 read progress)
//!
//! ## Naming Context
//!
//! The transform registry maps:
//! - Stream name `app_export` → Stream table `stream_ariata_ai_chat` (via normalize_stream_name)
//! - Stream table `stream_ariata_ai_chat` → Ontology `knowledge_ai_conversation` (this transform)
//!
//! ## Transform Logic
//!
//! 1. Read batches from S3 using checkpoint tracking
//! 2. Parse JSONL records (conversation_id, message_id, role, content, model, provider, timestamp)
//! 3. Insert into knowledge_ai_conversation with source tracking
//! 4. Use ON CONFLICT to prevent duplicates (by source_stream_id)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Batch size for database inserts
const BATCH_SIZE: usize = 500;

/// Transform AI chat messages to knowledge_ai_conversation ontology
pub struct ChatConversationTransform;

#[async_trait]
impl OntologyTransform for ChatConversationTransform {
    fn source_table(&self) -> &str {
        "stream_ariata_ai_chat"
    }

    fn target_table(&self) -> &str {
        "knowledge_ai_conversation"
    }

    fn domain(&self) -> &str {
        "knowledge"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting chat to knowledge_ai_conversation transformation"
        );

        // Read stream data using data source (memory for hot path)
        // Note: Must use the same stream name that the export writes with ("app_export")
        let checkpoint_key = "ariata_to_chat_conversation";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| {
            crate::Error::Other("No data source available for transform".to_string())
        })?;
        let batches = data_source
            .read_with_checkpoint(source_id, "app_export", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::debug!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched Ariata AI Chat batches (hot path)"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(String, String, String, String, Option<String>, String, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            tracing::debug!(
                batch_record_count = batch.records.len(),
                "Processing batch"
            );

            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(message_id) = record.get("message_id").and_then(|v| v.as_str()) else {
                    continue; // Skip records without message_id
                };

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let conversation_id = record.get("conversation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let role = record.get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");

                let content = record.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let model = record.get("model")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let provider = record.get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let metadata = record.get("metadata").cloned()
                    .unwrap_or(serde_json::Value::Null);

                // Add to pending batch
                pending_records.push((
                    conversation_id.to_string(),
                    message_id.to_string(),
                    role.to_string(),
                    content.to_string(),
                    model,
                    provider.to_string(),
                    timestamp,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_chat_conversation_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "app_export",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_chat_conversation_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "Chat to knowledge_ai_conversation transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![], // Chat transform doesn't chain to other transforms
        })
    }
}

/// Execute batch insert for chat conversation records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_chat_conversation_batch_insert(
    db: &Database,
    records: &[(String, String, String, String, Option<String>, String, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    // Build batch insert query using helper
    let columns = vec![
        "conversation_id",
        "message_id",
        "role",
        "content",
        "model",
        "provider",
        "timestamp",
        "source_stream_id",
        "source_table",
        "source_provider",
        "metadata",
    ];

    let query_str = Database::build_batch_insert_query(
        "elt.knowledge_ai_conversation",
        &columns,
        "source_stream_id",
        records.len(),
    );

    // Build query with proper parameter binding
    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (conversation_id, message_id, role, content, model, provider, timestamp, stream_id, metadata) in records {
        query = query
            .bind(conversation_id)
            .bind(message_id)
            .bind(role)
            .bind(content)
            .bind(model)
            .bind(provider)
            .bind(timestamp)
            .bind(stream_id)
            .bind("stream_ariata_ai_chat")
            .bind("ariata")
            .bind(metadata);
    }

    // Execute batch insert
    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_metadata() {
        let transform = ChatConversationTransform;
        assert_eq!(transform.source_table(), "stream_ariata_ai_chat");
        assert_eq!(transform.target_table(), "knowledge_ai_conversation");
        assert_eq!(transform.domain(), "knowledge");
    }
}
