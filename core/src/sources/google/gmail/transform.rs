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

/// Batch size for bulk inserts
const BATCH_SIZE: usize = 500;

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

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting Gmail to social_email transformation"
        );

        // Read stream data using data source (memory for hot path)
        let checkpoint_key = "gmail_to_social_email";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "gmail", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched Gmail batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, DateTime<Utc>, Option<String>, Option<String>, Vec<String>, Vec<String>, Vec<String>, Vec<String>, &'static str, Vec<String>, bool, bool, bool, i32, Option<i32>, Option<i32>, Uuid)> = Vec::new();
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

                // Add to pending batch
                pending_records.push((
                    message_id.to_string(),
                    thread_id.to_string(),
                    subject,
                    snippet,
                    body_plain,
                    body_html,
                    timestamp,
                    from_email,
                    from_name,
                    to_emails,
                    to_names,
                    cc_emails,
                    cc_names,
                    direction,
                    labels,
                    !is_unread, // is_read = !is_unread
                    is_starred,
                    has_attachments,
                    attachment_count,
                    thread_position,
                    thread_message_count,
                    stream_id,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_email_batch_insert(db, &pending_records).await;
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
                    "gmail",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_email_batch_insert(db, &pending_records).await;
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

/// Execute batch insert for email records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_email_batch_insert(
    db: &Database,
    records: &[(String, String, Option<String>, Option<String>, Option<String>, Option<String>, DateTime<Utc>, Option<String>, Option<String>, Vec<String>, Vec<String>, Vec<String>, Vec<String>, &str, Vec<String>, bool, bool, bool, i32, Option<i32>, Option<i32>, Uuid)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.social_email",
        &["message_id", "thread_id", "subject", "snippet", "body_plain", "body_html", "timestamp", "from_address", "from_name", "to_addresses", "to_names", "cc_addresses", "cc_names", "direction", "labels", "is_read", "is_starred", "has_attachments", "attachment_count", "thread_position", "thread_message_count", "source_stream_id", "source_table", "source_provider"],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (message_id, thread_id, subject, snippet, body_plain, body_html, timestamp, from_address, from_name, to_addresses, to_names, cc_addresses, cc_names, direction, labels, is_read, is_starred, has_attachments, attachment_count, thread_position, thread_message_count, stream_id) in records {
        query = query
            .bind(message_id)
            .bind(thread_id)
            .bind(subject)
            .bind(snippet)
            .bind(body_plain)
            .bind(body_html)
            .bind(timestamp)
            .bind(from_address)
            .bind(from_name)
            .bind(to_addresses)
            .bind(to_names)
            .bind(cc_addresses)
            .bind(cc_names)
            .bind(direction)
            .bind(labels)
            .bind(is_read)
            .bind(is_starred)
            .bind(has_attachments)
            .bind(attachment_count)
            .bind(thread_position)
            .bind(thread_message_count)
            .bind(stream_id)
            .bind("stream_google_gmail")
            .bind("google");
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
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
