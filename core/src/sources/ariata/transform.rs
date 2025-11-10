//! Chat to knowledge_ai_conversation ontology transformation
//!
//! Transforms raw chat messages from stream_ariata_ai_chat into the normalized
//! knowledge_ai_conversation ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

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

        tracing::info!(
            source_id = %source_id,
            "Starting chat to knowledge_ai_conversation transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "ariata_to_chat_conversation";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "ariata", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched Ariata AI Chat batches from S3"
        );

        for batch in batches {
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

                // Insert into knowledge_ai_conversation
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.knowledge_ai_conversation (
                        conversation_id, message_id, role, content,
                        model, provider, timestamp,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(conversation_id)
                .bind(message_id)
                .bind(role)
                .bind(content)
                .bind(&model)
                .bind(provider)
                .bind(timestamp)
                .bind(stream_id)
                .bind("stream_ariata_ai_chat")
                .bind("ariata")
                .bind(&metadata)
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            message_id = %message_id,
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform chat record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "ariata",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
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
