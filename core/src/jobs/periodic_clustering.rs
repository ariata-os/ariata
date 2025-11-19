//! Periodic location visit clustering job
//!
//! Runs hourly to cluster location_point primitives into location_visit records.

use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::jobs::TransformContext;
use crate::sources::base::OntologyTransform;
use crate::transforms::enrich::location::LocationVisitTransform;

/// Start the periodic location clustering job
///
/// Runs every hour (at :00) to cluster location points from the last 12 hours.
pub async fn start_location_clustering_job(
    db_pool: PgPool,
    context: Arc<TransformContext>,
) -> Result<()> {
    let scheduler = JobScheduler::new().await.map_err(|e| {
        crate::error::Error::Other(format!("Failed to create scheduler: {}", e))
    })?;

    // Every hour at :00 (6-field cron: sec min hour day month dow)
    let job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let db = db_pool.clone();
        let ctx = context.clone();

        Box::pin(async move {
            match run_location_clustering(&db, &ctx).await {
                Ok(visit_count) => {
                    tracing::info!(
                        visit_count,
                        "Location clustering job completed successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Location clustering job failed"
                    );
                }
            }
        })
    })
    .map_err(|e| crate::error::Error::Other(format!("Failed to create cron job: {}", e)))?;

    scheduler.add(job).await.map_err(|e| {
        crate::error::Error::Other(format!("Failed to add job to scheduler: {}", e))
    })?;

    scheduler.start().await.map_err(|e| {
        crate::error::Error::Other(format!("Failed to start scheduler: {}", e))
    })?;

    tracing::info!("Location clustering job scheduler started (runs hourly at :00)");

    Ok(())
}

/// Run location clustering for primary source
///
/// Note: source_id is fetched for logging purposes. The clustering
/// transform processes ALL location_point records (single-tenant assumption).
async fn run_location_clustering(db: &PgPool, context: &Arc<TransformContext>) -> Result<usize> {
    // Get primary source_id (single-tenant assumption)
    let source_id = fetch_primary_source_id(db).await?;

    // Create database wrapper
    let db_wrapper = Database::from_pool(db.clone());

    // Create visit transform
    let transform = LocationVisitTransform;

    // Run clustering
    let result = transform.transform(&db_wrapper, context, source_id).await?;

    Ok(result.records_written)
}

/// Fetch the primary source ID (for single-tenant)
///
/// In a single-tenant system, we just grab the first source ID.
/// In multi-tenant, this would iterate over all sources.
async fn fetch_primary_source_id(db: &PgPool) -> Result<Uuid> {
    let row = sqlx::query!(
        r#"
        SELECT id
        FROM data.source_connections
        ORDER BY created_at ASC
        LIMIT 1
        "#
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| crate::error::Error::NotFound("No sources found".to_string()))?;

    Ok(row.id)
}
