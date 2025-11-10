//! Google Calendar to activity_calendar_entry ontology transformation
//!
//! Transforms raw calendar events from stream_google_calendar into the normalized
//! activity_calendar_entry ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Transform Google Calendar events to activity_calendar_entry ontology
pub struct GoogleCalendarTransform;

#[async_trait]
impl OntologyTransform for GoogleCalendarTransform {
    fn source_table(&self) -> &str {
        "stream_google_calendar"
    }

    fn target_table(&self) -> &str {
        "activity_calendar_entry"
    }

    fn domain(&self) -> &str {
        "activity"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting Google Calendar to activity_calendar_entry transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "calendar_to_activity_calendar_entry";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "calendar", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched Google Calendar batches from S3"
        );

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(event_id) = record.get("event_id").and_then(|v| v.as_str()) else {
                    continue; // Skip records without event_id
                };

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let calendar_id = record.get("calendar_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let summary = record.get("summary")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let description = record.get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let location = record.get("location")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let status = record.get("status")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let start_time = record.get("start_time")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let end_time = record.get("end_time")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let all_day = record.get("all_day")
                    .and_then(|v| v.as_bool());

                let organizer_email = record.get("organizer_email")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let attendee_count = record.get("attendee_count")
                    .and_then(|v| v.as_i64())
                    .map(|c| c as i32);

                let has_conferencing = record.get("has_conferencing")
                    .and_then(|v| v.as_bool());

                let conference_type = record.get("conference_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let conference_link = record.get("conference_link")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let raw_json = record.get("raw_json").cloned()
                    .unwrap_or(serde_json::Value::Null);

                // Extract attendee emails from raw_json if available
                let attendee_identifiers: Vec<String> = raw_json
                    .get("attendees")
                    .and_then(|a| a.as_array())
                    .map(|attendees| {
                        attendees
                            .iter()
                            .filter_map(|att| att.get("email").and_then(|e| e.as_str()))
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                // Determine event type based on data
                let event_type = if has_conferencing.unwrap_or(false) {
                    Some("meeting")
                } else if attendee_count.unwrap_or(0) > 1 {
                    Some("meeting")
                } else if all_day.unwrap_or(false) {
                    Some("reminder")
                } else {
                    Some("appointment")
                };

                // Build metadata with Google-specific fields
                let metadata = serde_json::json!({
                    "google_event_id": event_id,
                    "google_calendar_id": calendar_id,
                    "is_recurring": raw_json.get("recurringEventId").is_some(),
                    "google_raw": raw_json,
                });

                // Insert into activity_calendar_entry
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.activity_calendar_entry (
                        title, description, calendar_name, event_type,
                        organizer_identifier, attendee_identifiers,
                        location_name, conference_url, conference_platform,
                        start_time, end_time, is_all_day,
                        status, response_status,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(&summary) // title
                .bind(&description) // description
                .bind(calendar_id) // calendar_name
                .bind(event_type) // event_type
                .bind(&organizer_email) // organizer_identifier
                .bind(&attendee_identifiers) // attendee_identifiers
                .bind(&location) // location_name
                .bind(&conference_link) // conference_url
                .bind(&conference_type) // conference_platform
                .bind(start_time) // start_time
                .bind(end_time) // end_time
                .bind(all_day.unwrap_or(false)) // is_all_day
                .bind(&status) // status
                .bind(None::<String>) // response_status (not available in stream table)
                .bind(stream_id) // source_stream_id
                .bind("stream_google_calendar") // source_table
                .bind("google") // source_provider
                .bind(&metadata) // metadata
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            event_id = %event_id,
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform calendar record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "calendar",
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
            "Google Calendar to activity_calendar_entry transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![], // Calendar transform doesn't chain to other transforms
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_metadata() {
        let transform = GoogleCalendarTransform;
        assert_eq!(transform.source_table(), "stream_google_calendar");
        assert_eq!(transform.target_table(), "activity_calendar_entry");
        assert_eq!(transform.domain(), "activity");
    }
}
