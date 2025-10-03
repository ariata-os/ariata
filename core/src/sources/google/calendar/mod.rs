//! Google Calendar sync implementation

use chrono::{DateTime, Utc, NaiveDate};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::Result,
    oauth::token_manager::TokenManager,
};
use super::{
    client::GoogleClient,
    config::GoogleCalendarConfig,
    types::{EventsResponse, Event},
};

/// Google Calendar synchronization
pub struct GoogleCalendarSync {
    source_id: Uuid,
    client: GoogleClient,
    db: PgPool,
    config: GoogleCalendarConfig,
}

impl GoogleCalendarSync {
    /// Create a new calendar sync instance with a token manager
    pub fn new(source_id: Uuid, db: PgPool, token_manager: Arc<TokenManager>) -> Self {
        let client = GoogleClient::with_api(
            source_id,
            token_manager,
            "calendar",
            "v3"
        );

        Self {
            source_id,
            client,
            db,
            config: GoogleCalendarConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        source_id: Uuid,
        db: PgPool,
        token_manager: Arc<TokenManager>,
        config: GoogleCalendarConfig
    ) -> Self {
        let client = GoogleClient::with_api(
            source_id,
            token_manager,
            "calendar",
            "v3"
        );

        Self {
            source_id,
            client,
            db,
            config,
        }
    }

    /// Create with default token manager
    pub fn with_default_manager(source_id: Uuid, db: PgPool) -> Self {
        let token_manager = Arc::new(TokenManager::new(db.clone()));
        Self::new(source_id, db, token_manager)
    }

    /// Load configuration from database
    pub async fn load_config(&mut self) -> Result<()> {
        let result = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT config FROM sources WHERE id = $1"
        )
        .bind(self.source_id)
        .fetch_optional(&self.db)
        .await?;

        if let Some((config_json,)) = result {
            if let Ok(config) = GoogleCalendarConfig::from_json(&config_json) {
                self.config = config;
            }
        }

        Ok(())
    }

    /// Sync calendar events with automatic token refresh
    pub async fn sync(&self) -> Result<SyncStats> {
        // Token refresh is now handled automatically by the OAuthSource trait
        self.sync_internal().await
    }

    /// Internal sync implementation
    async fn sync_internal(&self) -> Result<SyncStats> {
        let mut stats = SyncStats::default();

        // Get last sync token from database
        let last_sync_token = self.get_last_sync_token().await?;

        // Use calendars from configuration
        let calendars = self.config.calendar_ids.clone();

        for calendar_id in calendars {
            let result = if let Some(ref token) = last_sync_token {
                // Incremental sync using sync token
                self.sync_incremental(&calendar_id, token).await?
            } else {
                // Full sync - get all events from last 90 days
                self.sync_full(&calendar_id).await?
            };

            // Process events
            for event in result.items {
                if self.upsert_event(&calendar_id, &event).await? {
                    stats.upserted += 1;
                } else {
                    stats.skipped += 1;
                }
            }

            // Save new sync token
            if let Some(token) = result.next_sync_token {
                self.save_sync_token(&token).await?;
            }
        }

        Ok(stats)
    }

    /// Get events using sync token (incremental sync)
    async fn sync_incremental(&self, calendar_id: &str, sync_token: &str) -> Result<EventsResponse> {
        let params = vec![
            ("syncToken", sync_token),
        ];

        match self.client.get_with_params(
            &format!("calendars/{}/events", calendar_id),
            &params
        ).await {
            Ok(response) => Ok(response),
            Err(e) if GoogleClient::is_sync_token_error(&e) => {
                // Sync token is invalid, clear it and do full sync
                self.clear_sync_token().await?;
                self.sync_full(calendar_id).await
            },
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

        let param_refs: Vec<(&str, &str)> = params.iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        self.client.get_with_params(
            &format!("calendars/{}/events", calendar_id),
            &param_refs
        ).await
    }

    /// Insert or update an event
    async fn upsert_event(&self, calendar_id: &str, event: &Event) -> Result<bool> {
        // Extract key fields - handle both datetime and date formats
        let start_time = if let Some(start) = event.start.as_ref() {
            if let Some(dt_str) = &start.date_time {
                DateTime::parse_from_rfc3339(dt_str).ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else if let Some(date_str) = &start.date {
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
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
                DateTime::parse_from_rfc3339(dt_str).ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else if let Some(date_str) = &end.date {
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc())
            } else {
                None
            }
        } else {
            None
        };

        if start_time.is_none() || end_time.is_none() {
            return Ok(false); // Skip events without proper times
        }

        let all_day = event.start.as_ref()
            .and_then(|s| s.date.as_ref())
            .is_some();

        // Check if event is cancelled (we still store it but mark status)
        let status = event.status.clone().unwrap_or_else(|| "confirmed".to_string());

        // Extract organizer and creator
        let organizer_email = event.organizer.as_ref().and_then(|o| o.email.clone());
        let organizer_name = event.organizer.as_ref().and_then(|o| o.display_name.clone());
        let creator_email = event.creator.as_ref().and_then(|c| c.email.clone());
        let creator_name = event.creator.as_ref().and_then(|c| c.display_name.clone());

        // Count attendees
        let attendee_count = event.attendees.as_ref().map(|a| a.len()).unwrap_or(0) as i32;

        // Check for conferencing
        let has_conferencing = event.conference_data.is_some();
        let conference_type = event.conference_data.as_ref()
            .and_then(|c| c.conference_solution.as_ref())
            .and_then(|s| s.name.clone());
        let conference_link = event.conference_data.as_ref()
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
            "#
        )
        .bind(self.source_id)
        .bind(&event.id)
        .bind(calendar_id)
        .bind(&event.etag)
        .bind(event.summary.as_deref())
        .bind(event.description.as_deref())
        .bind(event.location.as_deref())
        .bind(&status)
        .bind(start_time.unwrap())
        .bind(end_time.unwrap())
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
        .bind(event.created.as_ref().and_then(|c| DateTime::parse_from_rfc3339(c).ok()).map(|dt| dt.with_timezone(&Utc)))
        .bind(event.updated.as_ref().and_then(|u| DateTime::parse_from_rfc3339(u).ok()).map(|dt| dt.with_timezone(&Utc)))
        .bind(event.recurring_event_id.is_some())
        .bind(event.recurring_event_id.as_deref())
        .bind(raw_json)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;

        Ok(true)
    }

    /// Get the last sync token from the database
    async fn get_last_sync_token(&self) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT last_sync_token FROM sources WHERE id = $1"
        )
        .bind(self.source_id)
        .fetch_one(&self.db)
        .await?;

        Ok(row.0)
    }

    /// Save the sync token to the database
    async fn save_sync_token(&self, token: &str) -> Result<()> {
        sqlx::query(
            "UPDATE sources SET last_sync_token = $1, last_sync_at = $2 WHERE id = $3"
        )
        .bind(token)
        .bind(Utc::now())
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Clear the sync token (used when token is invalid)
    async fn clear_sync_token(&self) -> Result<()> {
        sqlx::query(
            "UPDATE sources SET last_sync_token = NULL WHERE id = $1"
        )
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

}

/// Statistics from a sync operation
#[derive(Debug, Default)]
pub struct SyncStats {
    pub upserted: usize,
    pub skipped: usize,
}

#[cfg(test)]
mod test;