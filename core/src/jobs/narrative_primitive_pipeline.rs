//! Narrative Primitive Pipeline Job
//!
//! Unified hourly job that orchestrates the complete pipeline:
//! 1. Entity Resolution (inline)
//! 2. Changepoint Detection
//! 3. Boundary Aggregation
//! 4. Narrative Synthesis
//!
//! Replaces the old separate jobs (BoundarySweeperJob, PeriodicClusteringJob).

use chrono::{DateTime, Duration, Utc};
use tokio_cron_scheduler::Job;

use crate::database::Database;
use crate::entity_resolution;
use crate::timeline;
use crate::Result;

/// Default lookback window in hours
const DEFAULT_LOOKBACK_HOURS: i64 = 6;

/// Statistics from a pipeline run
#[derive(Debug, Default)]
pub struct PipelineStats {
    pub places_resolved: usize,
    pub people_resolved: usize,
    pub boundaries_detected: usize,
    pub boundary_groups_aggregated: usize,
    pub primitives_created: usize,
    pub duration_ms: u128,
    pub errors: Vec<String>,
    pub stages_completed: Vec<String>,
    pub stages_failed: Vec<String>,
}

/// Run the complete narrative primitive pipeline with default time window
///
/// Uses a 6-hour lookback from current time. This is the main entry point called by the scheduler.
pub async fn run_pipeline(db: &Database) -> Result<PipelineStats> {
    let end_time = Utc::now();
    let start_time = end_time - Duration::hours(DEFAULT_LOOKBACK_HOURS);

    run_pipeline_for_range(db, start_time, end_time).await
}

/// Run the complete narrative primitive pipeline for a specific date range
///
/// This variant allows processing historical data or custom time windows.
/// Useful for:
/// - Seeding/testing with historical data
/// - Backfilling narrative primitives
/// - Simulating hourly cron execution
pub async fn run_pipeline_for_range(
    db: &Database,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<PipelineStats> {
    let start = std::time::Instant::now();

    tracing::info!(
        start = %start_time,
        end = %end_time,
        "Starting narrative primitive pipeline"
    );

    let window = entity_resolution::TimeWindow::new(start_time, end_time);

    let mut stats = PipelineStats::default();

    // STAGE 1: Entity Resolution
    tracing::info!("Stage 1: Entity Resolution");
    match entity_resolution::resolve_entities(db, window).await {
        Ok(entity_stats) => {
            stats.places_resolved = entity_stats.places_resolved;
            stats.people_resolved = entity_stats.people_resolved;
            stats.stages_completed.push("Entity Resolution".to_string());
            tracing::info!(
                places = entity_stats.places_resolved,
                people = entity_stats.people_resolved,
                "Stage 1 completed successfully"
            );
        }
        Err(e) => {
            let error_msg = format!("Stage 1 (Entity Resolution) failed: {}", e);
            tracing::error!(error = %e, "Stage 1 failed, continuing with Stage 2");
            stats.errors.push(error_msg);
            stats.stages_failed.push("Entity Resolution".to_string());
        }
    }

    // STAGE 2: Changepoint Detection
    tracing::info!("Stage 2: Changepoint Detection");
    match timeline::boundaries::detect_boundaries(db, start_time, end_time).await {
        Ok(boundary_candidates) => {
            stats.boundaries_detected = boundary_candidates.len();

            // Persist boundaries
            match timeline::boundaries::persist_boundaries(db, boundary_candidates).await {
                Ok(_) => {
                    stats.stages_completed.push("Changepoint Detection".to_string());
                    tracing::info!(
                        boundaries = stats.boundaries_detected,
                        "Stage 2 completed successfully"
                    );
                }
                Err(e) => {
                    let error_msg = format!("Stage 2 (Boundary Persistence) failed: {}", e);
                    tracing::error!(error = %e, "Failed to persist boundaries, continuing with Stage 3");
                    stats.errors.push(error_msg);
                    stats.stages_failed.push("Changepoint Detection".to_string());
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Stage 2 (Changepoint Detection) failed: {}", e);
            tracing::error!(error = %e, "Stage 2 failed, continuing with Stage 3");
            stats.errors.push(error_msg);
            stats.stages_failed.push("Changepoint Detection".to_string());
        }
    }

    // STAGE 3: Boundary Aggregation
    tracing::info!("Stage 3: Boundary Aggregation");
    match timeline::boundaries::aggregation::aggregate_boundaries(db, start_time, end_time).await {
        Ok(boundary_groups) => {
            stats.boundary_groups_aggregated = boundary_groups;
            stats.stages_completed.push("Boundary Aggregation".to_string());
            tracing::info!(
                boundary_groups = boundary_groups,
                "Stage 3 completed successfully"
            );
        }
        Err(e) => {
            let error_msg = format!("Stage 3 (Boundary Aggregation) failed: {}", e);
            tracing::error!(error = %e, "Stage 3 failed, continuing with Stage 4");
            stats.errors.push(error_msg);
            stats.stages_failed.push("Boundary Aggregation".to_string());
        }
    }

    // STAGE 4: Narrative Synthesis
    tracing::info!("Stage 4: Narrative Synthesis");
    match timeline::synthesis::synthesize_primitives(db, start_time, end_time).await {
        Ok(primitives) => {
            stats.primitives_created = primitives;
            stats.stages_completed.push("Narrative Synthesis".to_string());
            tracing::info!(
                primitives = primitives,
                "Stage 4 completed successfully"
            );
        }
        Err(e) => {
            let error_msg = format!("Stage 4 (Narrative Synthesis) failed: {}", e);
            tracing::error!(error = %e, "Stage 4 failed");
            stats.errors.push(error_msg);
            stats.stages_failed.push("Narrative Synthesis".to_string());
        }
    }

    stats.duration_ms = start.elapsed().as_millis();

    // Determine overall success
    let all_stages_completed = stats.stages_failed.is_empty();

    if all_stages_completed {
        tracing::info!(
            places_resolved = stats.places_resolved,
            people_resolved = stats.people_resolved,
            boundaries_detected = stats.boundaries_detected,
            boundary_groups = stats.boundary_groups_aggregated,
            primitives_created = stats.primitives_created,
            duration_ms = stats.duration_ms,
            "Narrative primitive pipeline completed successfully"
        );
    } else {
        tracing::warn!(
            stages_completed = stats.stages_completed.len(),
            stages_failed = stats.stages_failed.len(),
            errors = stats.errors.len(),
            duration_ms = stats.duration_ms,
            "Narrative primitive pipeline completed with {} errors", stats.errors.len()
        );
    }

    Ok(stats)
}

/// Create the cron job for the scheduler
///
/// Runs every hour at :00
pub fn create_job(db: Database) -> Result<Job> {
    let job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let db = db.clone();
        Box::pin(async move {
            match run_pipeline(&db).await {
                Ok(stats) => {
                    if stats.stages_failed.is_empty() {
                        tracing::info!(
                            primitives = stats.primitives_created,
                            duration_ms = stats.duration_ms,
                            "Pipeline job completed successfully"
                        );
                    } else {
                        tracing::warn!(
                            primitives = stats.primitives_created,
                            stages_completed = stats.stages_completed.len(),
                            stages_failed = stats.stages_failed.len(),
                            errors = ?stats.errors,
                            duration_ms = stats.duration_ms,
                            "Pipeline job completed with partial failures"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Pipeline job failed"
                    );
                }
            }
        })
    })
    .map_err(|e| crate::error::Error::Other(format!("Failed to create cron job: {}", e)))?;

    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.places_resolved, 0);
        assert_eq!(stats.primitives_created, 0);
    }
}
