use crate::database::Database;
use crate::ontologies::{ontology_registry, DetectionAlgorithm};
use crate::timeline::events::{BoundaryType, EventBoundary};
use crate::Result;
use chrono::{DateTime, Utc};

pub mod aggregation;
pub mod algorithms;

/// Candidate boundary detected by a specific detector
#[derive(Debug, Clone)]
pub struct BoundaryCandidate {
    pub timestamp: DateTime<Utc>,
    pub boundary_type: BoundaryType,
    pub source_ontology: String,
    pub fidelity: f64,
    pub weight: i32,
    pub metadata: serde_json::Value,
}

/// Detect all boundaries within a time window using the ontology registry
///
/// This function first checks for registered boundary detectors (from ontology modules)
/// and falls back to generic algorithms for ontologies without custom detectors.
///
/// Detection priority:
/// 1. Custom detector registered via `BoundaryDetector` trait (preferred)
/// 2. Generic algorithm from `DetectionAlgorithm` config (fallback)
///
/// Errors in individual ontologies are logged but do not fail the entire detection process.
pub async fn detect_boundaries(
    db: &Database,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<Vec<BoundaryCandidate>> {
    let mut candidates = Vec::new();
    let mut successful_detectors = 0;
    let mut failed_detectors = 0;

    let registry = ontology_registry();

    // First, run all registered custom detectors
    for detector in registry.list_detectors() {
        let ontology_name = detector.ontology_name();

        match detector.detect_boundaries(db, start_time, end_time).await {
            Ok(boundaries) => {
                tracing::debug!(
                    ontology = %ontology_name,
                    boundaries = boundaries.len(),
                    "Custom detector succeeded"
                );
                candidates.extend(boundaries);
                successful_detectors += 1;
            }
            Err(e) => {
                tracing::warn!(
                    ontology = %ontology_name,
                    error = %e,
                    "Custom detector failed, continuing with other detectors"
                );
                failed_detectors += 1;
            }
        }
    }

    // Collect names of ontologies that have custom detectors
    let detector_ontologies: std::collections::HashSet<_> = registry
        .list_detectors()
        .iter()
        .map(|d| d.ontology_name())
        .collect();

    // Then, run generic algorithms for ontologies without custom detectors
    let boundary_ontologies: Vec<_> = registry.list_with_boundaries().iter().map(|o| o.name).collect();
    tracing::info!(
        ontologies = ?boundary_ontologies,
        "Ontologies with boundary detection enabled"
    );

    for ontology in registry.list_with_boundaries() {
        // Skip if this ontology has a custom detector
        if detector_ontologies.contains(ontology.name) {
            continue;
        }

        tracing::debug!(
            ontology = %ontology.name,
            table = %ontology.table_name,
            algorithm = ?ontology.boundary.algorithm,
            "Processing ontology for boundary detection"
        );

        // Run the appropriate generic detection algorithm
        let boundaries_result = match &ontology.boundary.algorithm {
            DetectionAlgorithm::Continuous {
                column,
                penalty,
                min_segment_minutes,
            } => {
                algorithms::continuous::detect(
                    db,
                    start_time,
                    end_time,
                    ontology.table_name,
                    column,
                    *penalty,
                    *min_segment_minutes,
                )
                .await
            }

            DetectionAlgorithm::Discrete {
                timestamp_col,
                duration_col,
                gap_minutes,
                min_duration_seconds,
            } => {
                algorithms::discrete::detect(
                    db,
                    start_time,
                    end_time,
                    ontology.table_name,
                    timestamp_col,
                    *duration_col,
                    *gap_minutes,
                    *min_duration_seconds,
                )
                .await
            }

            DetectionAlgorithm::Interval {
                start_col,
                end_col,
                filters,
            } => {
                algorithms::interval::detect(
                    db,
                    start_time,
                    end_time,
                    ontology.table_name,
                    start_col,
                    end_col,
                    filters,
                )
                .await
            }

            DetectionAlgorithm::None => {
                // Skip ontologies without boundary detection
                continue;
            }
        };

        match boundaries_result {
            Ok(mut boundaries) => {
                // Tag all boundaries with ontology name, fidelity score, and weight
                for boundary in &mut boundaries {
                    boundary.source_ontology = ontology.name.to_string();
                    boundary.fidelity = ontology.boundary.fidelity;
                    boundary.weight = ontology.boundary.weight;
                }

                tracing::debug!(
                    ontology = %ontology.name,
                    fidelity = ontology.boundary.fidelity,
                    boundaries = boundaries.len(),
                    "Generic algorithm detection succeeded"
                );

                candidates.extend(boundaries);
                successful_detectors += 1;
            }
            Err(e) => {
                tracing::warn!(
                    ontology = %ontology.name,
                    error = %e,
                    "Generic algorithm detection failed, continuing with other ontologies"
                );
                failed_detectors += 1;
            }
        }
    }

    // Sort by timestamp
    candidates.sort_by_key(|c| c.timestamp);

    if failed_detectors > 0 {
        tracing::warn!(
            boundaries = candidates.len(),
            successful_detectors,
            failed_detectors,
            total = successful_detectors + failed_detectors,
            "Boundary detection completed with {} failures",
            failed_detectors
        );
    } else {
        tracing::info!(
            boundaries = candidates.len(),
            detectors = successful_detectors,
            "Boundary detection completed successfully"
        );
    }

    Ok(candidates)
}

/// Persist detected boundaries to the database
pub async fn persist_boundaries(
    db: &Database,
    candidates: Vec<BoundaryCandidate>,
) -> Result<Vec<EventBoundary>> {
    let mut boundaries = Vec::new();

    for candidate in candidates {
        let boundary = EventBoundary::create(
            db,
            candidate.timestamp,
            candidate.boundary_type,
            &candidate.source_ontology,
            candidate.fidelity,
            candidate.weight,
            candidate.metadata,
        )
        .await?;

        boundaries.push(boundary);
    }

    Ok(boundaries)
}
