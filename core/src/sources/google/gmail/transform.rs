//! Gmail to social_email ontology transformation
//!
//! Transforms raw Gmail messages from stream_google_gmail into the normalized
//! social_email ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
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

    #[tracing::instrument(skip(self, db), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting Gmail to social_email transformation"
        );

        // Query stream_google_gmail for records not yet transformed
        // Use left join to find records that don't exist in social_email
        let rows = sqlx::query(
            r#"
            SELECT
                g.id, g.message_id, g.thread_id, g.subject, g.snippet,
                g.body_plain, g.body_html, g.date,
                g.from_email, g.from_name, g.to_emails, g.to_names,
                g.cc_emails, g.cc_names,
                g.labels, g.is_unread, g.is_starred,
                g.has_attachments, g.attachment_count,
                g.thread_position, g.thread_message_count,
                g.is_sent
            FROM elt.stream_google_gmail g
            LEFT JOIN elt.social_email e ON (e.source_stream_id = g.id)
            WHERE g.source_id = $1
              AND e.id IS NULL
            ORDER BY g.date ASC
            LIMIT 1000
            "#,
        )
        .bind(source_id)
        .fetch_all(db.pool())
        .await?;

        tracing::debug!(
            records_to_transform = rows.len(),
            "Fetched untransformed Gmail records"
        );

        for row in rows {
            records_read += 1;

            // Extract fields from row
            let stream_id: Uuid = row.try_get("id")?;
            let message_id: String = row.try_get("message_id")?;
            let thread_id: String = row.try_get("thread_id")?;
            let subject: Option<String> = row.try_get("subject")?;
            let snippet: Option<String> = row.try_get("snippet")?;
            let body_plain: Option<String> = row.try_get("body_plain")?;
            let body_html: Option<String> = row.try_get("body_html")?;
            let timestamp: DateTime<Utc> = row.try_get("date")?;

            let from_email: Option<String> = row.try_get("from_email")?;
            let from_name: Option<String> = row.try_get("from_name")?;
            let to_emails: Vec<String> = row.try_get("to_emails")?;
            let to_names: Vec<String> = row.try_get("to_names")?;
            let cc_emails: Vec<String> = row.try_get("cc_emails")?;
            let cc_names: Vec<String> = row.try_get("cc_names")?;

            let labels: Vec<String> = row.try_get("labels")?;
            let is_unread: bool = row.try_get("is_unread")?;
            let is_starred: bool = row.try_get("is_starred")?;
            let has_attachments: bool = row.try_get("has_attachments")?;
            let attachment_count: i32 = row.try_get("attachment_count")?;
            let thread_position: Option<i32> = row.try_get("thread_position")?;
            let thread_message_count: Option<i32> = row.try_get("thread_message_count")?;
            let is_sent: bool = row.try_get("is_sent")?;

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
            .bind(&message_id)
            .bind(&thread_id)
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
