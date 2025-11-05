//! Chat to knowledge_ai_conversation ontology transformation
//!
//! Transforms raw chat messages from stream_ariata_ai_chat into the normalized
//! knowledge_ai_conversation ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
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

    #[tracing::instrument(skip(self, db), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting chat to knowledge_ai_conversation transformation"
        );

        // Query stream_ariata_ai_chat for records not yet transformed
        // Use left join to find records that don't exist in knowledge_ai_conversation
        let rows = sqlx::query(
            r#"
            SELECT
                c.id, c.conversation_id, c.message_id, c.role, c.content,
                c.model, c.provider, c.timestamp, c.metadata
            FROM elt.stream_ariata_ai_chat c
            LEFT JOIN elt.knowledge_ai_conversation k ON (k.source_stream_id = c.id)
            WHERE c.source_id = $1
              AND k.id IS NULL
            ORDER BY c.timestamp ASC
            LIMIT 1000
            "#,
        )
        .bind(source_id)
        .fetch_all(db.pool())
        .await?;

        tracing::debug!(
            records_to_transform = rows.len(),
            "Fetched untransformed chat records"
        );

        for row in rows {
            records_read += 1;

            // Extract fields from row
            let stream_id: Uuid = row.try_get("id")?;
            let conversation_id: String = row.try_get("conversation_id")?;
            let message_id: String = row.try_get("message_id")?;
            let role: String = row.try_get("role")?;
            let content: String = row.try_get("content")?;
            let model: Option<String> = row.try_get("model")?;
            let provider: String = row.try_get("provider")?;
            let timestamp: DateTime<Utc> = row.try_get("timestamp")?;
            let metadata: serde_json::Value = row.try_get("metadata")?;

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
            .bind(&conversation_id)
            .bind(&message_id)
            .bind(&role)
            .bind(&content)
            .bind(&model)
            .bind(&provider)
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
