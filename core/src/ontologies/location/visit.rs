//! Location visit ontology
//!
//! Clustered visits derived from location_point.
//! Primary container for WHERE in narrative synthesis.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::ontologies::{
    BoundaryDetector, NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor,
};
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;

pub struct LocationVisitOntology;

impl OntologyDescriptor for LocationVisitOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("location_visit")
            .display_name("Location Visits")
            .description("Clustered location visits with place resolution")
            .domain("location")
            .table_name("location_visit")
            // Derived from location_point via clustering transform
            .source_streams(vec!["location_point"])
            .narrative_role(NarrativeRole::Container)
            // Interval detection - visits have start/end times
            .interval_boundaries(
                "start_time",
                "end_time",
                // Filter for significant visits (>30 minutes)
                vec![("EXTRACT(EPOCH FROM (end_time - start_time))", "> 1800")],
                0.90, // High fidelity - GPS is reliable
                100,  // Highest weight - location is master container
                vec!["place_name", "latitude", "longitude"],
            )
            // Substance query for WHERE extraction
            .substance_query(
                r#"
                SELECT
                    lv.id,
                    lv.place_id,
                    ep.canonical_name as place_name
                FROM data.location_visit lv
                LEFT JOIN data.entities_place ep ON lv.place_id = ep.id
                WHERE lv.start_time <= $2
                  AND lv.end_time >= $1
                ORDER BY (lv.end_time - lv.start_time) DESC
                LIMIT 1
                "#,
                vec!["where"],
                true, // Primary source for WHERE
            )
            .build()
    }
}

#[async_trait]
impl BoundaryDetector for LocationVisitOntology {
    fn ontology_name(&self) -> &'static str {
        "location_visit"
    }

    async fn detect_boundaries(
        &self,
        db: &Database,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<BoundaryCandidate>> {
        // Query visits with JOIN to get place names
        // Only include visits >30 minutes to filter out brief stops
        let visits = sqlx::query!(
            r#"
            SELECT
                lv.start_time,
                lv.end_time,
                lv.place_id,
                ep.canonical_name as place_name
            FROM data.location_visit lv
            LEFT JOIN data.entities_place ep ON lv.place_id = ep.id
            WHERE lv.start_time < $2 AND lv.end_time > $1
              AND (lv.end_time - lv.start_time) >= INTERVAL '30 minutes'
            ORDER BY lv.start_time ASC
            "#,
            start_time,
            end_time
        )
        .fetch_all(db.pool())
        .await?;

        let mut boundaries = Vec::new();

        for visit in &visits {
            // Arrival boundary
            boundaries.push(BoundaryCandidate {
                timestamp: visit.start_time,
                boundary_type: BoundaryType::Begin,
                source_ontology: "location_visit".to_string(),
                fidelity: 0.90, // High fidelity - GPS clustering is reliable
                weight: 100,    // Master container - highest priority
                metadata: serde_json::json!({
                    "type": "arrival",
                    "place_id": visit.place_id,
                    "place_name": visit.place_name
                }),
            });

            // Departure boundary
            boundaries.push(BoundaryCandidate {
                timestamp: visit.end_time,
                boundary_type: BoundaryType::End,
                source_ontology: "location_visit".to_string(),
                fidelity: 0.90,
                weight: 100,
                metadata: serde_json::json!({
                    "type": "departure",
                    "place_id": visit.place_id,
                    "place_name": visit.place_name
                }),
            });
        }

        tracing::debug!(
            "Location detector: Found {} visits (>30min), {} boundaries",
            visits.len(),
            boundaries.len()
        );

        Ok(boundaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_visit_ontology() {
        let ont = LocationVisitOntology::descriptor();
        assert_eq!(ont.name, "location_visit");
        assert_eq!(ont.narrative_role, NarrativeRole::Container);
        assert_eq!(ont.boundary.weight, 100);
        assert!(ont.substance.is_primary);
    }
}
