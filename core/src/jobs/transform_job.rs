//! Transform job execution logic
//!
//! Handles execution of transformation jobs that convert raw stream data
//! into normalized ontology tables.

use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

use crate::error::Result;
use crate::jobs::models::Job;
use crate::jobs::transform_context::TransformContext;
use crate::transforms::TransformFactory;

/// Execute a transform job
///
/// This function is called by the job executor to perform the actual transformation work.
/// It routes to the appropriate transformer based on metadata and updates job status.
///
/// # Arguments
/// * `db` - Database connection pool
/// * `context` - Transform context with storage and API keys
/// * `job` - The job to execute
#[tracing::instrument(skip(db, context, job), fields(job_id = %job.id, job_type = "transform"))]
pub async fn execute_transform_job(
    db: &PgPool,
    context: &Arc<TransformContext>,
    job: &Job,
) -> Result<()> {
    // Extract required metadata
    let source_table = job
        .metadata
        .get("source_table")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::Error::InvalidInput("Transform job missing source_table in metadata".into())
        })?;

    let target_table = job
        .metadata
        .get("target_table")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::Error::InvalidInput("Transform job missing target_table in metadata".into())
        })?;

    let source_id = job
        .source_id
        .ok_or_else(|| crate::Error::InvalidInput("Transform job missing source_id".into()))?;

    tracing::info!(
        source_table,
        target_table,
        source_id = %source_id,
        "Starting transform job execution"
    );

    // Create transformer using factory (single source of truth)
    let transformer = TransformFactory::create(source_table, target_table, context)?;

    // Create database wrapper from pool
    let db_wrapper = crate::database::Database::from_pool(db.clone());

    // Execute transformation
    let result = transformer.transform(&db_wrapper, context, source_id).await;

    match result {
        Ok(transform_result) => {
            // Build metadata with detailed transform info
            let metadata = json!({
                "source_table": source_table,
                "target_table": target_table,
                "domain": transformer.domain(),
                "records_read": transform_result.records_read,
                "records_written": transform_result.records_written,
                "records_failed": transform_result.records_failed,
                "last_processed_id": transform_result.last_processed_id,
            });

            // Update job with success
            sqlx::query(
                r#"
                UPDATE data.jobs
                SET status = 'succeeded',
                    completed_at = NOW(),
                    records_processed = $1,
                    metadata = $2
                WHERE id = $3
                "#,
            )
            .bind(transform_result.records_written as i64)
            .bind(metadata)
            .bind(job.id)
            .execute(db)
            .await?;

            tracing::info!(
                job_id = %job.id,
                source_table,
                target_table,
                records_read = transform_result.records_read,
                records_written = transform_result.records_written,
                records_failed = transform_result.records_failed,
                chained_transforms_count = transform_result.chained_transforms.len(),
                "Transform job completed successfully"
            );

            // Create chained transform jobs if any were returned
            for chained in &transform_result.chained_transforms {
                let chained_job = crate::jobs::create_chained_transform_job(
                    db,
                    job.id,
                    &chained.source_table,
                    chained.target_tables.iter().map(|s| s.as_str()).collect(),
                    &chained.domain,
                    chained.source_record_id,
                    &chained.transform_stage,
                )
                .await;

                match chained_job {
                    Ok(child_job) => {
                        tracing::info!(
                            parent_job_id = %job.id,
                            child_job_id = %child_job.id,
                            transform_stage = %chained.transform_stage,
                            source_table = %chained.source_table,
                            "Created chained transform job"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            parent_job_id = %job.id,
                            transform_stage = %chained.transform_stage,
                            error = %e,
                            "Failed to create chained transform job"
                        );
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            // Build metadata with error details
            let metadata = json!({
                "source_table": source_table,
                "target_table": target_table,
                "error": e.to_string(),
            });

            // Update job with failure
            sqlx::query(
                r#"
                UPDATE data.jobs
                SET status = 'failed',
                    completed_at = NOW(),
                    error_message = $1,
                    metadata = $2
                WHERE id = $3
                "#,
            )
            .bind(e.to_string())
            .bind(metadata)
            .bind(job.id)
            .execute(db)
            .await?;

            tracing::error!(
                job_id = %job.id,
                source_table,
                target_table,
                error = %e,
                "Transform job failed"
            );

            Err(e)
        }
    }
}
