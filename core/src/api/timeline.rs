//! Timeline API - Boundary detection and day views
//!
//! Provides access to detected temporal boundaries and structured day views.

use crate::database::Database;
use crate::error::Result;
use crate::timeline::events::EventBoundary;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Get all boundaries within a time range
pub async fn get_boundaries(
    db: &Database,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<EventBoundary>> {
    EventBoundary::find_in_range(db, start, end).await
}

/// Timeline block - represents a time segment in the day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBlock {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>, // None if still ongoing
    pub source_ontology: String,
    pub boundary_type: String,
    pub fidelity: f64,
    pub metadata: serde_json::Value,
}

/// Get a day view with boundaries organized into blocks
///
/// Takes all boundaries for a given day and pairs begin/end markers
/// into temporal blocks. Handles unclosed boundaries (still ongoing).
///
/// Uses an extended query window to capture events that span midnight,
/// then filters to blocks that overlap with the requested day.
pub async fn get_day_view(db: &Database, date: NaiveDate) -> Result<Vec<TimelineBlock>> {
    // Get start/end of day in UTC
    let day_start = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| crate::error::Error::InvalidInput("Invalid date".to_string()))?
        .and_utc();
    let day_end = date
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| crate::error::Error::InvalidInput("Invalid date".to_string()))?
        .and_utc();

    // Extend query window by 12 hours to capture events that span midnight
    let query_start = day_start - chrono::Duration::hours(12);
    let query_end = day_end + chrono::Duration::hours(12);

    let boundaries = EventBoundary::find_in_range(db, query_start, query_end).await?;

    // Convert boundaries to blocks
    let mut blocks = Vec::new();
    let mut open_blocks: std::collections::HashMap<String, TimelineBlock> =
        std::collections::HashMap::new();

    for boundary in boundaries {
        if boundary.boundary_type == "begin" {
            // Start a new block
            let block = TimelineBlock {
                start_time: boundary.timestamp,
                end_time: None,
                source_ontology: boundary.source_ontology.clone(),
                boundary_type: boundary.boundary_type.clone(),
                fidelity: boundary.fidelity,
                metadata: boundary.metadata.clone(),
            };

            // If there's an existing open block for this source, close it
            if let Some(existing) = open_blocks.insert(boundary.source_ontology.clone(), block) {
                blocks.push(existing);
            }
        } else if boundary.boundary_type == "end" {
            // Close the matching begin block
            if let Some(mut block) = open_blocks.remove(&boundary.source_ontology) {
                block.end_time = Some(boundary.timestamp);
                blocks.push(block);
            }
        }
    }

    // Add any unclosed blocks
    for (_, block) in open_blocks {
        blocks.push(block);
    }

    // Filter to blocks that overlap with the requested day
    // A block overlaps if it starts before day_end AND ends after day_start
    blocks.retain(|block| {
        let block_end = block.end_time.unwrap_or(day_end);
        block.start_time < day_end && block_end > day_start
    });

    // Sort by start time
    blocks.sort_by_key(|b| b.start_time);

    Ok(blocks)
}
