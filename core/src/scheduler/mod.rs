//! Built-in task scheduler for periodic stream syncs
//!
//! This scheduler reads enabled streams from the `streams` table and
//! schedules them based on their `cron_schedule` field.
//!
//! ## Cron Expression Format
//!
//! The scheduler uses tokio-cron-scheduler which requires 6-field cron expressions:
//! ```text
//! sec   min   hour   day   month   day_of_week
//! *     *     *      *     *       *
//! ```
//!
//! ### Examples:
//! - `0 0 */6 * * *` - Every 6 hours
//! - `0 */15 * * * *` - Every 15 minutes
//! - `0 0 0 * * *` - Daily at midnight
//! - `0 0 9 * * 1` - Every Monday at 9:00 AM

use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    jobs::PrudentContextJob,
    llm::LLMClient,
    storage::{stream_writer::StreamWriter, Storage},
};

/// Simplified scheduler using StreamFactory
pub struct Scheduler {
    db: PgPool,
    storage: Storage,
    stream_writer: Arc<Mutex<StreamWriter>>,
    scheduler: JobScheduler,
}

impl Scheduler {
    /// Create a new scheduler
    pub async fn new(
        db: PgPool,
        storage: Storage,
        stream_writer: Arc<Mutex<StreamWriter>>,
    ) -> Result<Self> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| Error::Other(format!("Failed to create scheduler: {e}")))?;

        Ok(Self {
            db,
            storage,
            stream_writer,
            scheduler,
        })
    }

    /// Start the scheduler
    ///
    /// Loads all enabled streams with cron schedules from the database
    /// and creates jobs for each.
    ///
    /// Note: Only pull streams (Google, Notion) are scheduled. Push streams
    /// (Mac, iOS) are not scheduled since they're initiated by client devices.
    pub async fn start(&self) -> Result<()> {
        // First, check for enabled streams WITHOUT cron schedules and log warnings
        let streams_without_schedule = sqlx::query_as::<_, (Uuid, String, String, String)>(
            r#"
            SELECT
                st.id as stream_connection_id,
                s.source,
                s.name as source_name,
                st.stream_name
            FROM stream_connections st
            JOIN source_connections s ON st.source_connection_id = s.id
            WHERE st.is_enabled = true
              AND st.cron_schedule IS NULL
              AND s.is_active = true
              AND s.source NOT IN ('mac', 'ios')  -- Exclude push-only sources
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        // Check if any enabled streams are missing schedules
        for (stream_id, source, source_name, stream_name) in &streams_without_schedule {
            // Look up registry default
            if let Some(registered_stream) = crate::registry::get_stream(source, stream_name) {
                if let Some(default_cron) = registered_stream.default_cron_schedule {
                    tracing::warn!(
                        "Stream {}/{} ({}) is enabled but has no cron_schedule. \
                        Registry default is '{}'. Consider updating the database or seed config.",
                        source, stream_name, source_name, default_cron
                    );

                    // Apply the default schedule to the database
                    tracing::info!(
                        "Applying registry default cron schedule '{}' to {}/{}",
                        default_cron, source, stream_name
                    );

                    sqlx::query!(
                        r#"
                        UPDATE data.stream_connections
                        SET cron_schedule = $1, updated_at = NOW()
                        WHERE id = $2
                        "#,
                        default_cron,
                        stream_id
                    )
                    .execute(&self.db)
                    .await?;
                } else {
                    tracing::debug!(
                        "Stream {}/{} ({}) is enabled but has no cron_schedule and no registry default. \
                        This stream may need manual scheduling or is not meant to be scheduled.",
                        source, stream_name, source_name
                    );
                }
            } else {
                tracing::warn!(
                    "Stream {}/{} ({}) not found in registry. Cannot apply default schedule.",
                    source, stream_name, source_name
                );
            }
        }

        // Load enabled streams from database
        // Filter to only pull streams (not 'mac' or 'ios' which are push-only)
        let streams = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>)>(
            r#"
            SELECT
                s.id as source_id,
                s.name as source_name,
                s.source,
                st.stream_name,
                st.cron_schedule
            FROM stream_connections st
            JOIN source_connections s ON st.source_connection_id = s.id
            WHERE st.is_enabled = true
              AND st.cron_schedule IS NOT NULL
              AND s.is_active = true
              AND s.source NOT IN ('mac', 'ios')  -- Exclude push-only sources
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        tracing::info!("Loading {} scheduled streams", streams.len());

        // Schedule each stream
        for (source_id, source_name, provider, stream_name, cron_schedule) in streams {
            let cron = cron_schedule.expect("cron_schedule is NOT NULL per WHERE clause");

            let db = self.db.clone();
            let storage = self.storage.clone();
            let stream_writer = self.stream_writer.clone();

            tracing::debug!(
                "Scheduling {}/{} ({}) with cron: {}",
                provider,
                stream_name,
                source_name,
                cron
            );

            // Clone values for error message before they're moved into closure
            let provider_for_error = provider.clone();
            let stream_name_for_error = stream_name.clone();
            let source_name_for_error = source_name.clone();

            let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
                let db = db.clone();
                let storage = storage.clone();
                let stream_writer = stream_writer.clone();
                let source_id = source_id;
                let stream_name = stream_name.clone();
                let source_name = source_name.clone();
                let stream_name_str = stream_name.clone();

                Box::pin(async move {
                    tracing::info!(
                        "Running scheduled sync: {} ({})",
                        stream_name_str,
                        source_name
                    );

                    // Use the job-based API
                    match crate::api::jobs::trigger_stream_sync(
                        &db,
                        &storage,
                        stream_writer,
                        source_id,
                        &stream_name,
                        None,
                    )
                    .await
                    {
                        Ok(response) => {
                            tracing::info!(
                                "Scheduled sync job created: {} - job_id={}, status={}",
                                stream_name_str,
                                response.job_id,
                                response.status
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to create scheduled sync job for {}: {}",
                                stream_name_str,
                                e
                            );
                        }
                    }
                })
            })
            .map_err(|e| {
                Error::Other(format!(
                    "Failed to create job for {}/{} ({}): {}. \
                    Note: Cron expressions must be in 6-field format (sec min hour day month dow). \
                    Example: '0 0 */6 * * *' for every 6 hours.",
                    provider_for_error, stream_name_for_error, source_name_for_error, e
                ))
            })?;

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

    /// Schedule the prudent context job (4x daily: 6am, 12pm, 6pm, 10pm)
    pub async fn schedule_prudent_context_job(&self, llm_client: Arc<dyn LLMClient>) -> Result<()> {
        let schedules = vec![
            ("0 0 6 * * *", "6am"),
            ("0 0 12 * * *", "12pm"),
            ("0 0 18 * * *", "6pm"),
            ("0 0 22 * * *", "10pm"),
        ];

        for (cron_expr, label) in schedules {
            let db = self.db.clone();
            let llm_client = llm_client.clone();

            tracing::info!("Scheduling PrudentContextJob at {}", label);

            let job = Job::new_async(cron_expr, move |_uuid, _lock| {
                let db_pool = Arc::new(db.clone());
                let llm = llm_client.clone();

                Box::pin(async move {
                    tracing::info!("Running PrudentContextJob");
                    let job = PrudentContextJob::new(db_pool, llm);

                    match job.execute().await {
                        Ok(()) => {
                            tracing::info!("PrudentContextJob completed successfully");
                        }
                        Err(e) => {
                            tracing::error!("PrudentContextJob failed: {}", e);
                        }
                    }
                })
            })
            .map_err(|e| {
                Error::Other(format!(
                    "Failed to create PrudentContextJob for {}: {}",
                    label, e
                ))
            })?;

            self.scheduler
                .add(job)
                .await
                .map_err(|e| Error::Other(format!("Failed to add PrudentContextJob: {}", e)))?;
        }

        tracing::info!("PrudentContextJob scheduled for 6am, 12pm, 6pm, and 10pm");
        Ok(())
    }

    /// Schedule the narrative primitive pipeline job (hourly)
    ///
    /// Unified pipeline that runs:
    /// 1. Entity resolution (place clustering, people resolution)
    /// 2. Changepoint detection
    /// 3. Boundary aggregation
    /// 4. Narrative synthesis
    ///
    /// Replaces BoundarySweeperJob and PeriodicClusteringJob
    pub async fn schedule_narrative_primitive_pipeline_job(&self) -> Result<()> {
        use crate::database::Database;

        let db = self.db.clone();

        tracing::info!("Scheduling NarrativePrimitivePipelineJob (hourly)");

        let job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
            let db_pool = db.clone();

            Box::pin(async move {
                tracing::info!("Running NarrativePrimitivePipelineJob");
                let db = Database::from_pool(db_pool);

                match crate::jobs::narrative_primitive_pipeline::run_pipeline(&db).await {
                    Ok(stats) => {
                        tracing::info!(
                            "NarrativePrimitivePipelineJob completed: \
                            places={}, people={}, boundaries={}, primitives={}",
                            stats.places_resolved,
                            stats.people_resolved,
                            stats.boundaries_detected,
                            stats.primitives_created
                        );
                    }
                    Err(e) => {
                        tracing::error!("NarrativePrimitivePipelineJob failed: {}", e);
                    }
                }
            })
        })
        .map_err(|e| {
            Error::Other(format!(
                "Failed to create NarrativePrimitivePipelineJob: {}",
                e
            ))
        })?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| Error::Other(format!("Failed to add NarrativePrimitivePipelineJob: {}", e)))?;

        tracing::info!("NarrativePrimitivePipelineJob scheduled (runs hourly at :00)");
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
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            r#"
            SELECT
                s.id as source_id,
                s.name as source_name,
                st.stream_name,
                st.cron_schedule,
                st.last_sync_at
            FROM stream_connections st
            JOIN source_connections s ON st.source_connection_id = s.id
            WHERE st.is_enabled = true
              AND st.cron_schedule IS NOT NULL
              AND s.source NOT IN ('mac', 'ios')  -- Exclude push-only sources
            ORDER BY s.name, st.stream_name
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let streams = rows
            .into_iter()
            .map(
                |(source_id, source_name, stream_name, cron_schedule, last_sync_at)| {
                    ScheduledStream {
                        source_id,
                        source_name,
                        stream_name,
                        cron_schedule,
                        last_sync_at,
                    }
                },
            )
            .collect();

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
        let storage = Storage::local("./test_data".to_string()).unwrap();
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let result = Scheduler::new(pool, storage, stream_writer).await;
        assert!(result.is_ok());
    }
}
