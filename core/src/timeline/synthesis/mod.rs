//! Narrative Primitive Synthesis
//!
//! Converts aggregated event boundaries into narrative primitives using the 6 W's framework:
//! - WHEN: Temporal segmentation based on primary boundaries
//! - WHERE: Place extraction from location boundaries
//! - WHO: Participant extraction from calendar boundaries
//! - WHAT: Activity categorization from app usage
//! - HOW: Biometric/environmental context
//! - WHY: Declared intent from calendar
//!
//! ## Pipeline
//!
//! 1. **Segmentation**: Group boundaries into temporal event segments
//! 2. **Substance Extraction**: Query ontology primitives to extract who/where/what/how/why
//! 3. **Evidence Building**: Create refs linking back to source data
//! 4. **Insertion**: Write narrative_primitive records

pub mod segmentation;
pub mod substance;
pub mod evidence;

use chrono::{DateTime, Utc};
use crate::database::Database;
use crate::error::Result;

pub use segmentation::{EventSegment, segment_events};
pub use substance::{SubstanceData, extract_substance};
pub use evidence::{EvidenceRef, build_evidence_refs};

/// Synthesize narrative primitives from aggregated boundaries
///
/// This is the main entry point for the synthesis pipeline.
/// Returns the number of narrative primitives created.
pub async fn synthesize_primitives(
    db: &Database,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Result<usize> {
    tracing::info!(
        start = %window_start,
        end = %window_end,
        "Starting narrative primitive synthesis"
    );

    // 1. Get primary boundaries (result of aggregation)
    let primary_boundaries = crate::timeline::boundaries::aggregation::get_primary_boundaries(
        db,
        window_start,
        window_end,
    )
    .await?;

    if primary_boundaries.is_empty() {
        tracing::debug!("No primary boundaries to synthesize");
        return Ok(0);
    }

    tracing::debug!(
        boundary_count = primary_boundaries.len(),
        "Fetched primary boundaries"
    );

    // 2. Segment boundaries into events
    let segments = segmentation::segment_events(primary_boundaries);

    tracing::debug!(
        segment_count = segments.len(),
        "Segmented boundaries into events"
    );

    // 3. Extract substance and create primitives
    let mut primitives_created = 0;
    for segment in segments {
        match create_narrative_primitive(db, segment).await {
            Ok(_) => primitives_created += 1,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to create narrative primitive"
                );
            }
        }
    }

    tracing::info!(
        primitives_created,
        "Narrative primitive synthesis completed"
    );

    Ok(primitives_created)
}

/// Create a single narrative primitive from an event segment
async fn create_narrative_primitive(db: &Database, segment: EventSegment) -> Result<()> {
    tracing::debug!(
        start = %segment.start_time,
        end = %segment.end_time,
        duration_mins = (segment.end_time - segment.start_time).num_minutes(),
        boundaries = segment.contributing_boundaries.len(),
        "Creating narrative primitive"
    );

    // Extract substance (who/where/what/how/why)
    let substance = substance::extract_substance(db, &segment).await?;

    // Build evidence refs
    let evidence_refs = evidence::build_evidence_refs(&segment);

    // Calculate total weight
    let total_weight: i32 = segment
        .contributing_boundaries
        .iter()
        .map(|b| b.aggregate_weight)
        .sum();

    // Get changepoint drivers (which ontologies contributed)
    let changepoint_drivers: Vec<String> = segment
        .contributing_boundaries
        .iter()
        .map(|b| b.source_ontology.clone())
        .collect();

    // Insert narrative primitive idempotently
    sqlx::query!(
        r#"
        INSERT INTO data.narrative_primitive (
            start_time,
            end_time,
            place_id,
            place_label,
            is_transit,
            participant_ids,
            participant_context,
            primary_activity,
            secondary_activities,
            activity_payload,
            biometric_context,
            declared_intent,
            changepoint_drivers,
            changepoint_weight,
            evidence_refs
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
        )
        ON CONFLICT (start_time, end_time)
        DO UPDATE SET
            place_id = EXCLUDED.place_id,
            place_label = EXCLUDED.place_label,
            is_transit = EXCLUDED.is_transit,
            participant_ids = EXCLUDED.participant_ids,
            participant_context = EXCLUDED.participant_context,
            primary_activity = EXCLUDED.primary_activity,
            secondary_activities = EXCLUDED.secondary_activities,
            activity_payload = EXCLUDED.activity_payload,
            biometric_context = EXCLUDED.biometric_context,
            declared_intent = EXCLUDED.declared_intent,
            changepoint_drivers = EXCLUDED.changepoint_drivers,
            changepoint_weight = EXCLUDED.changepoint_weight,
            evidence_refs = EXCLUDED.evidence_refs,
            updated_at = NOW()
        "#,
        segment.start_time,
        segment.end_time,
        substance.place_id,
        substance.place_label,
        substance.is_transit,
        &substance.participant_ids,
        substance.participant_context,
        substance.primary_activity,
        &substance.secondary_activities,
        substance.activity_payload,
        substance.biometric_context,
        substance.declared_intent,
        &changepoint_drivers,
        total_weight,
        evidence_refs
    )
    .execute(db.pool())
    .await?;

    tracing::info!(
        start = %segment.start_time,
        place = ?substance.place_label,
        activity = ?substance.primary_activity,
        weight = total_weight,
        "Created narrative primitive"
    );

    Ok(())
}
