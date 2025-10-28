//! Google Calendar stream implementation

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
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
        base::{SyncLogger, SyncMode, SyncResult},
        stream::Stream,
    },
};

/// Google Calendar stream
///
/// Syncs calendar events from Google Calendar API to the stream_google_calendar table.
pub struct GoogleCalendarStream {
    source_id: Uuid,
    client: GoogleClient,
    db: PgPool,
    config: GoogleCalendarConfig,
}

impl GoogleCalendarStream {
    /// Create a new calendar stream with SourceAuth
    pub fn new(source_id: Uuid, db: PgPool, auth: SourceAuth) -> Self {
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
        let started_at = Utc::now();
        let logger = SyncLogger::new(self.db.clone());

        tracing::info!("Starting calendar sync");

        // Execute the sync
        match self.sync_internal(sync_mode).await {
            Ok(result) => {
                // Log success to database
                if let Err(e) = logger
                    .log_success(self.source_id, "calendar", sync_mode, &result)
                    .await
                {
                    tracing::warn!(error = %e, "Failed to log sync success");
                }

                Ok(result)
            }
            Err(e) => {
                // Log failure to database
                if let Err(log_err) = logger
                    .log_failure(self.source_id, "calendar", sync_mode, started_at, &e)
                    .await
                {
                    tracing::warn!(error = %log_err, "Failed to log sync failure");
                }

                Err(e)
            }
        }
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
                self.save_sync_token_with_tx(&token, &mut tx).await?;
                next_cursor = Some(token);
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

    /// Get all events (full sync)
    async fn sync_full(&self, calendar_id: &str) -> Result<EventsResponse> {
        // Calculate time bounds based on configuration
        let (min_time_dt, max_time_dt) = self.config.calculate_time_bounds();

        let mut params = vec![
            ("maxResults", self.config.max_events_per_sync.to_string()),
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
        ];

        if let Some(min) = min_time_dt {
            params.push(("timeMin", min.to_rfc3339()));
        }
        if let Some(max) = max_time_dt {
            params.push(("timeMax", max.to_rfc3339()));
        }

        let param_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.client
            .get_with_params(&format!("calendars/{calendar_id}/events"), &param_refs)
            .await
    }

    /// Insert or update an event
    async fn upsert_event(&self, calendar_id: &str, event: &Event) -> Result<bool> {
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

        // Serialize full event as JSON
        let raw_json = serde_json::to_value(event)?;

        // Upsert into database
        sqlx::query(
            r#"
            INSERT INTO stream_google_calendar (
                source_id, event_id, calendar_id, etag,
                summary, description, location, status,
                start_time, end_time, all_day, timezone,
                organizer_email, organizer_name, creator_email, creator_name,
                attendee_count, has_conferencing, conference_type, conference_link,
                created_by_google, updated_by_google,
                is_recurring, recurring_event_id,
                raw_json, synced_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26
            )
            ON CONFLICT (source_id, event_id)
            DO UPDATE SET
                etag = EXCLUDED.etag,
                summary = EXCLUDED.summary,
                description = EXCLUDED.description,
                location = EXCLUDED.location,
                status = EXCLUDED.status,
                start_time = EXCLUDED.start_time,
                end_time = EXCLUDED.end_time,
                all_day = EXCLUDED.all_day,
                organizer_email = EXCLUDED.organizer_email,
                organizer_name = EXCLUDED.organizer_name,
                attendee_count = EXCLUDED.attendee_count,
                has_conferencing = EXCLUDED.has_conferencing,
                conference_type = EXCLUDED.conference_type,
                conference_link = EXCLUDED.conference_link,
                updated_by_google = EXCLUDED.updated_by_google,
                raw_json = EXCLUDED.raw_json,
                synced_at = EXCLUDED.synced_at,
                updated_at = NOW()
            "#,
        )
        .bind(self.source_id)
        .bind(&event.id)
        .bind(calendar_id)
        .bind(&event.etag)
        .bind(event.summary.as_deref())
        .bind(event.description.as_deref())
        .bind(event.location.as_deref())
        .bind(&status)
        .bind(start_time)
        .bind(end_time)
        .bind(all_day)
        .bind(event.start.as_ref().and_then(|s| s.time_zone.as_deref()))
        .bind(organizer_email.as_deref())
        .bind(organizer_name.as_deref())
        .bind(creator_email.as_deref())
        .bind(creator_name.as_deref())
        .bind(attendee_count)
        .bind(has_conferencing)
        .bind(conference_type.as_deref())
        .bind(conference_link.as_deref())
        .bind(
            event
                .created
                .as_ref()
                .and_then(|c| DateTime::parse_from_rfc3339(c).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        )
        .bind(
            event
                .updated
                .as_ref()
                .and_then(|u| DateTime::parse_from_rfc3339(u).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        )
        .bind(event.recurring_event_id.is_some())
        .bind(event.recurring_event_id.as_deref())
        .bind(raw_json)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;

        Ok(true)
    }

    /// Insert or update an event within a transaction
    async fn upsert_event_with_tx(
        &self,
        calendar_id: &str,
        event: &Event,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

        // Serialize full event as JSON
        let raw_json = serde_json::to_value(event)?;

        // Upsert into database within transaction
        sqlx::query(
            r#"
            INSERT INTO stream_google_calendar (
                source_id, event_id, calendar_id, etag,
                summary, description, location, status,
                start_time, end_time, all_day, timezone,
                organizer_email, organizer_name, creator_email, creator_name,
                attendee_count, has_conferencing, conference_type, conference_link,
                created_by_google, updated_by_google,
                is_recurring, recurring_event_id,
                raw_json, synced_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26
            )
            ON CONFLICT (source_id, event_id)
            DO UPDATE SET
                etag = EXCLUDED.etag,
                summary = EXCLUDED.summary,
                description = EXCLUDED.description,
                location = EXCLUDED.location,
                status = EXCLUDED.status,
                start_time = EXCLUDED.start_time,
                end_time = EXCLUDED.end_time,
                all_day = EXCLUDED.all_day,
                organizer_email = EXCLUDED.organizer_email,
                organizer_name = EXCLUDED.organizer_name,
                attendee_count = EXCLUDED.attendee_count,
                has_conferencing = EXCLUDED.has_conferencing,
                conference_type = EXCLUDED.conference_type,
                conference_link = EXCLUDED.conference_link,
                updated_by_google = EXCLUDED.updated_by_google,
                raw_json = EXCLUDED.raw_json,
                synced_at = EXCLUDED.synced_at,
                updated_at = NOW()
            "#,
        )
        .bind(self.source_id)
        .bind(&event.id)
        .bind(calendar_id)
        .bind(&event.etag)
        .bind(event.summary.as_deref())
        .bind(event.description.as_deref())
        .bind(event.location.as_deref())
        .bind(&status)
        .bind(start_time)
        .bind(end_time)
        .bind(all_day)
        .bind(event.start.as_ref().and_then(|s| s.time_zone.as_deref()))
        .bind(organizer_email.as_deref())
        .bind(organizer_name.as_deref())
        .bind(creator_email.as_deref())
        .bind(creator_name.as_deref())
        .bind(attendee_count)
        .bind(has_conferencing)
        .bind(conference_type.as_deref())
        .bind(conference_link.as_deref())
        .bind(
            event
                .created
                .as_ref()
                .and_then(|c| DateTime::parse_from_rfc3339(c).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        )
        .bind(
            event
                .updated
                .as_ref()
                .and_then(|u| DateTime::parse_from_rfc3339(u).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        )
        .bind(event.recurring_event_id.is_some())
        .bind(event.recurring_event_id.as_deref())
        .bind(raw_json)
        .bind(Utc::now())
        .execute(&mut **tx)
        .await?;

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

    /// Save the sync token to the database (streams table only)
    async fn save_sync_token(&self, token: &str) -> Result<()> {
        sqlx::query(
            "UPDATE streams SET last_sync_token = $1, last_sync_at = $2 WHERE source_id = $3 AND stream_name = 'calendar'"
        )
        .bind(token)
        .bind(Utc::now())
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
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

    use crate::sources::google::config::{GoogleCalendarConfig, SyncDirection};
    use chrono::{Duration, Utc};

    #[test]
    fn test_config_time_bounds_past() {
        // Test past sync
        let config = GoogleCalendarConfig {
            sync_window_days: 30,
            sync_direction: SyncDirection::Past,
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
    fn test_config_time_bounds_future() {
        // Test future sync
        let config = GoogleCalendarConfig {
            sync_window_days: 7,
            sync_direction: SyncDirection::Future,
            ..Default::default()
        };

        let (min, max) = config.calculate_time_bounds();
        let now = Utc::now();

        assert!(min.is_some());
        assert!(max.is_some());

        let min_time = min.unwrap();
        let max_time = max.unwrap();

        let diff = (min_time - now).num_seconds().abs();
        assert!(diff < 60, "Min time should be ~now for future sync");

        let expected_max = now + Duration::days(7);
        let diff = (max_time - expected_max).num_seconds().abs();
        assert!(diff < 60, "Max time should be ~7 days from now");
    }
}
