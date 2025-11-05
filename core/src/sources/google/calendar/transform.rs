//! Google Calendar to activity_calendar_entry ontology transformation
//!
//! Transforms raw calendar events from stream_google_calendar into the normalized
//! activity_calendar_entry ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
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

    #[tracing::instrument(skip(self, db), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting Google Calendar to activity_calendar_entry transformation"
        );

        // Query stream_google_calendar for records not yet transformed
        // Use left join to find records that don't exist in activity_calendar_entry
        let rows = sqlx::query(
            r#"
            SELECT
                c.id, c.event_id, c.calendar_id,
                c.summary, c.description, c.location, c.status,
                c.start_time, c.end_time, c.all_day, c.timezone,
                c.organizer_email, c.organizer_name, c.attendee_count,
                c.has_conferencing, c.conference_type, c.conference_link,
                c.raw_json
            FROM elt.stream_google_calendar c
            LEFT JOIN elt.activity_calendar_entry a ON (a.source_stream_id = c.id)
            WHERE c.source_id = $1
              AND a.id IS NULL
            ORDER BY c.start_time ASC
            LIMIT 1000
            "#,
        )
        .bind(source_id)
        .fetch_all(db.pool())
        .await?;

        tracing::debug!(
            records_to_transform = rows.len(),
            "Fetched untransformed calendar records"
        );

        for row in rows {
            records_read += 1;

            // Extract fields from row
            let stream_id: Uuid = row.try_get("id")?;
            let event_id: String = row.try_get("event_id")?;
            let calendar_id: String = row.try_get("calendar_id")?;
            let summary: Option<String> = row.try_get("summary")?;
            let description: Option<String> = row.try_get("description")?;
            let location: Option<String> = row.try_get("location")?;
            let status: Option<String> = row.try_get("status")?;
            let start_time: DateTime<Utc> = row.try_get("start_time")?;
            let end_time: DateTime<Utc> = row.try_get("end_time")?;
            let all_day: Option<bool> = row.try_get("all_day")?;
            let organizer_email: Option<String> = row.try_get("organizer_email")?;
            let _organizer_name: Option<String> = row.try_get("organizer_name")?;
            let attendee_count: Option<i32> = row.try_get("attendee_count")?;
            let has_conferencing: Option<bool> = row.try_get("has_conferencing")?;
            let conference_type: Option<String> = row.try_get("conference_type")?;
            let conference_link: Option<String> = row.try_get("conference_link")?;
            let raw_json: serde_json::Value = row.try_get("raw_json")?;

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
            .bind(&calendar_id) // calendar_name
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
