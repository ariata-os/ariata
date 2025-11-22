//! Evidence References
//!
//! Builds JSONB evidence_refs that link narrative primitives back to
//! source ontology primitives.
//!
//! This creates a graph edge from the synthesized narrative back to
//! the raw data, enabling:
//! - Provenance tracking
//! - Drill-down from narrative to details
//! - Debugging/validation of synthesis

use crate::ontologies::ontology_registry;
use serde_json::{json, Value as JsonValue};
use super::segmentation::EventSegment;

/// Evidence reference linking to a source ontology primitive
#[derive(Debug, Clone)]
pub struct EvidenceRef {
    pub table: String,
    pub id: String,
    pub role: String,  // "container", "structure", "substance"
}

/// Build evidence refs from an event segment
///
/// Maps each boundary to its ontology role:
/// - location_visit → "container" (the stage/where)
/// - praxis_calendar → "structure" (the script/why)
/// - activity_app_usage → "substance" (the action/what)
pub fn build_evidence_refs(segment: &EventSegment) -> JsonValue {
    let mut refs = Vec::new();

    for boundary in &segment.contributing_boundaries {
        let role = determine_ontology_role(&boundary.source_ontology);

        // Extract source IDs from boundary metadata if available
        // For now, we just store the boundary itself as evidence
        refs.push(json!({
            "table": "event_boundaries",
            "id": boundary.id.to_string(),
            "source_ontology": boundary.source_ontology,
            "role": role,
            "timestamp": boundary.timestamp.to_rfc3339(),
            "weight": boundary.aggregate_weight
        }));
    }

    json!(refs)
}

/// Determine the role of an ontology in narrative construction
///
/// Uses the unified ontology registry to look up narrative roles.
fn determine_ontology_role(ontology_name: &str) -> &'static str {
    // Use the ontology registry to look up the role
    ontology_registry()
        .get_narrative_role(ontology_name)
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_ontology_role() {
        // These now use the ontology registry
        assert_eq!(determine_ontology_role("location_visit"), "container");
        assert_eq!(determine_ontology_role("praxis_calendar"), "structure");
        assert_eq!(determine_ontology_role("activity_app_usage"), "substance");
        assert_eq!(determine_ontology_role("unknown_ontology"), "unknown");
    }
}
