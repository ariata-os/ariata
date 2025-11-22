//! Event Segmentation
//!
//! Converts a stream of primary boundaries into discrete event segments.
//!
//! ## Algorithm
//!
//! - Strong cuts (weight > 150): Always start new event
//! - Moderate cuts (weight 80-150): Start new event if >15 mins from last
//! - Weak cuts are already filtered during aggregation
//!
//! Each segment captures:
//! - Temporal bounds (start/end)
//! - Contributing boundaries (for evidence)

use chrono::{DateTime, Utc};
use crate::timeline::boundaries::aggregation::PrimaryBoundary;

/// Minimum segment duration (filter very short events)
const MIN_SEGMENT_DURATION_MINUTES: i64 = 5;

/// Strong cut threshold - always creates new event
const STRONG_CUT_WEIGHT: i32 = 150;

/// Moderate cut threshold - creates event if sufficient time gap
const MODERATE_CUT_WEIGHT: i32 = 80;

/// Time gap required for moderate cut to create new event
const MODERATE_CUT_GAP_MINUTES: i64 = 15;

/// An event segment (contiguous time block with associated boundaries)
#[derive(Debug, Clone)]
pub struct EventSegment {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub contributing_boundaries: Vec<PrimaryBoundary>,
}

/// Segment primary boundaries into discrete events
///
/// This uses weight-based heuristics to determine event boundaries.
pub fn segment_events(boundaries: Vec<PrimaryBoundary>) -> Vec<EventSegment> {
    if boundaries.is_empty() {
        return vec![];
    }

    let mut segments = Vec::new();
    let mut current_segment_start = boundaries[0].timestamp;
    let mut current_boundaries = vec![boundaries[0].clone()];

    for i in 1..boundaries.len() {
        let boundary = &boundaries[i];
        let prev_boundary = &boundaries[i - 1];

        let time_gap = (boundary.timestamp - prev_boundary.timestamp).num_minutes();
        let should_split = should_start_new_segment(
            boundary.aggregate_weight,
            time_gap,
            &current_boundaries,
        );

        if should_split {
            // Close current segment
            let segment = EventSegment {
                start_time: current_segment_start,
                end_time: prev_boundary.timestamp,
                contributing_boundaries: current_boundaries.clone(),
            };

            // Only keep segments above minimum duration
            if (segment.end_time - segment.start_time).num_minutes() >= MIN_SEGMENT_DURATION_MINUTES {
                segments.push(segment);
            }

            // Start new segment
            current_segment_start = boundary.timestamp;
            current_boundaries = vec![boundary.clone()];
        } else {
            // Extend current segment
            current_boundaries.push(boundary.clone());
        }
    }

    // Close final segment
    let last_boundary = boundaries.last().unwrap();
    let final_segment = EventSegment {
        start_time: current_segment_start,
        end_time: last_boundary.timestamp,
        contributing_boundaries: current_boundaries,
    };

    if (final_segment.end_time - final_segment.start_time).num_minutes() >= MIN_SEGMENT_DURATION_MINUTES {
        segments.push(final_segment);
    }

    tracing::debug!(
        input_boundaries = boundaries.len(),
        output_segments = segments.len(),
        "Segmentation complete"
    );

    segments
}

/// Determine if a boundary should start a new segment
fn should_start_new_segment(
    weight: i32,
    time_gap_minutes: i64,
    current_boundaries: &[PrimaryBoundary],
) -> bool {
    // Strong cut - always split
    if weight >= STRONG_CUT_WEIGHT {
        tracing::trace!(
            weight,
            "Strong cut detected - starting new segment"
        );
        return true;
    }

    // Moderate cut - split if sufficient time gap
    if weight >= MODERATE_CUT_WEIGHT && time_gap_minutes >= MODERATE_CUT_GAP_MINUTES {
        tracing::trace!(
            weight,
            time_gap_minutes,
            "Moderate cut with sufficient gap - starting new segment"
        );
        return true;
    }

    // Check if we've been in the same segment too long (>2 hours)
    if !current_boundaries.is_empty() {
        let segment_duration = (current_boundaries.last().unwrap().timestamp
            - current_boundaries.first().unwrap().timestamp)
            .num_minutes();

        if segment_duration > 120 {
            tracing::trace!(
                segment_duration,
                "Segment too long - forcing split"
            );
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_boundary(timestamp: DateTime<Utc>, weight: i32) -> PrimaryBoundary {
        PrimaryBoundary {
            id: Uuid::new_v4(),
            timestamp,
            source_ontology: "test".to_string(),
            weight,
            boundary_type: "begin".to_string(),
            metadata: serde_json::json!({}),
            aggregate_weight: weight,
        }
    }

    #[test]
    fn test_strong_cut_creates_segment() {
        let now = Utc::now();
        let boundaries = vec![
            make_boundary(now, 100),
            make_boundary(now + Duration::minutes(10), 160), // Strong cut
        ];

        let segments = segment_events(boundaries);
        assert_eq!(segments.len(), 1, "Strong cut should create segment");
    }

    #[test]
    fn test_moderate_cut_with_gap() {
        let now = Utc::now();
        let boundaries = vec![
            make_boundary(now, 100),
            make_boundary(now + Duration::minutes(20), 90), // Moderate cut with >15 min gap
        ];

        let segments = segment_events(boundaries);
        assert_eq!(segments.len(), 1, "Moderate cut with gap should create segment");
    }

    #[test]
    fn test_moderate_cut_without_gap() {
        let now = Utc::now();
        let boundaries = vec![
            make_boundary(now, 100),
            make_boundary(now + Duration::minutes(5), 90), // Moderate cut but <15 min gap
        ];

        let segments = segment_events(boundaries);
        assert_eq!(segments.len(), 0, "Moderate cut without gap should not create segment (too short)");
    }

    #[test]
    fn test_minimum_duration_filter() {
        let now = Utc::now();
        let boundaries = vec![
            make_boundary(now, 100),
            make_boundary(now + Duration::minutes(2), 160), // Strong cut but very short segment
        ];

        let segments = segment_events(boundaries);
        assert_eq!(segments.len(), 0, "Segments <5 mins should be filtered");
    }
}
