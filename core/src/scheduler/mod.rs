//! Built-in task scheduler for periodic syncs

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    database::Database,
    oauth::OAuthManager,
};

/// Schedule configuration for a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub source_name: String,
    pub cron_expression: String,  // e.g., "0 */5 * * * *" for every 5 minutes
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
}

/// Sync task that can be scheduled
#[async_trait::async_trait]
pub trait SyncTask: Send + Sync {
    /// Name of the source
    fn source_name(&self) -> &str;

    /// Execute the sync
    async fn sync(&self) -> Result<SyncResult>;

    /// Check if sync is needed
    async fn should_sync(&self) -> bool {
        true
    }
}

/// Result of a sync operation
#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub source: String,
    pub records_synced: usize,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Main scheduler for managing periodic syncs
pub struct Scheduler {
    db: Arc<Database>,
    oauth: Arc<OAuthManager>,
    scheduler: Arc<RwLock<JobScheduler>>,
    tasks: Arc<RwLock<Vec<Box<dyn SyncTask>>>>,
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
            tasks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register a sync task
    pub async fn register_task(&self, task: Box<dyn SyncTask>) {
        let mut tasks = self.tasks.write().await;
        tasks.push(task);
    }

    /// Add a scheduled sync for a source
    pub async fn add_schedule(&self, config: ScheduleConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        let source_name = config.source_name.clone();
        let tasks = self.tasks.clone();
        let db = self.db.clone();

        // Create a cron job
        let job = Job::new_async(config.cron_expression.as_str(), move |_uuid, _lock| {
            let source_name = source_name.clone();
            let tasks = tasks.clone();
            let db = db.clone();

            Box::pin(async move {
                let start = std::time::Instant::now();
                let started_at = Utc::now();

                // Find the task for this source
                let tasks_guard = tasks.read().await;
                let task = tasks_guard.iter()
                    .find(|t| t.source_name() == source_name);

                if let Some(task) = task {
                    if task.should_sync().await {
                        match task.sync().await {
                            Ok(result) => {
                                let duration_ms = start.elapsed().as_millis() as i32;

                                tracing::info!(
                                    "Sync completed for {}: {} records in {}ms",
                                    source_name, result.records_synced, duration_ms
                                );

                                // Get source_id for logging
                                if let Ok(source_id) = get_source_id_for_logging(&db, &source_name).await {
                                    // Record sync result to sync_logs table
                                    let _ = sqlx::query(
                                        "INSERT INTO sync_logs
                                         (source_id, sync_mode, started_at, completed_at, duration_ms,
                                          status, records_fetched, records_written, records_failed)
                                         VALUES ($1, 'scheduled', $2, NOW(), $3, 'success', $4, $4, 0)"
                                    )
                                    .bind(source_id)
                                    .bind(started_at)
                                    .bind(duration_ms)
                                    .bind(result.records_synced as i32)
                                    .execute(db.pool())
                                    .await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Sync failed for {}: {}", source_name, e);

                                // Get source_id for logging
                                if let Ok(source_id) = get_source_id_for_logging(&db, &source_name).await {
                                    // Record failure to sync_logs table
                                    let _ = sqlx::query(
                                        "INSERT INTO sync_logs
                                         (source_id, sync_mode, started_at, completed_at,
                                          status, error_message, error_class)
                                         VALUES ($1, 'scheduled', $2, NOW(), 'failed', $3, 'sync_error')"
                                    )
                                    .bind(source_id)
                                    .bind(started_at)
                                    .bind(e.to_string())
                                    .execute(db.pool())
                                    .await;
                                }
                            }
                        }
                    }
                } else {
                    tracing::warn!("No task found for scheduled source: {}", source_name);
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

        tracing::info!("Scheduler started with {} tasks", self.tasks.read().await.len());

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
        // Get source_id from source name
        let source_id = self.get_source_id_by_name(&config.source_name).await?;

        sqlx::query(
            "INSERT INTO sync_schedules (source_id, cron_expression, enabled)
             VALUES ($1, $2, $3)
             ON CONFLICT (source_id) DO UPDATE
             SET cron_expression = $2, enabled = $3, updated_at = NOW()"
        )
        .bind(source_id)
        .bind(&config.cron_expression)
        .bind(config.enabled)
        .execute(self.db.pool())
        .await
        .map_err(|e| Error::Database(format!("Failed to save schedule: {e}")))?;

        Ok(())
    }

    /// Load schedules from database
    async fn load_schedules(&self) -> Result<Vec<ScheduleConfig>> {
        let rows = sqlx::query_as::<_, (String, String, bool, Option<DateTime<Utc>>)>(
            "SELECT s.name as source_name, ss.cron_expression, ss.enabled, ss.last_run
             FROM sync_schedules ss
             JOIN sources s ON ss.source_id = s.id
             WHERE ss.enabled = true"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| Error::Database(format!("Failed to load schedules: {e}")))?;

        Ok(rows.into_iter().map(|(name, cron, enabled, last_run)| ScheduleConfig {
            source_name: name,
            cron_expression: cron,
            enabled,
            last_run,
            next_run: None,
        }).collect())
    }

    /// Manually trigger a sync for a source
    pub async fn trigger_sync(&self, source_name: &str) -> Result<SyncResult> {
        let tasks = self.tasks.read().await;

        let task = tasks.iter()
            .find(|t| t.source_name() == source_name)
            .ok_or_else(|| Error::Other(format!("No task for source: {source_name}")))?;

        task.sync().await
    }

    /// List all registered tasks
    pub async fn list_tasks(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks.iter().map(|t| t.source_name().to_string()).collect()
    }

    /// Helper: Get source_id from source name
    async fn get_source_id_by_name(&self, name: &str) -> Result<Uuid> {
        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM sources WHERE name = $1"
        )
        .bind(name)
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| Error::Database(format!("Source '{name}' not found: {e}")))?;

        Ok(row.0)
    }
}

/// Helper function to get source_id from name (for use in closures)
async fn get_source_id_for_logging(db: &Database, name: &str) -> Result<Uuid> {
    let row = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM sources WHERE name = $1"
    )
    .bind(name)
    .fetch_one(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Source '{name}' not found: {e}")))?;

    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_config() {
        let config = ScheduleConfig {
            source_name: "google_calendar".to_string(),
            cron_expression: "0 */5 * * * *".to_string(),
            enabled: true,
            last_run: None,
            next_run: None,
        };

        assert_eq!(config.source_name, "google_calendar");
        assert!(config.enabled);
    }
}