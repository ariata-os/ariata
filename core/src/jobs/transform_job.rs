//! Transform job execution logic
//!
//! Handles execution of transformation jobs that convert raw stream data
//! into normalized ontology tables.

use serde_json::json;
use sqlx::PgPool;

use crate::error::Result;
use crate::jobs::models::Job;
use crate::sources::base::OntologyTransform;
use crate::sources::google::gmail::transform::GmailEmailTransform;

/// Execute a transform job
///
/// This function is called by the job executor to perform the actual transformation work.
/// It routes to the appropriate transformer based on metadata and updates job status.
#[tracing::instrument(skip(db, job), fields(job_id = %job.id, job_type = "transform"))]
pub async fn execute_transform_job(db: &PgPool, job: &Job) -> Result<()> {
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

    let source_id = job.source_id.ok_or_else(|| {
        crate::Error::InvalidInput("Transform job missing source_id".into())
    })?;

    tracing::info!(
        source_table,
        target_table,
        source_id = %source_id,
        "Starting transform job execution"
    );

    // Route to appropriate transformer based on source/target pair
    let transformer: Box<dyn OntologyTransform> = match (source_table, target_table) {
        ("stream_google_gmail", "social_email") => Box::new(GmailEmailTransform),
        _ => {
            return Err(crate::Error::InvalidInput(format!(
                "Unknown transform mapping: {} -> {}",
                source_table, target_table
            )));
        }
    };

    // Create database wrapper from pool
    let db_wrapper = crate::database::Database::from_pool(db.clone());

    // Execute transformation
    let result = transformer.transform(&db_wrapper, source_id).await;

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
                UPDATE elt.jobs
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
                "Transform job completed successfully"
            );

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
                UPDATE elt.jobs
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
