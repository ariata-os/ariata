//! Shared transform triggering logic for both cloud syncs and device ingest
//!
//! This module contains the logic for creating and executing transform jobs
//! after records have been collected, whether from cloud API syncs or device ingest batches.

use crate::error::Result;
use crate::jobs::models::{CreateJobRequest, JobStatus, JobType};
use crate::jobs::{JobExecutor, TransformContext};
use crate::sources::base::MemoryDataSource;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Create and execute a transform job for a stream with in-memory records (hot path)
///
/// This function:
/// 1. Checks if a transform exists for the stream using the centralized registry
/// 2. Creates a transform job in the database
/// 3. Creates a MemoryDataSource with the provided records
/// 4. Executes the transform asynchronously using the hot path
///
/// # Arguments
///
/// * `db` - Database connection pool
/// * `executor` - Job executor for async execution (not used with memory path, but kept for consistency)
/// * `context` - Transform context with storage and API keys
/// * `source_id` - UUID of the data source
/// * `stream_name` - Name of the stream (e.g., "healthkit", "location")
/// * `records` - In-memory records to transform (if None, uses S3 cold path)
///
/// # Returns
///
/// Returns Ok(()) even if no transform is configured (just logs a debug message).
/// Returns Err only on database or execution errors.
pub async fn create_transform_job_for_stream(
    db: &PgPool,
    executor: &JobExecutor,
    context: &Arc<TransformContext>,
    source_id: Uuid,
    stream_name: &str,
    records: Option<Vec<serde_json::Value>>,
) -> Result<Uuid> {
    // Normalize stream name using centralized registry function
    let table_name = crate::transforms::normalize_stream_name(stream_name);

    // Use centralized transform registry as single source of truth
    let route = match crate::transforms::get_transform_route(&table_name) {
        Ok(route) => route,
        Err(e) => {
            tracing::debug!(
                error = %e,
                stream_name,
                table_name,
                "No transform configured for stream, skipping transform job creation"
            );
            return Err(e);
        }
    };

    // Create transform job metadata from registry
    let metadata = json!({
        "source_table": route.source_table,
        "target_table": route.target_tables[0], // Use first target for now
        "domain": route.domain,
    });

    // Create the transform job
    let request = CreateJobRequest {
        job_type: JobType::Transform,
        status: JobStatus::Pending,
        source_id: Some(source_id),
        stream_name: Some(stream_name.to_string()),
        sync_mode: None,
        transform_id: None,
        transform_strategy: None,
        parent_job_id: None,
        transform_stage: None,
        metadata,
    };

    let job = crate::jobs::create_job(db, request).await?;

    // If we have records, create a custom context with MemoryDataSource for direct transform
    if let Some(records) = records {
        tracing::info!(
            job_id = %job.id,
            source_id = %source_id,
            stream_name,
            record_count = records.len(),
            source_table = route.source_table,
            target_table = route.target_tables[0],
            domain = route.domain,
            "Created transform job with memory data source (HOT PATH - direct transform)"
        );

        // Create MemoryDataSource with records
        let memory_source = MemoryDataSource::new(
            records,
            source_id,
            stream_name.to_string(),
            None, // min_timestamp - could be extracted if needed
            None, // max_timestamp - could be extracted if needed
            db.clone(),
        );

        // Create a new context with memory data source using with_data_source constructor
        let transform_context_with_memory = Arc::new(TransformContext::with_data_source(
            Arc::clone(&context.storage),
            context.stream_writer.clone(),
            Arc::new(memory_source),
            context.api_keys.clone(),
        ));

        // Create a new executor with the memory-enabled context
        let memory_executor =
            JobExecutor::new(db.clone(), (*transform_context_with_memory).clone());

        // Execute with memory data source
        memory_executor.execute_async(job.id);
    } else {
        tracing::info!(
            job_id = %job.id,
            source_id = %source_id,
            stream_name,
            source_table = route.source_table,
            target_table = route.target_tables[0],
            domain = route.domain,
            "Created transform job with S3 data source (COLD PATH - traditional S3 read)"
        );

        // Execute with standard S3 reader (cold path)
        executor.execute_async(job.id);
    }

    Ok(job.id)
}
