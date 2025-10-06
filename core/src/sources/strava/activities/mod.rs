//! Strava activities stream implementation

// Processor and transformer no longer needed - we write directly to database
// pub mod processor;
// pub mod transformer;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::Result,
    oauth::token_manager::TokenManager,
    sources::base::{
        DataSource, HealthStatus, StravaTimestampCursor, SyncCursor, SyncLogger, SyncMode,
        SyncResult, TypedCursor,
    },
};

use super::{client::StravaApiClient, types::Activity};

/// Strava activities source
pub struct StravaActivitiesSource {
    source_id: Uuid,
    db: PgPool,
    token_manager: Arc<TokenManager>,
    client: StravaApiClient,
}

impl StravaActivitiesSource {
    /// Create a new Strava activities source
    pub fn new(source_id: Uuid, db: PgPool, token_manager: Arc<TokenManager>) -> Self {
        Self {
            source_id,
            db,
            token_manager,
            client: StravaApiClient::new(),
        }
    }

    /// Fetch activities from Strava
    async fn fetch_activities(
        &self,
        after: Option<i64>,
        before: Option<i64>,
    ) -> Result<Vec<Activity>> {
        let token = self
            .token_manager
            .get_valid_token(self.source_id)
            .await?;

        let mut params = vec![("per_page", "100".to_string())];

        if let Some(after_ts) = after {
            params.push(("after", after_ts.to_string()));
        }

        if let Some(before_ts) = before {
            params.push(("before", before_ts.to_string()));
        }

        let param_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.client
            .get_with_params("athlete/activities", &token, &param_refs)
            .await
    }

    /// Store an activity in the database
    async fn store_activity(&self, activity: Activity) -> Result<bool> {
        // Extract polyline if available
        let summary_polyline = activity
            .map
            .as_ref()
            .and_then(|m| m.summary_polyline.clone());

        sqlx::query(
            r#"
            INSERT INTO stream_strava_activities (
                source_id, activity_id, name, sport_type, distance_meters, moving_time_seconds,
                elapsed_time_seconds, total_elevation_gain_meters, start_date, start_date_local,
                timezone, average_speed_ms, max_speed_ms, average_cadence, average_watts,
                weighted_average_watts, kilojoules, has_heartrate, average_heartrate,
                max_heartrate, achievement_count, kudos_count, comment_count, athlete_count,
                photo_count, trainer, commute, manual, private, flagged, gear_id,
                summary_polyline, pr_count, synced_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34
            )
            ON CONFLICT (source_id, activity_id)
            DO UPDATE SET
                name = EXCLUDED.name,
                distance_meters = EXCLUDED.distance_meters,
                moving_time_seconds = EXCLUDED.moving_time_seconds,
                kudos_count = EXCLUDED.kudos_count,
                comment_count = EXCLUDED.comment_count,
                athlete_count = EXCLUDED.athlete_count,
                photo_count = EXCLUDED.photo_count,
                synced_at = EXCLUDED.synced_at,
                updated_at = NOW()
            "#,
        )
        .bind(self.source_id)
        .bind(activity.id)
        .bind(&activity.name)
        .bind(&activity.sport_type)
        .bind(activity.distance)
        .bind(activity.moving_time)
        .bind(activity.elapsed_time)
        .bind(activity.total_elevation_gain)
        .bind(activity.start_date)
        .bind(activity.start_date_local)
        .bind(&activity.timezone)
        .bind(activity.average_speed)
        .bind(activity.max_speed)
        .bind(activity.average_cadence)
        .bind(activity.average_watts)
        .bind(activity.weighted_average_watts)
        .bind(activity.kilojoules)
        .bind(activity.has_heartrate)
        .bind(activity.average_heartrate)
        .bind(activity.max_heartrate)
        .bind(activity.achievement_count)
        .bind(activity.kudos_count)
        .bind(activity.comment_count)
        .bind(activity.athlete_count)
        .bind(activity.photo_count)
        .bind(activity.trainer)
        .bind(activity.commute)
        .bind(activity.manual)
        .bind(activity.private)
        .bind(activity.flagged)
        .bind(activity.gear_id)
        .bind(summary_polyline)
        .bind(activity.pr_count)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;

        Ok(true)
    }

    /// Get the last cursor from the database
    async fn get_last_cursor(&self) -> Result<Option<StravaTimestampCursor>> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT cursor_metadata FROM sources WHERE id = $1",
        )
        .bind(self.source_id)
        .fetch_one(&self.db)
        .await?;

        if let Some(json) = row.0 {
            let typed = TypedCursor::from_json(&json)?;
            Ok(Some(typed.decode::<StravaTimestampCursor>()?))
        } else {
            Ok(None)
        }
    }

    /// Save the cursor to the database
    async fn save_cursor(&self, cursor: &StravaTimestampCursor) -> Result<()> {
        let typed = TypedCursor::new(cursor);

        sqlx::query(
            "UPDATE sources SET cursor_metadata = $1, cursor_type = $2, last_sync_at = $3 WHERE id = $4",
        )
        .bind(typed.to_json())
        .bind(StravaTimestampCursor::cursor_type())
        .bind(Utc::now())
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get the last sync timestamp
    async fn get_last_sync_at(&self) -> Result<Option<chrono::DateTime<Utc>>> {
        let row = sqlx::query_as::<_, (Option<chrono::DateTime<Utc>>,)>(
            "SELECT last_sync_at FROM sources WHERE id = $1",
        )
        .bind(self.source_id)
        .fetch_one(&self.db)
        .await?;

        Ok(row.0)
    }

    /// Internal sync implementation
    async fn sync_internal(&self, mode: &SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();
        let mut records_fetched = 0;
        let mut records_written = 0;
        let mut records_failed = 0;

        match mode {
            SyncMode::Incremental { cursor } => {
                // Try to use cursor from mode, or load from database
                let strava_cursor = if let Some(cursor_str) = cursor {
                    Some(StravaTimestampCursor::from_string(cursor_str)?)
                } else {
                    self.get_last_cursor().await?
                };

                let after_ts = strava_cursor.as_ref().and_then(|c| c.after);
                let before_ts = strava_cursor.as_ref().and_then(|c| c.before);

                let activities = self.fetch_activities(after_ts, before_ts).await?;

                // Find the latest activity timestamp for next sync
                let mut latest_ts: Option<i64> = None;

                for activity in activities {
                    records_fetched += 1;

                    let activity_ts = activity.start_date.timestamp();
                    if latest_ts.is_none() || latest_ts.unwrap() < activity_ts {
                        latest_ts = Some(activity_ts);
                    }

                    match self.store_activity(activity).await {
                        Ok(true) => records_written += 1,
                        Ok(false) => records_failed += 1,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to store activity");
                            records_failed += 1;
                        }
                    }
                }

                // Save cursor with the latest timestamp as "after" for next sync
                if let Some(ts) = latest_ts {
                    let new_cursor = StravaTimestampCursor {
                        before: None,
                        after: Some(ts),
                    };
                    self.save_cursor(&new_cursor).await?;
                }
            }
            SyncMode::FullRefresh => {
                // Full refresh - fetch all activities
                let activities = self.fetch_activities(None, None).await?;

                // Find the latest activity timestamp
                let mut latest_ts: Option<i64> = None;

                for activity in activities {
                    records_fetched += 1;

                    let activity_ts = activity.start_date.timestamp();
                    if latest_ts.is_none() || latest_ts.unwrap() < activity_ts {
                        latest_ts = Some(activity_ts);
                    }

                    match self.store_activity(activity).await {
                        Ok(true) => records_written += 1,
                        Ok(false) => records_failed += 1,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to store activity");
                            records_failed += 1;
                        }
                    }
                }

                // Save cursor for future incremental syncs
                if let Some(ts) = latest_ts {
                    let cursor = StravaTimestampCursor {
                        before: None,
                        after: Some(ts),
                    };
                    self.save_cursor(&cursor).await?;
                }
            }
        }

        let completed_at = Utc::now();

        Ok(SyncResult {
            records_fetched,
            records_written,
            records_failed,
            next_cursor: None, // Strava doesn't return a cursor, we calculate it
            started_at,
            completed_at,
        })
    }
}

// Implement DataSource trait
#[async_trait]
impl DataSource for StravaActivitiesSource {
    fn source_id(&self) -> Uuid {
        self.source_id
    }

    fn source_type(&self) -> &str {
        "strava"
    }

    fn stream_name(&self) -> &str {
        "activities"
    }

    fn validate_config(_config: &Value) -> Result<()>
    where
        Self: Sized,
    {
        // Strava activities doesn't require specific config validation
        Ok(())
    }

    fn required_scopes(&self) -> Vec<String> {
        vec![
            "read".to_string(),
            "activity:read".to_string(),
            "activity:read_all".to_string(),
        ]
    }

    async fn sync(&self, mode: &SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();
        let logger = SyncLogger::new(self.db.clone());

        tracing::info!(mode = ?mode, "Starting Strava activities sync");

        // Execute the sync
        match self.sync_internal(mode).await {
            Ok(result) => {
                // Log success to database
                if let Err(e) = logger.log_success(self.source_id, mode, &result).await {
                    tracing::warn!(error = %e, "Failed to log sync success");
                }

                Ok(result)
            }
            Err(e) => {
                // Log failure to database
                if let Err(log_err) = logger
                    .log_failure(self.source_id, mode, started_at, &e)
                    .await
                {
                    tracing::warn!(error = %log_err, "Failed to log sync failure");
                }

                Err(e)
            }
        }
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        // Check if we can get a valid token
        let token_check = self
            .token_manager
            .get_valid_token(self.source_id)
            .await;

        let last_sync = self.get_last_sync_at().await.unwrap_or(None);

        match token_check {
            Ok(_) => Ok(HealthStatus {
                is_healthy: true,
                message: "Strava OAuth token is valid".to_string(),
                last_check: Utc::now(),
                last_successful_sync: last_sync,
                error_count: 0,
            }),
            Err(e) => Ok(HealthStatus {
                is_healthy: false,
                message: format!("Strava OAuth token error: {e}"),
                last_check: Utc::now(),
                last_successful_sync: last_sync,
                error_count: 1,
            }),
        }
    }

    fn sync_schedule(&self) -> String {
        // Sync every 15 minutes (athletes might finish activities)
        "0 */15 * * * *".to_string()
    }

    async fn should_sync(&self) -> bool {
        // Check if OAuth token is valid
        match self.token_manager.get_valid_token(self.source_id).await {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!(
                    source_id = %self.source_id,
                    error = %e,
                    "Strava sync skipped: no valid OAuth token"
                );
                false
            }
        }
    }
}
