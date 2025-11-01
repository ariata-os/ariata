//! Job executor for running async jobs in background tasks

use crate::error::Result;
use crate::jobs::models::{JobStatus, JobType};
use crate::jobs::sync_job::execute_sync_job;
use crate::jobs::transform_job::execute_transform_job;
use sqlx::PgPool;
use uuid::Uuid;

/// Job executor that spawns background tasks for async job execution
#[derive(Clone)]
pub struct JobExecutor {
    db: PgPool,
}

impl JobExecutor {
    /// Create a new job executor
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Execute a job asynchronously in a background task
    ///
    /// This spawns a Tokio task to run the job and returns immediately.
    /// The job status will be updated in the database as it progresses.
    pub fn execute_async(&self, job_id: Uuid) {
        let db = self.db.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_job(&db, job_id).await {
                tracing::error!(
                    job_id = %job_id,
                    error = %e,
                    "Job execution failed"
                );
            }
        });
    }

    /// Internal method to run a job
    async fn run_job(db: &PgPool, job_id: Uuid) -> Result<()> {
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

        // Create executor for job chaining
        let executor = JobExecutor::new(db.clone());

        // Execute the job based on type
        let result = match job.job_type {
            JobType::Sync => execute_sync_job(db, &executor, &job).await,
            JobType::Transform => execute_transform_job(db, &job).await,
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
