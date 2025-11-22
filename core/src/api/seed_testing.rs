//! Seed Testing API
//!
//! Provides endpoints to inspect the results of the Monday in Rome seed data,
//! allowing validation of the full pipeline: Archive → Transform → Clustering → Boundary Detection

use crate::database::Database;
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Pipeline status overview showing all stages
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineStatus {
    pub archive_jobs: ArchiveJobsStatus,
    pub transform_jobs: TransformJobsStatus,
    pub location_clustering: LocationClusteringStatus,
    pub boundary_sweeper: BoundarySweeperStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveJobsStatus {
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
    pub records_archived: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransformJobsStatus {
    pub total: i64,
    pub completed: i64,
    pub records_processed: i64,
    pub ontology_tables_populated: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationClusteringStatus {
    pub raw_points: i64,
    pub visits_created: i64,
    pub has_data: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundarySweeperStatus {
    pub boundaries_detected: i64,
    pub ontologies_with_boundaries: i64,
    pub has_data: bool,
}

/// Summary of boundaries detected, grouped by ontology
#[derive(Debug, Serialize, Deserialize)]
pub struct BoundariesSummary {
    pub date_range: DateRange,
    pub by_ontology: Vec<OntologyBoundaries>,
    pub total_boundaries: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OntologyBoundaries {
    pub ontology: String,
    pub count: i64,
    pub fidelity: f64,
    pub detection_type: String,
    pub samples: Vec<BoundarySample>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundarySample {
    pub timestamp: DateTime<Utc>,
    pub boundary_type: String,
    pub metadata: serde_json::Value,
}

/// Data quality metrics for seed data
#[derive(Debug, Serialize, Deserialize)]
pub struct DataQualityMetrics {
    pub total_records: i64,
    pub boundary_coverage_percent: f64,
    pub fidelity_distribution: FidelityDistribution,
    pub time_coverage: TimeCoverage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FidelityDistribution {
    pub tier1_0_90_0_95: i64, // High fidelity
    pub tier2_0_80_0_89: i64, // Medium fidelity
    pub tier3_0_70_0_79: i64, // Low fidelity
    pub below_0_70: i64,      // Very low fidelity
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeCoverage {
    pub total_hours: f64,
    pub covered_hours: f64,
    pub coverage_percent: f64,
}

/// Get pipeline status for all stages
pub async fn get_pipeline_status(db: &Database) -> Result<PipelineStatus> {
    // Archive jobs status
    let archive_row = sqlx::query(
        r#"
        SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'completed') as completed,
            COUNT(*) FILTER (WHERE status = 'failed') as failed,
            COALESCE(SUM(records_processed), 0)::bigint as records_archived
        FROM data.jobs
        WHERE job_type = 'archive'
        "#,
    )
    .fetch_one(db.pool())
    .await?;

    let archive_jobs = ArchiveJobsStatus {
        total: archive_row.try_get("total")?,
        completed: archive_row.try_get("completed")?,
        failed: archive_row.try_get("failed")?,
        records_archived: archive_row.try_get("records_archived")?,
    };

    // Transform jobs status
    let transform_row = sqlx::query(
        r#"
        SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'completed') as completed,
            COALESCE(SUM(records_processed), 0)::bigint as records_processed
        FROM data.jobs
        WHERE job_type = 'transform'
        "#,
    )
    .fetch_one(db.pool())
    .await?;

    // Count unique ontology tables populated (rough estimate from jobs metadata)
    let ontology_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT metadata->>'stream_name') FROM data.jobs WHERE job_type = 'transform' AND status = 'completed'"
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    let transform_jobs = TransformJobsStatus {
        total: transform_row.try_get("total")?,
        completed: transform_row.try_get("completed")?,
        records_processed: transform_row.try_get("records_processed")?,
        ontology_tables_populated: ontology_count,
    };

    // Location clustering status
    let location_points: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM data.location_point")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);

    let location_visits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM data.location_visit")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);

    let location_clustering = LocationClusteringStatus {
        raw_points: location_points,
        visits_created: location_visits,
        has_data: location_visits > 0,
    };

    // Boundary sweeper status
    let boundary_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM data.event_boundaries")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);

    let ontologies_with_boundaries: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT source_ontology) FROM data.event_boundaries",
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    let boundary_sweeper = BoundarySweeperStatus {
        boundaries_detected: boundary_count,
        ontologies_with_boundaries,
        has_data: boundary_count > 0,
    };

    Ok(PipelineStatus {
        archive_jobs,
        transform_jobs,
        location_clustering,
        boundary_sweeper,
    })
}

/// Get summary of detected boundaries grouped by ontology
pub async fn get_boundaries_summary(
    db: &Database,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<BoundariesSummary> {
    // Get boundaries grouped by ontology with stats
    let rows = sqlx::query(
        r#"
        SELECT
            source_ontology,
            COUNT(*) as count,
            AVG(fidelity) as avg_fidelity,
            metadata->>'detection_type' as detection_type
        FROM data.event_boundaries
        WHERE timestamp >= $1 AND timestamp <= $2
        GROUP BY source_ontology, metadata->>'detection_type'
        ORDER BY count DESC
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(db.pool())
    .await?;

    let mut by_ontology = Vec::new();
    let mut total_boundaries = 0i64;

    for row in rows {
        let ontology: String = row.try_get("source_ontology")?;
        let count: i64 = row.try_get("count")?;
        let fidelity: f64 = row.try_get::<f64, _>("avg_fidelity")?;
        let detection_type: Option<String> = row.try_get("detection_type").ok();

        total_boundaries += count;

        // Fetch sample boundaries for this ontology (limit 3)
        let sample_rows = sqlx::query(
            r#"
            SELECT timestamp, boundary_type, metadata
            FROM data.event_boundaries
            WHERE source_ontology = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp
            LIMIT 3
            "#,
        )
        .bind(&ontology)
        .bind(start)
        .bind(end)
        .fetch_all(db.pool())
        .await?;

        let samples = sample_rows
            .iter()
            .map(|r| BoundarySample {
                timestamp: r.try_get("timestamp").unwrap(),
                boundary_type: r.try_get("boundary_type").unwrap(),
                metadata: r.try_get("metadata").unwrap_or(serde_json::json!({})),
            })
            .collect();

        by_ontology.push(OntologyBoundaries {
            ontology,
            count,
            fidelity,
            detection_type: detection_type.unwrap_or_else(|| "unknown".to_string()),
            samples,
        });
    }

    Ok(BoundariesSummary {
        date_range: DateRange { start, end },
        by_ontology,
        total_boundaries,
    })
}

/// Get data quality metrics for seed data
pub async fn get_data_quality_metrics(
    db: &Database,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<DataQualityMetrics> {
    // Total records across all seed ontologies (estimate from key tables)
    let total_records: i64 = sqlx::query_scalar(
        r#"
        SELECT
            (SELECT COUNT(*) FROM data.praxis_calendar) +
            (SELECT COUNT(*) FROM data.health_sleep) +
            (SELECT COUNT(*) FROM data.location_visit) +
            (SELECT COUNT(*) FROM data.speech_transcription) +
            (SELECT COUNT(*) FROM data.social_message) +
            (SELECT COUNT(*) FROM data.social_email) +
            (SELECT COUNT(*) FROM data.axiology_value) +
            (SELECT COUNT(*) FROM data.axiology_virtue) +
            (SELECT COUNT(*) FROM data.praxis_task)
        "#,
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    // Fidelity distribution
    let fidelity_row = sqlx::query(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE fidelity >= 0.90 AND fidelity <= 0.95) as tier1,
            COUNT(*) FILTER (WHERE fidelity >= 0.80 AND fidelity < 0.90) as tier2,
            COUNT(*) FILTER (WHERE fidelity >= 0.70 AND fidelity < 0.80) as tier3,
            COUNT(*) FILTER (WHERE fidelity < 0.70) as below
        FROM data.event_boundaries
        WHERE timestamp >= $1 AND timestamp <= $2
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(db.pool())
    .await?;

    let fidelity_distribution = FidelityDistribution {
        tier1_0_90_0_95: fidelity_row.try_get("tier1")?,
        tier2_0_80_0_89: fidelity_row.try_get("tier2")?,
        tier3_0_70_0_79: fidelity_row.try_get("tier3")?,
        below_0_70: fidelity_row.try_get("below")?,
    };

    // Time coverage (how much of the day has boundaries)
    let duration = end.signed_duration_since(start);
    let total_hours = duration.num_seconds() as f64 / 3600.0;

    // Estimate covered hours from timeline blocks
    let covered_hours: f64 = sqlx::query_scalar(
        r#"
        WITH blocks AS (
            SELECT
                b.timestamp as start_time,
                LEAD(b.timestamp) OVER (ORDER BY b.timestamp) as end_time
            FROM data.event_boundaries b
            WHERE b.timestamp >= $1 AND b.timestamp <= $2
              AND b.boundary_type = 'begin'
        )
        SELECT COALESCE(SUM(EXTRACT(EPOCH FROM (end_time - start_time)) / 3600.0), 0)
        FROM blocks
        WHERE end_time IS NOT NULL
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0.0);

    let coverage_percent = if total_hours > 0.0 {
        (covered_hours / total_hours) * 100.0
    } else {
        0.0
    };

    let time_coverage = TimeCoverage {
        total_hours,
        covered_hours,
        coverage_percent,
    };

    let boundary_coverage_percent = coverage_percent;

    Ok(DataQualityMetrics {
        total_records,
        boundary_coverage_percent,
        fidelity_distribution,
        time_coverage,
    })
}
