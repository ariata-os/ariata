//! Entity Resolution Module
//!
//! Pre-resolution pipeline that converts raw ontology primitives into canonical entities.
//! This module is called inline by the NarrativePrimitivePipeline, NOT as a separate cron job.
//!
//! ## Architecture
//!
//! Entity resolution happens BEFORE changepoint detection to ensure boundaries reference
//! properly resolved entities (places with IDs, people with canonical names).
//!
//! ## Modules
//!
//! - `places`: Location clustering (location_point → location_visit → entities_place)
//! - `people`: Calendar attendee resolution (calendar attendees → entities_person)
//!
//! ## Usage
//!
//! ```rust
//! let window = TimeWindow::new(start, end);
//! let stats = entity_resolution::resolve_entities(db, window).await?;
//! ```

pub mod places;
pub mod people;

use chrono::{DateTime, Utc};
use crate::database::Database;
use crate::error::Result;

/// Time window for entity resolution
#[derive(Debug, Clone, Copy)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeWindow {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Create a window from now - duration to now
    pub fn from_lookback_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::hours(hours);
        Self { start, end }
    }
}

/// Statistics from entity resolution
#[derive(Debug, Default)]
pub struct ResolutionStats {
    pub places_resolved: usize,
    pub people_resolved: usize,
    pub duration_ms: u128,
}

/// Main entry point: Resolve all entities in time window
///
/// This function orchestrates place and people resolution.
/// Called inline by the narrative primitive pipeline.
pub async fn resolve_entities(db: &Database, window: TimeWindow) -> Result<ResolutionStats> {
    let start = std::time::Instant::now();

    tracing::info!(
        start = %window.start,
        end = %window.end,
        "Starting entity resolution"
    );

    // 1. Resolve places (location clustering)
    let places_resolved = places::resolve_places(db, window).await?;

    // 2. Resolve people (calendar attendees)
    let people_resolved = people::resolve_people(db, window).await?;

    let duration_ms = start.elapsed().as_millis();

    tracing::info!(
        places_resolved,
        people_resolved,
        duration_ms,
        "Entity resolution completed"
    );

    Ok(ResolutionStats {
        places_resolved,
        people_resolved,
        duration_ms,
    })
}
