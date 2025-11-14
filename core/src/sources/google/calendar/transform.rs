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

/// Batch size for bulk inserts
const BATCH_SIZE: usize = 500;

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
    async fn transform(
        &self,
        db: &Database,
        context: &crate::jobs::transform_context::TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting Google Calendar to activity_calendar_entry transformation"
        );

        // Read stream data using data source (memory for hot path)
        let checkpoint_key = "calendar_to_activity_calendar_entry";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| {
            crate::Error::Other("No data source available for transform".to_string())
        })?;
        let batches = data_source
            .read_with_checkpoint(source_id, "calendar", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched Google Calendar batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(
            Option<String>,
            Option<String>,
            String,
            Option<&'static str>,
            Option<String>,
            Vec<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
            bool,
            Option<String>,
            Option<String>,
            Uuid,
            serde_json::Value,
        )> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            tracing::debug!(batch_record_count = batch.records.len(), "Processing batch");

            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(event_id) = record.get("event_id").and_then(|v| v.as_str()) else {
                    continue; // Skip records without event_id
                };

                let stream_id = record
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let calendar_id = record
                    .get("calendar_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let summary = record
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let description = record
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let location = record
                    .get("location")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let status = record
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let start_time = record
                    .get("start_time")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let end_time = record
                    .get("end_time")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let all_day = record
                    .get("all_day")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let organizer_email = record
                    .get("organizer_email")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let attendee_count = record
                    .get("attendee_count")
                    .and_then(|v| v.as_i64())
                    .map(|c| c as i32);

                let has_conferencing = record.get("has_conferencing").and_then(|v| v.as_bool());

                let conference_type = record
                    .get("conference_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let conference_link = record
                    .get("conference_link")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let raw_json = record
                    .get("raw_json")
                    .cloned()
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
                } else if all_day {
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

                // Add to pending batch
                pending_records.push((
                    summary,
                    description,
                    calendar_id,
                    event_type,
                    organizer_email,
                    attendee_identifiers,
                    location,
                    conference_link,
                    conference_type,
                    start_time,
                    end_time,
                    all_day,
                    status,
                    None, // response_status (not available in stream table)
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_calendar_batch_insert(db, &pending_records).await;
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
                data_source
                    .update_checkpoint(source_id, "calendar", checkpoint_key, max_ts)
                    .await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_calendar_batch_insert(db, &pending_records).await;
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

/// Execute batch insert for calendar records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_calendar_batch_insert(
    db: &Database,
    records: &[(
        Option<String>,
        Option<String>,
        String,
        Option<&str>,
        Option<String>,
        Vec<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
        DateTime<Utc>,
        bool,
        Option<String>,
        Option<String>,
        Uuid,
        serde_json::Value,
    )],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.activity_calendar_entry",
        &[
            "title",
            "description",
            "calendar_name",
            "event_type",
            "organizer_identifier",
            "attendee_identifiers",
            "location_name",
            "conference_url",
            "conference_platform",
            "start_time",
            "end_time",
            "is_all_day",
            "status",
            "response_status",
            "source_stream_id",
            "metadata",
            "source_table",
            "source_provider",
        ],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (
        title,
        description,
        calendar_name,
        event_type,
        organizer_identifier,
        attendee_identifiers,
        location_name,
        conference_url,
        conference_platform,
        start_time,
        end_time,
        is_all_day,
        status,
        response_status,
        stream_id,
        metadata,
    ) in records
    {
        query = query
            .bind(title)
            .bind(description)
            .bind(calendar_name)
            .bind(event_type)
            .bind(organizer_identifier)
            .bind(attendee_identifiers)
            .bind(location_name)
            .bind(conference_url)
            .bind(conference_platform)
            .bind(start_time)
            .bind(end_time)
            .bind(is_all_day)
            .bind(status)
            .bind(response_status)
            .bind(stream_id)
            .bind(metadata)
            .bind("stream_google_calendar")
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
        let transform = GoogleCalendarTransform;
        assert_eq!(transform.source_table(), "stream_google_calendar");
        assert_eq!(transform.target_table(), "activity_calendar_entry");
        assert_eq!(transform.domain(), "activity");
    }
}
