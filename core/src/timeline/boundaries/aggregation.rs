//! Boundary Aggregation
//!
//! Merges boundaries within temporal windows to reduce noise and identify
//! the strongest signals for narrative primitive creation.
//!
//! ## Algorithm
//!
//! 1. Group boundaries within 2-minute windows
//! 2. Sum weights of co-occurring boundaries
//! 3. Mark strongest boundary in each group as "primary"
//! 4. Filter weak boundary groups (total weight < threshold)
//!
//! ## Weight-Based Event Creation
//!
//! - Weight > 150: Strong cut (always creates new narrative primitive)
//! - Weight 80-150: Moderate cut (creates primitive if >15 mins from last)
//! - Weight < 80: Weak cut (ignored/filtered)

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;

/// Aggregation window (boundaries within this window are grouped)
const AGGREGATION_WINDOW_MINUTES: i64 = 2;

/// Minimum total weight to keep a boundary group
const MIN_AGGREGATE_WEIGHT: i32 = 50;

/// Boundary with weight for aggregation
#[derive(Debug, Clone)]
struct WeightedBoundary {
    id: Uuid,
    timestamp: DateTime<Utc>,
    source_ontology: String,
    weight: i32,
    _boundary_type: String,
}

/// Aggregate boundaries in time window
///
/// This function groups boundaries by temporal proximity, sums weights,
/// and marks primary boundaries for downstream processing.
///
/// Returns the number of boundary groups processed.
pub async fn aggregate_boundaries(
    db: &Database,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Result<usize> {
    tracing::info!(
        start = %window_start,
        end = %window_end,
        "Starting boundary aggregation"
    );

    // 1. Fetch all boundaries in window
    let boundaries = fetch_boundaries(db, window_start, window_end).await?;

    if boundaries.is_empty() {
        tracing::debug!("No boundaries to aggregate");
        return Ok(0);
    }

    tracing::debug!(
        boundary_count = boundaries.len(),
        "Fetched boundaries for aggregation"
    );

    // 2. Group boundaries by temporal proximity
    let groups = group_boundaries_by_proximity(boundaries);

    tracing::debug!(
        group_count = groups.len(),
        "Grouped boundaries into temporal windows"
    );

    // 3. Process each group: calculate total weight, mark primary
    let mut groups_processed = 0;
    for group in groups {
        match process_boundary_group(db, group).await {
            Ok(_) => groups_processed += 1,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to process boundary group"
                );
            }
        }
    }

    tracing::info!(
        groups_processed,
        "Boundary aggregation completed"
    );

    Ok(groups_processed)
}

/// Fetch boundaries from database in time window
async fn fetch_boundaries(
    db: &Database,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<WeightedBoundary>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            timestamp,
            source_ontology,
            weight,
            boundary_type
        FROM data.event_boundaries
        WHERE timestamp >= $1
          AND timestamp < $2
        ORDER BY timestamp ASC
        "#,
        start,
        end
    )
    .fetch_all(db.pool())
    .await?;

    let boundaries = rows
        .into_iter()
        .map(|row| WeightedBoundary {
            id: row.id,
            timestamp: row.timestamp,
            source_ontology: row.source_ontology,
            weight: row.weight,
            _boundary_type: row.boundary_type,
        })
        .collect();

    Ok(boundaries)
}

/// Group boundaries by temporal proximity (2-minute windows)
fn group_boundaries_by_proximity(boundaries: Vec<WeightedBoundary>) -> Vec<Vec<WeightedBoundary>> {
    if boundaries.is_empty() {
        return vec![];
    }

    let mut groups: Vec<Vec<WeightedBoundary>> = Vec::new();
    let mut current_group = vec![boundaries[0].clone()];

    for i in 1..boundaries.len() {
        let prev_timestamp = current_group.last().unwrap().timestamp;
        let curr_timestamp = boundaries[i].timestamp;

        let gap = (curr_timestamp - prev_timestamp).num_minutes();

        if gap <= AGGREGATION_WINDOW_MINUTES {
            // Within window - add to current group
            current_group.push(boundaries[i].clone());
        } else {
            // Gap too large - start new group
            groups.push(current_group);
            current_group = vec![boundaries[i].clone()];
        }
    }

    // Push final group
    groups.push(current_group);

    groups
}

/// Process a boundary group: calculate weight, mark primary, filter if needed
async fn process_boundary_group(db: &Database, group: Vec<WeightedBoundary>) -> Result<()> {
    if group.is_empty() {
        return Ok(());
    }

    // Calculate total weight
    let total_weight: i32 = group.iter().map(|b| b.weight).sum();

    // Filter weak groups
    if total_weight < MIN_AGGREGATE_WEIGHT {
        tracing::debug!(
            total_weight,
            boundary_count = group.len(),
            "Filtering weak boundary group"
        );
        return Ok(());
    }

    // Find the boundary with highest weight (primary)
    let primary_boundary = group
        .iter()
        .max_by_key(|b| b.weight)
        .unwrap();

    tracing::debug!(
        total_weight,
        primary_source = %primary_boundary.source_ontology,
        primary_weight = primary_boundary.weight,
        boundary_count = group.len(),
        timestamp = %primary_boundary.timestamp,
        "Processing boundary group"
    );

    // Mark primary boundary in database
    sqlx::query!(
        r#"
        UPDATE data.event_boundaries
        SET is_primary = true
        WHERE id = $1
        "#,
        primary_boundary.id
    )
    .execute(db.pool())
    .await?;

    // Optionally: Store aggregate metadata on primary boundary
    // This could include contributing ontologies, total weight, etc.
    let contributing_ontologies: Vec<String> = group
        .iter()
        .map(|b| b.source_ontology.clone())
        .collect();

    let aggregate_metadata = serde_json::json!({
        "aggregate_weight": total_weight,
        "contributing_ontologies": contributing_ontologies,
        "boundary_count": group.len(),
    });

    sqlx::query!(
        r#"
        UPDATE data.event_boundaries
        SET metadata = metadata || $1
        WHERE id = $2
        "#,
        aggregate_metadata,
        primary_boundary.id
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

/// Get aggregated boundaries for narrative synthesis
///
/// Returns only primary boundaries (marked during aggregation),
/// which represent the strongest signals for event creation.
pub async fn get_primary_boundaries(
    db: &Database,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<PrimaryBoundary>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            timestamp,
            source_ontology,
            weight,
            boundary_type,
            metadata
        FROM data.event_boundaries
        WHERE timestamp >= $1
          AND timestamp < $2
          AND is_primary = true
        ORDER BY timestamp ASC
        "#,
        start,
        end
    )
    .fetch_all(db.pool())
    .await?;

    let boundaries = rows
        .into_iter()
        .map(|row| {
            let aggregate_weight = row
                .metadata
                .as_ref()
                .and_then(|m| m.get("aggregate_weight"))
                .and_then(|w| w.as_i64())
                .map(|w| w as i32)
                .unwrap_or(row.weight);

            PrimaryBoundary {
                id: row.id,
                timestamp: row.timestamp,
                source_ontology: row.source_ontology,
                weight: row.weight,
                boundary_type: row.boundary_type,
                metadata: row.metadata.unwrap_or_default(),
                aggregate_weight,
            }
        })
        .collect();

    Ok(boundaries)
}

/// Primary boundary (result of aggregation)
#[derive(Debug, Clone)]
pub struct PrimaryBoundary {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source_ontology: String,
    pub weight: i32,
    pub boundary_type: String,
    pub metadata: serde_json::Value,
    pub aggregate_weight: i32,  // Total weight from all contributing boundaries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_boundaries_by_proximity() {
        let boundaries = vec![
            WeightedBoundary {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source_ontology: "location_visit".to_string(),
                weight: 100,
                _boundary_type: "begin".to_string(),
            },
            WeightedBoundary {
                id: Uuid::new_v4(),
                timestamp: Utc::now() + Duration::minutes(1),
                source_ontology: "praxis_calendar".to_string(),
                weight: 80,
                _boundary_type: "begin".to_string(),
            },
            WeightedBoundary {
                id: Uuid::new_v4(),
                timestamp: Utc::now() + Duration::minutes(10),  // Gap > 2 mins
                source_ontology: "device_activity".to_string(),
                weight: 60,
                _boundary_type: "begin".to_string(),
            },
        ];

        let groups = group_boundaries_by_proximity(boundaries);

        assert_eq!(groups.len(), 2, "Should have 2 groups (gap at 10 mins)");
        assert_eq!(groups[0].len(), 2, "First group should have 2 boundaries");
        assert_eq!(groups[1].len(), 1, "Second group should have 1 boundary");
    }

    #[test]
    fn test_group_single_boundary() {
        let boundaries = vec![
            WeightedBoundary {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source_ontology: "location_visit".to_string(),
                weight: 100,
                _boundary_type: "begin".to_string(),
            },
        ];

        let groups = group_boundaries_by_proximity(boundaries);

        assert_eq!(groups.len(), 1, "Should have 1 group");
        assert_eq!(groups[0].len(), 1, "Group should have 1 boundary");
    }

    #[test]
    fn test_group_empty() {
        let boundaries = vec![];
        let groups = group_boundaries_by_proximity(boundaries);
        assert_eq!(groups.len(), 0, "Should have 0 groups");
    }
}
