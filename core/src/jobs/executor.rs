//! Job executor for running async jobs in background tasks

use std::sync::Arc;
use crate::error::Result;
use crate::jobs::models::{JobStatus, JobType};
use crate::jobs::sync_job::execute_sync_job;
use crate::jobs::transform_context::TransformContext;
use crate::jobs::transform_job::execute_transform_job;
use sqlx::PgPool;
use uuid::Uuid;

/// Job executor that spawns background tasks for async job execution
#[derive(Clone)]
pub struct JobExecutor {
    db: PgPool,
    context: Arc<TransformContext>,
}

impl JobExecutor {
    /// Create a new job executor
    ///
    /// # Arguments
    /// * `db` - Database connection pool
    /// * `context` - Transform context with storage and API keys
    pub fn new(db: PgPool, context: TransformContext) -> Self {
        Self {
            db,
            context: Arc::new(context),
        }
    }

    /// Execute a job asynchronously in a background task
    ///
    /// This spawns a Tokio task to run the job and returns immediately.
    /// The job status will be updated in the database as it progresses.
    pub fn execute_async(&self, job_id: Uuid) {
        let db = self.db.clone();
        let context = self.context.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_job(&db, &context, job_id).await {
                tracing::error!(
                    job_id = %job_id,
                    error = %e,
                    "Job execution failed"
                );
            }
        });
    }

    /// Internal method to run a job
    async fn run_job(db: &PgPool, context: &Arc<TransformContext>, job_id: Uuid) -> Result<()> {
        // Fetch the job
        let job = super::get_job(db, job_id).await?;

        // Check if job is in pending state
        if job.status != JobStatus::Pending {
            tracing::warn!(
                job_id = %job_id,
                status = %job.status,
                "Job is not in pending state, skipping execution"
            );
            return Ok(());
        }

        // Update job status to running
        super::update_job_status(db, job_id, JobStatus::Running, None).await?;

        tracing::info!(
            job_id = %job_id,
            job_type = %job.job_type,
            "Starting job execution"
        );

        // Create executor for job chaining (with same context)
        let executor = JobExecutor {
            db: db.clone(),
            context: context.clone(),
        };

        // Execute the job based on type
        let result = match job.job_type {
            JobType::Sync => execute_sync_job(db, &executor, context, &job).await,
            JobType::Transform => execute_transform_job(db, context, &job).await,
        };

        // Log result
        match &result {
            Ok(_) => {
                tracing::info!(
                    job_id = %job_id,
                    job_type = %job.job_type,
                    "Job completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    job_id = %job_id,
                    job_type = %job.job_type,
                    error = %e,
                    "Job failed"
                );
            }
        }

        result
    }
}
