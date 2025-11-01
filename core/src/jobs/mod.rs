//! Job management system for async operations
//!
//! Provides a unified job system for tracking sync, transform, and other async operations.
//! Jobs are tracked in the database and can be polled for status updates.

pub mod executor;
pub mod models;
pub mod sync_job;
pub mod transform_job;

pub use executor::JobExecutor;
pub use models::{CreateJobRequest, Job, JobStatus, JobType, SyncJobMetadata};

use crate::error::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper function to convert a row to a Job
fn job_from_row(row: &sqlx::postgres::PgRow) -> Result<Job> {
    use sqlx::Row;

    Ok(Job {
        id: row.try_get("id")?,
        job_type: row.try_get::<String, _>("job_type")?.parse().map_err(|e: String| Error::Other(e))?,
        status: row.try_get::<String, _>("status")?.parse().map_err(|e: String| Error::Other(e))?,
        source_id: row.try_get("source_id")?,
        stream_name: row.try_get("stream_name")?,
        sync_mode: row.try_get("sync_mode")?,
        transform_id: row.try_get("transform_id")?,
        transform_strategy: row.try_get("transform_strategy")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        records_processed: row.try_get("records_processed")?,
        error_message: row.try_get("error_message")?,
        error_class: row.try_get("error_class")?,
        metadata: row.try_get("metadata")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Check if a stream has an active (pending or running) sync job
pub async fn has_active_sync_job(
    db: &PgPool,
    source_id: Uuid,
    stream_name: &str,
) -> Result<bool> {
    let result = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM elt.jobs
            WHERE source_id = $1
              AND stream_name = $2
              AND job_type = 'sync'
              AND status IN ('pending', 'running')
        )
        "#,
    )
    .bind(source_id)
    .bind(stream_name)
    .fetch_one(db)
    .await?;

    Ok(result)
}

/// Create a new job in the database
pub async fn create_job(db: &PgPool, request: CreateJobRequest) -> Result<Job> {
    let row = sqlx::query(
        r#"
        INSERT INTO elt.jobs (
            job_type,
            status,
            source_id,
            stream_name,
            sync_mode,
            transform_id,
            transform_strategy,
            metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(&request.job_type.to_string())
    .bind(&request.status.to_string())
    .bind(request.source_id)
    .bind(&request.stream_name)
    .bind(&request.sync_mode)
    .bind(request.transform_id)
    .bind(&request.transform_strategy)
    .bind(&request.metadata)
    .fetch_one(db)
    .await?;

    Ok(job_from_row(&row)?)
}

/// Get a job by ID
pub async fn get_job(db: &PgPool, job_id: Uuid) -> Result<Job> {
    let row = sqlx::query(
        r#"
        SELECT * FROM elt.jobs
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Job not found: {}", job_id)))?;

    job_from_row(&row)
}

/// Update job status
pub async fn update_job_status(
    db: &PgPool,
    job_id: Uuid,
    status: JobStatus,
    error_message: Option<String>,
) -> Result<()> {
    // Terminal statuses should set completed_at
    let is_terminal = matches!(
        status,
        JobStatus::Succeeded | JobStatus::Failed | JobStatus::Cancelled
    );

    let query = if is_terminal {
        sqlx::query(
            r#"
            UPDATE elt.jobs
            SET status = $1,
                error_message = $2,
                completed_at = NOW()
            WHERE id = $3
            "#,
        )
    } else {
        sqlx::query(
            r#"
            UPDATE elt.jobs
            SET status = $1,
                error_message = $2
            WHERE id = $3
            "#,
        )
    };

    query
        .bind(status.to_string())
        .bind(error_message)
        .bind(job_id)
        .execute(db)
        .await?;

    Ok(())
}

// Note: update_job_progress() was removed - not needed since we update progress
// in execute_sync_job() directly when the job completes

/// Query jobs with filters
pub async fn query_jobs(
    db: &PgPool,
    source_id: Option<Uuid>,
    statuses: Option<Vec<JobStatus>>,
    limit: Option<i64>,
) -> Result<Vec<Job>> {
    let mut query = String::from("SELECT * FROM elt.jobs WHERE 1=1");
    let mut bind_count = 0;

    if source_id.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND source_id = ${}", bind_count));
    }

    // Convert statuses to strings outside the binding scope
    let status_strings: Option<Vec<String>> = statuses.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.iter().map(|status| status.to_string()).collect())
        }
    });

    if status_strings.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND status = ANY(${})", bind_count));
    }

    query.push_str(" ORDER BY created_at DESC");

    if limit.is_some() {
        bind_count += 1;
        query.push_str(&format!(" LIMIT ${}", bind_count));
    }

    let mut q = sqlx::query(&query);

    if let Some(sid) = source_id {
        q = q.bind(sid);
    }

    if let Some(ref status_strs) = status_strings {
        q = q.bind(status_strs.as_slice());
    }

    if let Some(lim) = limit {
        q = q.bind(lim);
    }

    let rows = q.fetch_all(db).await?;

    let mut jobs = Vec::new();
    for row in rows {
        jobs.push(job_from_row(&row)?);
    }

    Ok(jobs)
}

/// Cancel a running job
pub async fn cancel_job(db: &PgPool, job_id: Uuid) -> Result<()> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE elt.jobs
        SET status = 'cancelled',
            completed_at = NOW()
        WHERE id = $1
          AND status IN ('pending', 'running')
        "#,
    )
    .bind(job_id)
    .execute(db)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(Error::InvalidInput(
            "Job cannot be cancelled (not found or already completed)".to_string(),
        ));
    }

    Ok(())
}
