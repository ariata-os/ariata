//! Built-in task scheduler for periodic stream syncs
//!
//! This scheduler reads enabled streams from the `streams` table and
//! schedules them based on their `cron_schedule` field.

use tokio_cron_scheduler::{Job, JobScheduler};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    sources::StreamFactory,
};

/// Simplified scheduler using StreamFactory
pub struct Scheduler {
    db: PgPool,
    factory: StreamFactory,
    scheduler: JobScheduler,
}

impl Scheduler {
    /// Create a new scheduler
    pub async fn new(db: PgPool) -> Result<Self> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| Error::Other(format!("Failed to create scheduler: {e}")))?;

        Ok(Self {
            factory: StreamFactory::new(db.clone()),
            scheduler,
            db,
        })
    }

    /// Start the scheduler
    ///
    /// Loads all enabled streams with cron schedules from the database
    /// and creates jobs for each.
    pub async fn start(&self) -> Result<()> {
        // Load enabled streams from database
        let streams = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>)>(
            r#"
            SELECT
                s.id as source_id,
                s.name as source_name,
                s.provider,
                st.stream_name,
                st.cron_schedule
            FROM streams st
            JOIN sources s ON st.source_id = s.id
            WHERE st.is_enabled = true
              AND st.cron_schedule IS NOT NULL
              AND s.is_active = true
            "#
        )
        .fetch_all(&self.db)
        .await?;

        tracing::info!("Loading {} scheduled streams", streams.len());

        // Schedule each stream
        for (source_id, source_name, provider, stream_name, cron_schedule) in streams {
            let cron = cron_schedule.expect("cron_schedule is NOT NULL per WHERE clause");

            let db = self.db.clone();

            tracing::info!(
                "Scheduling {}/{} ({}) with cron: {}",
                provider,
                stream_name,
                source_name,
                cron
            );

            let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
                let db = db.clone();
                let source_id = source_id;
                let stream_name = stream_name.clone();
                let source_name = source_name.clone();
                let stream_name_str = stream_name.clone();

                Box::pin(async move {
                    tracing::info!("Running scheduled sync: {} ({})", stream_name_str, source_name);

                    // Use the job-based API
                    match crate::api::jobs::trigger_stream_sync(&db, source_id, &stream_name, None).await {
                        Ok(response) => {
                            tracing::info!(
                                "Scheduled sync job created: {} - job_id={}, status={}",
                                stream_name_str,
                                response.job_id,
                                response.status
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to create scheduled sync job for {}: {}", stream_name_str, e);
                        }
                    }
                })
            })
            .map_err(|e| Error::Other(format!("Failed to create job: {e}")))?;

            self.scheduler
                .add(job)
                .await
                .map_err(|e| Error::Other(format!("Failed to add job: {e}")))?;
        }

        // Start the scheduler
        self.scheduler
            .start()
            .await
            .map_err(|e| Error::Other(format!("Failed to start scheduler: {e}")))?;

        tracing::info!("Scheduler started successfully");
        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&mut self) -> Result<()> {
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| Error::Other(format!("Failed to stop scheduler: {e}")))?;

        tracing::info!("Scheduler stopped");
        Ok(())
    }

    /// Get list of scheduled streams
    pub async fn list_scheduled(&self) -> Result<Vec<ScheduledStream>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, Option<chrono::DateTime<chrono::Utc>>)>(
            r#"
            SELECT
                s.id as source_id,
                s.name as source_name,
                st.stream_name,
                st.cron_schedule,
                st.last_sync_at
            FROM streams st
            JOIN sources s ON st.source_id = s.id
            WHERE st.is_enabled = true
              AND st.cron_schedule IS NOT NULL
            ORDER BY s.name, st.stream_name
            "#
        )
        .fetch_all(&self.db)
        .await?;

        let streams = rows.into_iter().map(|(source_id, source_name, stream_name, cron_schedule, last_sync_at)| {
            ScheduledStream {
                source_id,
                source_name,
                stream_name,
                cron_schedule,
                last_sync_at,
            }
        }).collect();

        Ok(streams)
    }
}

/// Information about a scheduled stream
#[derive(Debug)]
pub struct ScheduledStream {
    pub source_id: Uuid,
    pub source_name: String,
    pub stream_name: String,
    pub cron_schedule: String,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let result = Scheduler::new(pool).await;
        assert!(result.is_ok());
    }
}
