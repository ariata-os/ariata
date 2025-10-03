//! Strava activities stream implementation

pub mod processor;
pub mod transformer;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::{
    error::Result,
    oauth::OAuthManager,
    sources::{DataSource, SourceRecord, SyncState},
};

use super::{
    auth::StravaAuth,
    client::StravaApiClient,
    types::Activity,
};

use processor::ActivitiesProcessor;

/// Strava activities source
pub struct StravaActivitiesSource {
    auth: StravaAuth,
    client: StravaApiClient,
    processor: ActivitiesProcessor,
}

impl StravaActivitiesSource {
    /// Create a new Strava activities source
    pub fn new(oauth: Arc<OAuthManager>) -> Self {
        Self {
            auth: StravaAuth::new(oauth),
            client: StravaApiClient::new(),
            processor: ActivitiesProcessor::new("strava_activities"),
        }
    }

    /// Fetch activities from Strava
    async fn fetch_activities(&self, after: Option<i64>, before: Option<i64>) -> Result<Vec<Activity>> {
        let token = self.auth.get_token().await?;

        let mut params = vec![
            ("per_page", "100"),
        ];

        // Convert timestamps to strings for params
        let after_str;
        let before_str;

        if let Some(after_ts) = after {
            after_str = after_ts.to_string();
            params.push(("after", &after_str));
        }

        if let Some(before_ts) = before {
            before_str = before_ts.to_string();
            params.push(("before", &before_str));
        }

        self.client.get_with_params("athlete/activities", &token, &params).await
    }
}

#[async_trait]
impl DataSource for StravaActivitiesSource {
    fn name(&self) -> &str {
        "strava_activities"
    }

    fn requires_oauth(&self) -> bool {
        true
    }

    async fn fetch(&self, since: Option<DateTime<Utc>>) -> Result<Vec<SourceRecord>> {
        let after_ts = since.map(|dt| dt.timestamp());
        let activities = self.fetch_activities(after_ts, None).await?;

        let records = self.processor.process_activities(activities)?;

        // Update sync state
        if !records.is_empty() {
            let new_state = SyncState {
                source: self.name().to_string(),
                last_sync: Some(Utc::now()),
                sync_token: None,
                cursor: None,
                checkpoint: None,
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