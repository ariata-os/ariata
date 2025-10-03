//! Google Calendar stream implementation

pub mod processor;
pub mod transformer;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::{
    error::Result,
    oauth::OAuthManager,
    sources::{DataSource, SourceRecord, SyncState},
    scheduler::{SyncTask, SyncResult},
};

use super::{
    auth::GoogleAuth,
    client::GoogleApiClient,
    types::{Calendar, CalendarListResponse, EventsResponse},
};

use processor::CalendarProcessor;

/// Google Calendar source
pub struct GoogleCalendarSource {
    auth: GoogleAuth,
    client: GoogleApiClient,
    processor: CalendarProcessor,
    calendar_id: String,
}

impl GoogleCalendarSource {
    /// Create a new Google Calendar source
    pub fn new(oauth: Arc<OAuthManager>) -> Self {
        Self {
            auth: GoogleAuth::new(oauth),
            client: GoogleApiClient::with_api("calendar", "v3"),
            processor: CalendarProcessor::new("google_calendar"),
            calendar_id: "primary".to_string(),
        }
    }

    /// Set specific calendar ID
    pub fn with_calendar_id(mut self, calendar_id: String) -> Self {
        self.calendar_id = calendar_id;
        self
    }

    /// List available calendars
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let token = self.auth.get_token().await?;
        let response: CalendarListResponse = self.client
            .get("users/me/calendarList", &token)
            .await?;
        Ok(response.items)
    }

    /// Fetch events with incremental sync
    async fn fetch_events(&self, sync_token: Option<String>) -> Result<EventsResponse> {
        let token = self.auth.get_token().await?;

        let mut params = vec![
            ("maxResults", "250"),
            ("singleEvents", "true"),
        ];

        if let Some(sync_token) = sync_token.as_ref() {
            params.push(("syncToken", sync_token.as_str()));
        }

        self.client.get_with_params(
            &format!("calendars/{}/events", self.calendar_id),
            &token,
            &params
        ).await
    }
}

#[async_trait]
impl DataSource for GoogleCalendarSource {
    fn name(&self) -> &str {
        "google_calendar"
    }

    fn requires_oauth(&self) -> bool {
        true
    }

    async fn fetch(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<SourceRecord>> {
        let state = self.get_sync_state().await?;
        let response = self.fetch_events(state.sync_token.clone()).await?;

        let records = self.processor.process_events(response.items, &self.calendar_id)?;

        if let Some(next_sync_token) = response.next_sync_token {
            let new_state = SyncState {
                source: state.source,
                sync_token: Some(next_sync_token),
                last_sync: Some(Utc::now()),
                cursor: state.cursor,
                checkpoint: state.checkpoint,
            };
            self.update_sync_state(new_state).await?;
        }

        Ok(records)
    }

    async fn get_sync_state(&self) -> Result<SyncState> {
        // TODO: Load from database
        Ok(SyncState {
            source: self.name().to_string(),
            last_sync: None,
            sync_token: None,
            cursor: None,
            checkpoint: None,
        })
    }

    async fn update_sync_state(&self, state: SyncState) -> Result<()> {
        // TODO: Save to database
        tracing::debug!("Updated sync state for {}: {:?}", self.name(), state);
        Ok(())
    }
}

#[async_trait]
impl SyncTask for GoogleCalendarSource {
    fn source_name(&self) -> &str {
        self.name()
    }

    async fn sync(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        match self.fetch(None).await {
            Ok(records) => {
                let count = records.len();

                // TODO: Store records
                for record in &records {
                    tracing::debug!("Synced event: {}", record.id);
                }

                Ok(SyncResult {
                    source: self.name().to_string(),
                    records_synced: count,
                    duration_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                Ok(SyncResult {
                    source: self.name().to_string(),
                    records_synced: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}