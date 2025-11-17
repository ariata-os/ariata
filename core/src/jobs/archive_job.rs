//! Archive job execution for async S3 archival
//!
//! Handles background upload of stream records to S3 storage after sync completes.
//! This enables direct transforms (hot path) while archival happens asynchronously.

use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Row};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::Result;
use crate::storage::Storage;
use crate::storage::{encryption, EncryptionKey};

/// Archive job model from database
#[derive(Debug, sqlx::FromRow)]
pub struct ArchiveJob {
    pub id: Uuid,
    pub sync_job_id: Option<Uuid>,
    pub source_id: Uuid,
    pub stream_name: String,
    pub s3_key: String,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub record_count: i32,
    pub size_bytes: i64,
    pub min_timestamp: Option<chrono::DateTime<Utc>>,
    pub max_timestamp: Option<chrono::DateTime<Utc>>,
}

/// Archive job execution context
pub struct ArchiveContext {
    pub storage: Storage,
    pub master_key: Vec<u8>,
}

/// Execute an archive job to upload records to S3
///
/// # Arguments
///
/// * `db` - Database connection pool
/// * `context` - Archive context with storage and encryption key
/// * `archive_job_id` - UUID of the archive job to execute
/// * `records` - In-memory records to archive
///
/// # Returns
///
/// Result indicating success or failure
pub async fn execute_archive_job(
    db: &PgPool,
    context: &ArchiveContext,
    archive_job_id: Uuid,
    records: Vec<Value>,
) -> Result<()> {
    info!(
        archive_job_id = %archive_job_id,
        record_count = records.len(),
        "Starting archive job execution"
    );

    // Fetch archive job from database
    let archive_job = fetch_archive_job(db, archive_job_id).await?;

    // Update job status to in_progress
    mark_job_in_progress(db, archive_job_id).await?;

    // Execute the archival
    let result = execute_archival(db, context, &archive_job, records).await;

    match result {
        Ok(()) => {
            // Mark job as completed
            mark_job_completed(db, archive_job_id).await?;

            info!(
                archive_job_id = %archive_job_id,
                s3_key = %archive_job.s3_key,
                "Archive job completed successfully"
            );

            Ok(())
        }
        Err(e) => {
            error!(
                archive_job_id = %archive_job_id,
                error = %e,
                retry_count = archive_job.retry_count,
                "Archive job failed"
            );

            // Check if we should retry
            if archive_job.retry_count < archive_job.max_retries {
                mark_job_failed_with_retry(db, archive_job_id, &e.to_string()).await?;
                warn!(
                    archive_job_id = %archive_job_id,
                    retry_count = archive_job.retry_count + 1,
                    "Archive job will be retried"
                );
            } else {
                mark_job_failed_permanent(db, archive_job_id, &e.to_string()).await?;
                error!(
                    archive_job_id = %archive_job_id,
                    "Archive job permanently failed after max retries"
                );
            }

            Err(e)
        }
    }
}

/// Fetch archive job from database
async fn fetch_archive_job(db: &PgPool, archive_job_id: Uuid) -> Result<ArchiveJob> {
    let job = sqlx::query_as::<_, ArchiveJob>(
        "SELECT id, sync_job_id, source_id, stream_name, s3_key, status,
                retry_count, max_retries, record_count, size_bytes,
                min_timestamp, max_timestamp
         FROM data.archive_jobs
         WHERE id = $1",
    )
    .bind(archive_job_id)
    .fetch_one(db)
    .await?;

    Ok(job)
}

/// Execute the actual archival: upload to S3 and update metadata
async fn execute_archival(
    db: &PgPool,
    context: &ArchiveContext,
    archive_job: &ArchiveJob,
    records: Vec<Value>,
) -> Result<()> {
    // Extract date from stream name for key derivation
    let date = archive_job
        .max_timestamp
        .unwrap_or_else(Utc::now)
        .date_naive();

    // Derive encryption key for this stream/date
    let encryption_key_bytes = encryption::derive_stream_key(
        &context.master_key,
        archive_job.source_id,
        &archive_job.stream_name,
        date,
    )?;

    info!(
        archive_job_id = %archive_job.id,
        s3_key = %archive_job.s3_key,
        record_count = records.len(),
        "Uploading records to S3"
    );

    // Upload to S3 with encryption
    let encryption_key = EncryptionKey {
        key_base64: encryption::encode_key_base64(&encryption_key_bytes),
    };

    context
        .storage
        .upload_jsonl_encrypted(&archive_job.s3_key, &records, &encryption_key)
        .await?;

    // Calculate actual size
    let size_bytes = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default().len() as i64)
        .sum::<i64>();

    // Record metadata in stream_objects table
    sqlx::query(
        "INSERT INTO data.stream_objects
         (source_id, stream_name, s3_key, record_count, size_bytes,
          min_timestamp, max_timestamp, archive_job_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())",
    )
    .bind(archive_job.source_id)
    .bind(&archive_job.stream_name)
    .bind(&archive_job.s3_key)
    .bind(records.len() as i32)
    .bind(size_bytes)
    .bind(archive_job.min_timestamp)
    .bind(archive_job.max_timestamp)
    .bind(archive_job.id)
    .execute(db)
    .await?;

    info!(
        archive_job_id = %archive_job.id,
        s3_key = %archive_job.s3_key,
        size_bytes = size_bytes,
        "S3 object metadata recorded"
    );

    Ok(())
}

/// Mark archive job as in progress
async fn mark_job_in_progress(db: &PgPool, archive_job_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE data.archive_jobs
         SET status = 'in_progress',
             started_at = NOW()
         WHERE id = $1",
    )
    .bind(archive_job_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Mark archive job as completed
async fn mark_job_completed(db: &PgPool, archive_job_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE data.archive_jobs
         SET status = 'completed',
             completed_at = NOW()
         WHERE id = $1",
    )
    .bind(archive_job_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Mark archive job as failed with retry
async fn mark_job_failed_with_retry(
    db: &PgPool,
    archive_job_id: Uuid,
    error_message: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE data.archive_jobs
         SET status = 'pending',
             retry_count = retry_count + 1,
             error_message = $2
         WHERE id = $1",
    )
    .bind(archive_job_id)
    .bind(error_message)
    .execute(db)
    .await?;

    Ok(())
}

/// Mark archive job as permanently failed
async fn mark_job_failed_permanent(
    db: &PgPool,
    archive_job_id: Uuid,
    error_message: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE data.archive_jobs
         SET status = 'failed',
             error_message = $2,
             completed_at = NOW()
         WHERE id = $1",
    )
    .bind(archive_job_id)
    .bind(error_message)
    .execute(db)
    .await?;

    Ok(())
}

/// Create a new archive job
///
/// # Arguments
///
/// * `db` - Database connection pool
/// * `sync_job_id` - Parent sync job ID (None for device ingests)
/// * `source_id` - Source UUID
/// * `stream_name` - Stream name
/// * `s3_key` - S3 object key where data will be archived
/// * `records` - Records to archive (for metadata extraction)
///
/// # Returns
///
/// UUID of the created archive job
pub async fn create_archive_job(
    db: &PgPool,
    sync_job_id: Option<Uuid>,
    source_id: Uuid,
    stream_name: &str,
    s3_key: &str,
    records: &[Value],
    min_timestamp: Option<chrono::DateTime<Utc>>,
    max_timestamp: Option<chrono::DateTime<Utc>>,
) -> Result<Uuid> {
    let record_count = records.len() as i32;
    let size_bytes = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default().len() as i64)
        .sum::<i64>();

    let row = sqlx::query(
        "INSERT INTO data.archive_jobs
         (sync_job_id, source_id, stream_name, s3_key, status,
          record_count, size_bytes, min_timestamp, max_timestamp, created_at)
         VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8, NOW())
         RETURNING id",
    )
    .bind(sync_job_id)
    .bind(source_id)
    .bind(stream_name)
    .bind(s3_key)
    .bind(record_count)
    .bind(size_bytes)
    .bind(min_timestamp)
    .bind(max_timestamp)
    .fetch_one(db)
    .await?;

    let archive_job_id: Uuid = row.get("id");

    info!(
        archive_job_id = %archive_job_id,
        sync_job_id = ?sync_job_id,
        stream_name = %stream_name,
        record_count = record_count,
        "Archive job created"
    );

    Ok(archive_job_id)
}

/// Spawn an archive job with async S3 archival (hot path helper)
///
/// This function encapsulates the complete archive job pattern used by both
/// sync jobs and device ingest:
/// 1. Generate S3 key
/// 2. Create archive job in database
/// 3. Spawn async tokio task for S3 upload
///
/// # Arguments
///
/// * `db` - Database connection pool
/// * `storage` - Storage instance for S3 operations
/// * `parent_job_id` - Parent job UUID (None for device ingests, Some(job.id) for sync jobs)
/// * `source_id` - Source UUID
/// * `stream_name` - Stream name (e.g., "healthkit", "gmail")
/// * `records` - In-memory records to archive
/// * `timestamp_range` - (min_timestamp, max_timestamp) tuple
///
/// # Returns
///
/// UUID of the created archive job
///
/// # Example
///
/// ```ignore
/// // Cloud sync job
/// let archive_id = spawn_archive_job_async(
///     db,
///     storage,
///     Some(sync_job.id),
///     source_id,
///     "gmail",
///     records,
///     (Some(min_ts), Some(max_ts)),
/// ).await?;
///
/// // Device ingest (no parent job)
/// let archive_id = spawn_archive_job_async(
///     db,
///     storage,
///     None,
///     source_id,
///     "healthkit",
///     records,
///     (min_ts, max_ts),
/// ).await?;
/// ```
pub async fn spawn_archive_job_async(
    db: &PgPool,
    storage: &Storage,
    parent_job_id: Option<Uuid>,
    source_id: Uuid,
    stream_name: &str,
    records: Vec<Value>,
    timestamp_range: (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>),
) -> Result<Uuid> {
    let (min_timestamp, max_timestamp) = timestamp_range;

    info!(
        parent_job_id = ?parent_job_id,
        source_id = %source_id,
        stream_name,
        record_count = records.len(),
        "Spawning archive job for async S3 archival"
    );

    // Get provider from source
    let provider: String = sqlx::query_scalar("SELECT provider FROM sources WHERE id = $1")
        .bind(source_id)
        .fetch_one(db)
        .await?;

    // Generate S3 key for archival
    let date = Utc::now().date_naive();
    let key_builder = crate::storage::models::StreamKeyBuilder::new(&provider, source_id, stream_name, date);
    let s3_key = key_builder.build();

    // Create archive job in database
    let archive_id = create_archive_job(
        db,
        parent_job_id,
        source_id,
        stream_name,
        &s3_key,
        &records,
        min_timestamp,
        max_timestamp,
    )
    .await?;

    // Get master key from environment
    let master_key_hex = std::env::var("STREAM_ENCRYPTION_MASTER_KEY").map_err(|_| {
        crate::error::Error::Configuration(
            "STREAM_ENCRYPTION_MASTER_KEY required for archival".into(),
        )
    })?;
    let master_key_bytes = crate::storage::encryption::parse_master_key_hex(&master_key_hex)?;

    // Prepare context for async execution
    let archive_context = ArchiveContext {
        storage: storage.clone(),
        master_key: master_key_bytes.to_vec(),
    };

    // Clone for async task
    let db_clone = db.clone();
    let records_clone = records;

    // Spawn async archival in background (fire-and-forget)
    tokio::spawn(async move {
        if let Err(e) =
            execute_archive_job(&db_clone, &archive_context, archive_id, records_clone).await
        {
            error!(
                archive_job_id = %archive_id,
                error = %e,
                "Archive job execution failed"
            );
        }
    });

    Ok(archive_id)
}

/// Fetch pending archive jobs for execution
pub async fn fetch_pending_archive_jobs(db: &PgPool, limit: i32) -> Result<Vec<ArchiveJob>> {
    let jobs = sqlx::query_as::<_, ArchiveJob>(
        "SELECT id, sync_job_id, source_id, stream_name, s3_key, status,
                retry_count, max_retries, record_count, size_bytes,
                min_timestamp, max_timestamp
         FROM data.archive_jobs
         WHERE status IN ('pending', 'failed')
         ORDER BY created_at ASC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(jobs)
}
