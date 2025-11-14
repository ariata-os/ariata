//! Google Calendar stream implementation

pub mod transform;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    client::GoogleClient,
    config::GoogleCalendarConfig,
    types::{Event, EventsResponse},
};
use crate::{
    error::Result,
    sources::{
        auth::SourceAuth,
        base::{SyncMode, SyncResult},
        stream::Stream,
    },
    storage::stream_writer::StreamWriter,
};

/// Google Calendar stream
///
/// Syncs calendar events from Google Calendar API to object storage via StreamWriter.
pub struct GoogleCalendarStream {
    source_id: Uuid,
    client: GoogleClient,
    db: PgPool,
    stream_writer: Arc<Mutex<StreamWriter>>,
    config: GoogleCalendarConfig,
}

impl GoogleCalendarStream {
    /// Create a new calendar stream with SourceAuth and StreamWriter
    pub fn new(
        source_id: Uuid,
        db: PgPool,
        stream_writer: Arc<Mutex<StreamWriter>>,
        auth: SourceAuth,
    ) -> Self {
        // Extract token manager from auth
        let token_manager = auth
            .token_manager()
            .expect("GoogleCalendarStream requires OAuth2 auth")
            .clone();

        let client = GoogleClient::with_api(source_id, token_manager, "calendar", "v3");

        Self {
            source_id,
            client,
            db,
            stream_writer,
            config: GoogleCalendarConfig::default(),
        }
    }

    /// Load configuration from database (called by Stream trait)
    async fn load_config_internal(&mut self, db: &PgPool, source_id: Uuid) -> Result<()> {
        // Load from streams table only
        let result = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT config FROM streams WHERE source_id = $1 AND stream_name = 'calendar'",
        )
        .bind(source_id)
        .fetch_optional(db)
        .await?;

        if let Some((config_json,)) = result {
            if let Ok(config) = GoogleCalendarConfig::from_json(&config_json) {
                self.config = config;
            }
        }

        Ok(())
    }

    /// Sync calendar events with explicit sync mode
    #[tracing::instrument(skip(self), fields(source_id = %self.source_id, mode = ?sync_mode))]
    pub async fn sync_with_mode(&self, sync_mode: &SyncMode) -> Result<SyncResult> {
        tracing::info!("Starting calendar sync");

        // Execute the sync (logging is handled by job executor)
        self.sync_internal(sync_mode).await
    }

    /// Internal sync implementation
    #[tracing::instrument(skip(self), fields(source_id = %self.source_id, mode = ?sync_mode))]
    async fn sync_internal(&self, sync_mode: &SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();
        let mut records_fetched = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut next_cursor = None;

        // Start database transaction for atomicity
        let mut tx = self.db.begin().await?;

        // Get last sync token from database
        let last_sync_token = self.get_last_sync_token().await?;

        // Use calendars from configuration
        let calendars = self.config.calendar_ids.clone();

        for calendar_id in &calendars {
            tracing::debug!(calendar_id = %calendar_id, "Syncing calendar");

            let result = if let Some(ref token) = last_sync_token {
                // Incremental sync using sync token
                self.sync_incremental(calendar_id, token).await?
            } else {
                // Full sync - get all events from configured time bounds
                self.sync_full(calendar_id).await?
            };

            records_fetched += result.items.len();

            tracing::debug!(
                items = result.items.len(),
                has_page_token = result.next_page_token.is_some(),
                has_sync_token = result.next_sync_token.is_some(),
                "Response metadata"
            );

            // Process events within transaction
            for event in result.items {
                match self
                    .upsert_event_with_tx(calendar_id, &event, &mut tx)
                    .await
                {
                    Ok(true) => records_written += 1,
                    Ok(false) => {
                        // Event skipped (missing required fields)
                        records_failed += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            event_id = %event.id,
                            error = %e,
                            "Failed to upsert event"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Save new sync token within transaction
            if let Some(token) = result.next_sync_token {
                tracing::info!("Saving sync token: {}", &token);
                self.save_sync_token_with_tx(&token, &mut tx).await?;
                next_cursor = Some(token);
            } else {
                tracing::warn!("No sync token returned from API response");
            }
        }

        // Commit transaction - all or nothing
        tx.commit().await?;

        let completed_at = Utc::now();

        Ok(SyncResult {
            records_fetched,
            records_written,
            records_failed,
            next_cursor,
            started_at,
            completed_at,
            records: None, // Google Calendar uses database, not direct transform
            archive_job_id: None,
        })
    }

    /// Get events using sync token (incremental sync)
    async fn sync_incremental(
        &self,
        calendar_id: &str,
        sync_token: &str,
    ) -> Result<EventsResponse> {
        let params = vec![("syncToken", sync_token)];

        match self
            .client
            .get_with_params(&format!("calendars/{calendar_id}/events"), &params)
            .await
        {
            Ok(response) => Ok(response),
            Err(e) if GoogleClient::is_sync_token_error(&e) => {
                // Sync token is invalid, clear it and do full sync
                self.clear_sync_token().await?;
                self.sync_full(calendar_id).await
            }
            Err(e) => Err(e),
        }
    }

    /// Get all events (full sync) with pagination
    async fn sync_full(&self, calendar_id: &str) -> Result<EventsResponse> {
        // Calculate time bounds based on configuration
        let (min_time_dt, max_time_dt) = self.config.calculate_time_bounds();

        let mut all_events = Vec::new();
        let mut page_token: Option<String> = None;
        let mut final_sync_token: Option<String> = None;

        loop {
            let mut params = vec![
                ("maxResults", self.config.max_events_per_sync.to_string()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "updated".to_string()),
                ("showDeleted", "false".to_string()),
                ("showHiddenInvitations", "false".to_string()),
            ];

            if let Some(min) = min_time_dt {
                params.push(("timeMin", min.to_rfc3339()));
            }
            if let Some(max) = max_time_dt {
                params.push(("timeMax", max.to_rfc3339()));
            }

            // Add page token if we have one
            if let Some(ref token) = page_token {
                params.push(("pageToken", token.clone()));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();

            let response: EventsResponse = self
                .client
                .get_with_params(&format!("calendars/{calendar_id}/events"), &param_refs)
                .await?;

            // Accumulate events from this page
            all_events.extend(response.items);

            // Save the sync token from the last page
            if response.next_sync_token.is_some() {
                final_sync_token = response.next_sync_token;
            }

            // Check if there are more pages
            page_token = response.next_page_token;
            if page_token.is_none() {
                break;
            }

            tracing::debug!(
                events_so_far = all_events.len(),
                has_more = page_token.is_some(),
                "Fetched calendar events page"
            );
        }

        tracing::info!(
            total_events = all_events.len(),
            calendar_id = %calendar_id,
            "Completed paginated calendar sync"
        );

        Ok(EventsResponse {
            items: all_events,
            next_sync_token: final_sync_token,
            next_page_token: None, // All pages consumed
        })
    }

    /// Insert or update an event within a transaction
    async fn upsert_event_with_tx(
        &self,
        calendar_id: &str,
        event: &Event,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<bool> {
        // Extract key fields - handle both datetime and date formats
        let start_time = if let Some(start) = event.start.as_ref() {
            if let Some(dt_str) = &start.date_time {
                DateTime::parse_from_rfc3339(dt_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else if let Some(date_str) = &start.date {
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc())
            } else {
                None
            }
        } else {
            None
        };

        let end_time = if let Some(end) = event.end.as_ref() {
            if let Some(dt_str) = &end.date_time {
                DateTime::parse_from_rfc3339(dt_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else if let Some(date_str) = &end.date {
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc())
            } else {
                None
            }
        } else {
            None
        };

        // Destructure times - both must be present
        let (start_time, end_time) = match (start_time, end_time) {
            (Some(start), Some(end)) => (start, end),
            _ => return Ok(false), // Skip events without proper times
        };

        let all_day = event.start.as_ref().and_then(|s| s.date.as_ref()).is_some();

        // Check if event is cancelled (we still store it but mark status)
        let status = event
            .status
            .clone()
            .unwrap_or_else(|| "confirmed".to_string());

        // Extract organizer and creator
        let organizer_email = event.organizer.as_ref().and_then(|o| o.email.clone());
        let organizer_name = event
            .organizer
            .as_ref()
            .and_then(|o| o.display_name.clone());
        let creator_email = event.creator.as_ref().and_then(|c| c.email.clone());
        let creator_name = event.creator.as_ref().and_then(|c| c.display_name.clone());

        // Count attendees
        let attendee_count = event.attendees.as_ref().map(|a| a.len()).unwrap_or(0) as i32;

        // Check for conferencing
        let has_conferencing = event.conference_data.is_some();
        let conference_type = event
            .conference_data
            .as_ref()
            .and_then(|c| c.conference_solution.as_ref())
            .and_then(|s| s.name.clone());
        let conference_link = event
            .conference_data
            .as_ref()
            .and_then(|c| c.entry_points.as_ref())
            .and_then(|eps| eps.first())
            .and_then(|ep| ep.uri.clone());

        // Build complete record with all parsed fields for storage
        let created_by_google = event
            .created
            .as_ref()
            .and_then(|c| DateTime::parse_from_rfc3339(c).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let updated_by_google = event
            .updated
            .as_ref()
            .and_then(|u| DateTime::parse_from_rfc3339(u).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let record = serde_json::json!({
            "event_id": event.id,
            "calendar_id": calendar_id,
            "etag": event.etag,
            "summary": event.summary,
            "description": event.description,
            "location": event.location,
            "status": status,
            "start_time": start_time,
            "end_time": end_time,
            "all_day": all_day,
            "timezone": event.start.as_ref().and_then(|s| s.time_zone.as_ref()),
            "organizer_email": organizer_email,
            "organizer_name": organizer_name,
            "creator_email": creator_email,
            "creator_name": creator_name,
            "attendee_count": attendee_count,
            "has_conferencing": has_conferencing,
            "conference_type": conference_type,
            "conference_link": conference_link,
            "created_by_google": created_by_google,
            "updated_by_google": updated_by_google,
            "is_recurring": event.recurring_event_id.is_some(),
            "recurring_event_id": event.recurring_event_id,
            "raw_event": event,
            "synced_at": Utc::now(),
        });

        // Write to S3/object storage via StreamWriter
        {
            let mut writer = self.stream_writer.lock().await;
            writer.write_record(self.source_id, "calendar", record, Some(start_time))?;
        }

        tracing::debug!(event_id = %event.id, "Wrote calendar event to object storage");
        Ok(true)
    }

    /// Get the last sync token from the database (streams table only)
    async fn get_last_sync_token(&self) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT last_sync_token FROM streams WHERE source_id = $1 AND stream_name = 'calendar'",
        )
        .bind(self.source_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.and_then(|(token,)| token))
    }

    /// Save the sync token to the database within a transaction
    async fn save_sync_token_with_tx(
        &self,
        token: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE streams SET last_sync_token = $1, last_sync_at = $2 WHERE source_id = $3 AND stream_name = 'calendar'"
        )
        .bind(token)
        .bind(Utc::now())
        .bind(self.source_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Clear the sync token (used when token is invalid)
    async fn clear_sync_token(&self) -> Result<()> {
        sqlx::query(
            "UPDATE streams SET last_sync_token = NULL WHERE source_id = $1 AND stream_name = 'calendar'"
        )
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

// Implement Stream trait for GoogleCalendarStream
#[async_trait]
impl Stream for GoogleCalendarStream {
    async fn sync(&self, mode: SyncMode) -> Result<SyncResult> {
        self.sync_with_mode(&mode).await
    }

    fn table_name(&self) -> &str {
        "stream_google_calendar"
    }

    fn stream_name(&self) -> &str {
        "calendar"
    }

    fn source_name(&self) -> &str {
        "google"
    }

    async fn load_config(&mut self, db: &PgPool, source_id: Uuid) -> Result<()> {
        self.load_config_internal(db, source_id).await
    }

    fn supports_incremental(&self) -> bool {
        true
    }

    fn supports_full_refresh(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {

    use crate::sources::base::SyncStrategy;
    use crate::sources::google::config::GoogleCalendarConfig;
    use chrono::{Duration, Utc};

    #[test]
    fn test_config_time_bounds_lookback() {
        // Test lookback sync with TimeWindow strategy
        let config = GoogleCalendarConfig {
            sync_strategy: SyncStrategy::TimeWindow { days_back: 30 },
            ..Default::default()
        };

        let (min, max) = config.calculate_time_bounds();
        let now = Utc::now();

        assert!(min.is_some());
        assert!(max.is_some());

        let min_time = min.unwrap();
        let max_time = max.unwrap();

        // Check that we're looking 30 days in the past
        let expected_min = now - Duration::days(30);
        let diff = (min_time - expected_min).num_seconds().abs();
        assert!(diff < 60, "Min time should be ~30 days ago");

        let diff = (max_time - now).num_seconds().abs();
        assert!(diff < 60, "Max time should be ~now");
    }

    #[test]
    fn test_config_time_bounds_full_history() {
        // Test full history strategy
        let config = GoogleCalendarConfig {
            sync_strategy: SyncStrategy::FullHistory { max_records: None },
            ..Default::default()
        };

        let (min, max) = config.calculate_time_bounds();

        assert!(min.is_none(), "Full history should have no min bound");
        assert!(max.is_none(), "Full history should have no max bound");
    }
}
