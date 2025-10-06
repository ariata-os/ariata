//! Built-in task scheduler for periodic syncs

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    database::Database,
    oauth::OAuthManager,
    sources::{DataSource, base::{SyncMode, SyncResult}},
};

/// Schedule configuration for a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub source_id: Uuid,
    pub source_type: String,
    pub stream_name: String,
    pub cron_expression: String,  // e.g., "0 */5 * * * *" for every 5 minutes
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
}

/// Main scheduler for managing periodic syncs
pub struct Scheduler {
    db: Arc<Database>,
    oauth: Arc<OAuthManager>,
    scheduler: Arc<RwLock<JobScheduler>>,
    sources: Arc<RwLock<Vec<Box<dyn DataSource>>>>,
}

impl Scheduler {
    /// Create a new scheduler
    pub async fn new(db: Arc<Database>, oauth: Arc<OAuthManager>) -> Result<Self> {
        let scheduler = JobScheduler::new().await
            .map_err(|e| Error::Other(format!("Failed to create scheduler: {e}")))?;

        Ok(Self {
            db,
            oauth,
            scheduler: Arc::new(RwLock::new(scheduler)),
            sources: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register a data source for scheduling
    pub async fn register_source(&self, source: Box<dyn DataSource>) {
        let mut sources = self.sources.write().await;
        sources.push(source);
    }

    /// Add a scheduled sync for a source
    pub async fn add_schedule(&self, config: ScheduleConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        let source_id = config.source_id;
        let sources = self.sources.clone();

        // Create a cron job
        let job = Job::new_async(config.cron_expression.as_str(), move |_uuid, _lock| {
            let source_id = source_id;
            let sources = sources.clone();

            Box::pin(async move {
                // Find the source
                let sources_guard = sources.read().await;
                let source = sources_guard.iter()
                    .find(|s| s.source_id() == source_id);

                if let Some(source) = source {
                    if source.should_sync().await {
                        // Determine sync mode (for now, use incremental)
                        let mode = SyncMode::Incremental { cursor: None };

                        match source.sync(&mode).await {
                            Ok(result) => {
                                tracing::info!(
                                    source_id = %source_id,
                                    source_type = source.source_type(),
                                    stream = source.stream_name(),
                                    records_fetched = result.records_fetched,
                                    records_written = result.records_written,
                                    duration_ms = (result.completed_at - result.started_at).num_milliseconds(),
                                    "Scheduled sync completed"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    source_id = %source_id,
                                    error = %e,
                                    "Scheduled sync failed"
                                );
                            }
                        }
                    }
                } else {
                    tracing::warn!(source_id = %source_id, "No source found for scheduled sync");
                }
            })
        }).map_err(|e| Error::Other(format!("Failed to create job: {e}")))?;

        let scheduler = self.scheduler.write().await;
        scheduler.add(job).await
            .map_err(|e| Error::Other(format!("Failed to add job: {e}")))?;

        // Store schedule in database
        self.save_schedule(&config).await?;

        Ok(())
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        // Load schedules from database
        let schedules = self.load_schedules().await?;

        for schedule in schedules {
            if schedule.enabled {
                self.add_schedule(schedule).await?;
            }
        }

        // Add token refresh job (every 30 minutes)
        self.add_token_refresh_job().await?;

        // Add cleanup job (daily at 2 AM)
        self.add_cleanup_job().await?;

        // Start the scheduler
        let scheduler = self.scheduler.write().await;
        scheduler.start().await
            .map_err(|e| Error::Other(format!("Failed to start scheduler: {e}")))?;

        tracing::info!("Scheduler started with {} sources", self.sources.read().await.len());

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<()> {
        let mut scheduler = self.scheduler.write().await;
        scheduler.shutdown().await
            .map_err(|e| Error::Other(format!("Failed to stop scheduler: {e}")))?;

        tracing::info!("Scheduler stopped");
        Ok(())
    }

    /// Add job to refresh expiring OAuth tokens
    async fn add_token_refresh_job(&self) -> Result<()> {
        let oauth = self.oauth.clone();

        let job = Job::new_async("0 */30 * * * *", move |_uuid, _lock| {
            let oauth = oauth.clone();

            Box::pin(async move {
                tracing::debug!("Checking for expiring OAuth tokens...");

                // Get tokens expiring in next 60 minutes
                let expiring = oauth.get_expiring_tokens(60).await;

                for provider in expiring {
                    tracing::info!("Refreshing token for provider: {}", provider);

                    match oauth.refresh_token(&provider).await {
                        Ok(_) => {
                            tracing::info!("Token refreshed for: {}", provider);
                        }
                        Err(e) => {
                            tracing::error!("Failed to refresh token for {}: {}", provider, e);
                        }
                    }
                }
            })
        }).map_err(|e| Error::Other(format!("Failed to create token refresh job: {e}")))?;

        let scheduler = self.scheduler.write().await;
        scheduler.add(job).await
            .map_err(|e| Error::Other(format!("Failed to add token refresh job: {e}")))?;

        Ok(())
    }

    /// Add daily cleanup job
    async fn add_cleanup_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 0 2 * * *", move |_uuid, _lock| {
            let db = db.clone();

            Box::pin(async move {
                tracing::info!("Running daily cleanup tasks...");

                // Clean up old pipeline activities (older than 30 days)
                let query = "
                    DELETE FROM pipeline_activities
                    WHERE created_at < NOW() - INTERVAL '30 days'
                ";

                match db.execute(query, &[]).await {
                    Ok(_) => tracing::info!("Cleaned up old pipeline activities"),
                    Err(e) => tracing::error!("Cleanup failed: {}", e),
                }
            })
        }).map_err(|e| Error::Other(format!("Failed to create cleanup job: {e}")))?;

        let scheduler = self.scheduler.write().await;
        scheduler.add(job).await
            .map_err(|e| Error::Other(format!("Failed to add cleanup job: {e}")))?;

        Ok(())
    }

    /// Save schedule configuration to database
    async fn save_schedule(&self, config: &ScheduleConfig) -> Result<()> {
        let query = "
            INSERT INTO sync_schedules (source_id, source_type, stream_name, cron_expression, enabled, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (source_id) DO UPDATE
            SET cron_expression = $4, enabled = $5, updated_at = NOW()
        ";

        self.db.execute(
            query,
            &[
                &config.source_id.to_string(),
                &config.source_type,
                &config.stream_name,
                &config.cron_expression,
                &config.enabled.to_string(),
            ]
        ).await?;

        Ok(())
    }

    /// Load schedules from database
    async fn load_schedules(&self) -> Result<Vec<ScheduleConfig>> {
        let query = "
            SELECT source_id, source_type, stream_name, cron_expression, enabled, last_run
            FROM sync_schedules
            WHERE enabled = true
        ";

        let results = self.db.query(query).await?;

        let schedules = results.into_iter()
            .filter_map(|row| {
                let source_id = row.get("source_id")?.as_str()?.parse::<Uuid>().ok()?;
                let source_type = row.get("source_type")?.as_str()?.to_string();
                let stream_name = row.get("stream_name")?.as_str()?.to_string();
                let cron = row.get("cron_expression")?.as_str()?.to_string();
                let enabled = row.get("enabled")?.as_bool()?;

                Some(ScheduleConfig {
                    source_id,
                    source_type,
                    stream_name,
                    cron_expression: cron,
                    enabled,
                    last_run: None,
                    next_run: None,
                })
            })
            .collect();

        Ok(schedules)
    }

    /// Manually trigger a sync for a source
    pub async fn trigger_sync(&self, source_id: Uuid) -> Result<SyncResult> {
        let sources = self.sources.read().await;

        let source = sources.iter()
            .find(|s| s.source_id() == source_id)
            .ok_or_else(|| Error::Other(format!("No source found: {source_id}")))?;

        // Use incremental mode by default
        let mode = SyncMode::Incremental { cursor: None };
        source.sync(&mode).await
    }

    /// List all registered sources
    pub async fn list_sources(&self) -> Vec<(Uuid, String, String)> {
        let sources = self.sources.read().await;
        sources.iter()
            .map(|s| (s.source_id(), s.source_type().to_string(), s.stream_name().to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_config() {
        let source_id = Uuid::new_v4();

        let config = ScheduleConfig {
            source_id,
            source_type: "google".to_string(),
            stream_name: "calendar".to_string(),
            cron_expression: "0 */5 * * * *".to_string(),
            enabled: true,
            last_run: None,
            next_run: None,
        };

        assert_eq!(config.source_type, "google");
        assert_eq!(config.stream_name, "calendar");
        assert!(config.enabled);
    }
}