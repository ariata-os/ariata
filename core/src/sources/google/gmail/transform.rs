//! Gmail to social_email ontology transformation
//!
//! Transforms raw Gmail messages from stream_google_gmail into the normalized
//! social_email ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Transform Gmail messages to social_email ontology
pub struct GmailEmailTransform;

#[async_trait]
impl OntologyTransform for GmailEmailTransform {
    fn source_table(&self) -> &str {
        "stream_google_gmail"
    }

    fn target_table(&self) -> &str {
        "social_email"
    }

    fn domain(&self) -> &str {
        "social"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting Gmail to social_email transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "gmail_to_social_email";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "gmail", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched Gmail batches from S3"
        );

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract required fields from JSONL record
                let Some(message_id) = record.get("message_id").and_then(|v| v.as_str()) else {
                    continue; // Skip records without message_id
                };
                let Some(thread_id) = record.get("thread_id").and_then(|v| v.as_str()) else {
                    continue; // Skip records without thread_id
                };

                let timestamp = record.get("date")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let subject = record.get("subject").and_then(|v| v.as_str()).map(String::from);
                let snippet = record.get("snippet").and_then(|v| v.as_str()).map(String::from);
                let body_plain = record.get("body_plain").and_then(|v| v.as_str()).map(String::from);
                let body_html = record.get("body_html").and_then(|v| v.as_str()).map(String::from);

                let from_email = record.get("from_email").and_then(|v| v.as_str()).map(String::from);
                let from_name = record.get("from_name").and_then(|v| v.as_str()).map(String::from);

                let to_emails: Vec<String> = record.get("to_emails")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let to_names: Vec<String> = record.get("to_names")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let cc_emails: Vec<String> = record.get("cc_emails")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let cc_names: Vec<String> = record.get("cc_names")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                let labels: Vec<String> = record.get("labels")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                let is_unread = record.get("is_unread").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_starred = record.get("is_starred").and_then(|v| v.as_bool()).unwrap_or(false);
                let has_attachments = record.get("has_attachments").and_then(|v| v.as_bool()).unwrap_or(false);
                let attachment_count = record.get("attachment_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let thread_position = record.get("thread_position").and_then(|v| v.as_i64()).map(|v| v as i32);
                let thread_message_count = record.get("thread_message_count").and_then(|v| v.as_i64()).map(|v| v as i32);
                let is_sent = record.get("is_sent").and_then(|v| v.as_bool()).unwrap_or(false);

                // Determine direction
                let direction = if is_sent { "sent" } else { "received" };

                // Insert into social_email
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.social_email (
                        message_id, thread_id, subject, snippet,
                        body_plain, body_html, timestamp,
                        from_address, from_name, to_addresses, to_names,
                        cc_addresses, cc_names,
                        direction, labels, is_read, is_starred,
                        has_attachments, attachment_count,
                        thread_position, thread_message_count,
                        source_stream_id, source_table, source_provider
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                        $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24
                    )
                    ON CONFLICT (source_table, message_id) DO NOTHING
                    "#,
                )
                .bind(message_id)
                .bind(thread_id)
                .bind(&subject)
                .bind(&snippet)
                .bind(&body_plain)
                .bind(&body_html)
                .bind(timestamp)
                .bind(&from_email)
                .bind(&from_name)
                .bind(&to_emails)
                .bind(&to_names)
                .bind(&cc_emails)
                .bind(&cc_names)
                .bind(direction)
                .bind(&labels)
                .bind(!is_unread) // is_read = !is_unread
                .bind(is_starred)
                .bind(has_attachments)
                .bind(attachment_count)
                .bind(thread_position)
                .bind(thread_message_count)
                .bind(stream_id)
                .bind("stream_google_gmail")
                .bind("google")
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
                            "Failed to transform email record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "gmail",
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
            "Gmail to social_email transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![], // Gmail transform doesn't chain to other transforms
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_metadata() {
        let transform = GmailEmailTransform;
        assert_eq!(transform.source_table(), "stream_google_gmail");
        assert_eq!(transform.target_table(), "social_email");
        assert_eq!(transform.domain(), "social");
    }
}
