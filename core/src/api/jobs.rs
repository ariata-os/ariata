//! Jobs API for async job tracking and management

use crate::error::{Error, Result};
use crate::jobs::{
    self, ApiKeys, CreateJobRequest, Job, JobExecutor, JobStatus, SyncJobMetadata, TransformContext,
};
use crate::storage::{stream_writer::StreamWriter, Storage};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Response when a job is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobResponse {
    pub job_id: Uuid,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Trigger a sync for a specific stream (async version)
///
/// This creates a job and starts it in the background, returning immediately.
pub async fn trigger_stream_sync(
    db: &PgPool,
    storage: &Storage,
    stream_writer: Arc<Mutex<StreamWriter>>,
    source_id: Uuid,
    stream_name: &str,
    sync_mode: Option<crate::sources::base::SyncMode>,
) -> Result<CreateJobResponse> {
    // Check if there's already an active sync for this stream
    if jobs::has_active_sync_job(db, source_id, stream_name).await? {
        return Err(Error::InvalidInput(format!(
            "Stream '{}' already has an active sync job",
            stream_name
        )));
    }

    // Convert sync mode to string for storage
    let sync_mode_str = match sync_mode {
        Some(crate::sources::base::SyncMode::FullRefresh) => "full_refresh",
        Some(crate::sources::base::SyncMode::Incremental { .. }) | None => "incremental",
    }
    .to_string();

    // Load cursor from database for incremental syncs
    let cursor_before = if sync_mode_str == "incremental" {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT last_sync_token FROM elt.streams WHERE source_id = $1 AND stream_name = $2",
        )
        .bind(source_id)
        .bind(stream_name)
        .fetch_optional(db)
        .await?
        .flatten()
    } else {
        None
    };

    // Create job request
    let request = CreateJobRequest::new_sync_job(
        source_id,
        stream_name.to_string(),
        sync_mode_str.clone(),
        SyncJobMetadata {
            sync_mode: sync_mode_str,
            cursor_before,
        },
    );

    // Create job in database
    let job = jobs::create_job(db, request).await?;

    // Create context without data source
    // The sync_job executor will create its own context with the actual records after sync completes.
    let api_keys = ApiKeys::from_env();
    let context = TransformContext::new(Arc::new(storage.clone()), stream_writer, api_keys);

    // Start job execution in background
    let executor = JobExecutor::new(db.clone(), context);
    executor.execute_async(job.id);

    Ok(CreateJobResponse {
        job_id: job.id,
        status: job.status.to_string(),
        started_at: job.started_at,
    })
}

/// Get job status by ID
pub async fn get_job_status(db: &PgPool, job_id: Uuid) -> Result<Job> {
    jobs::get_job(db, job_id).await
}

/// Query jobs with filters
#[derive(Debug, Clone, Deserialize)]
pub struct QueryJobsRequest {
    pub source_id: Option<Uuid>,
    pub status: Option<Vec<String>>,
    pub limit: Option<i64>,
}

pub async fn query_jobs(db: &PgPool, request: QueryJobsRequest) -> Result<Vec<Job>> {
    // Parse status strings to JobStatus enums
    let statuses = if let Some(status_strs) = request.status {
        let parsed: Result<Vec<JobStatus>> = status_strs
            .iter()
            .map(|s| {
                s.parse::<JobStatus>()
                    .map_err(|e| Error::InvalidInput(e.to_string()))
            })
            .collect();
        Some(parsed?)
    } else {
        None
    };

    jobs::query_jobs(db, request.source_id, statuses, request.limit).await
}

/// Cancel a running job
pub async fn cancel_job(db: &PgPool, job_id: Uuid) -> Result<()> {
    jobs::cancel_job(db, job_id).await
}

/// Get job history for a specific stream
///
/// Returns jobs for a specific source and stream, ordered by most recent first.
pub async fn get_job_history(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
    limit: i64,
) -> Result<Vec<Job>> {
    let jobs = sqlx::query_as::<_, Job>(
        r#"
        SELECT *
        FROM elt.jobs
        WHERE source_id = $1 AND stream_name = $2 AND job_type = 'sync'
        ORDER BY created_at DESC
        LIMIT $3
        "#,
    )
    .bind(source_id)
    .bind(stream_name)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|e| {
        Error::Database(format!(
            "Failed to get job history for stream {}: {}",
            stream_name, e
        ))
    })?;

    Ok(jobs)
}
