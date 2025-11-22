//! Sleep ontology
//!
//! Sleep sessions from HealthKit with start/end times and quality metrics.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::ontologies::{
    BoundaryDetector, NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor,
};
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;

pub struct SleepOntology;

impl OntologyDescriptor for SleepOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("health_sleep")
            .display_name("Sleep Sessions")
            .description("Sleep analysis from HealthKit with quality metrics")
            .domain("health")
            .table_name("health_sleep")
            .source_streams(vec!["stream_ios_healthkit"])
            .narrative_role(NarrativeRole::Structure)
            // Interval detection - sleep has pre-defined start/end
            .interval_boundaries(
                "start_time",
                "end_time",
                vec![], // No additional filters
                0.95,   // High fidelity - physical measurement
                90,     // High weight - sleep is major daily structure
                vec!["sleep_quality_score", "total_duration_minutes"],
            )
            .build()
    }
}

#[async_trait]
impl BoundaryDetector for SleepOntology {
    fn ontology_name(&self) -> &'static str {
        "health_sleep"
    }

    async fn detect_boundaries(
        &self,
        db: &Database,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<BoundaryCandidate>> {
        let sessions = sqlx::query!(
            r#"
            SELECT start_time, end_time, source_stream_id
            FROM data.health_sleep
            WHERE start_time < $2 AND end_time > $1
            ORDER BY start_time ASC
            "#,
            start_time,
            end_time
        )
        .fetch_all(db.pool())
        .await?;

        let mut boundaries = Vec::new();

        for session in &sessions {
            // Sleep start boundary
            boundaries.push(BoundaryCandidate {
                timestamp: session.start_time,
                boundary_type: BoundaryType::Begin,
                source_ontology: "health_sleep".to_string(),
                fidelity: 0.95, // High fidelity - HealthKit sleep data is reliable
                weight: 90,     // Strong structural marker
                metadata: serde_json::json!({
                    "type": "sleep_start",
                    "source_stream_id": session.source_stream_id
                }),
            });

            // Sleep end boundary (wake up)
            boundaries.push(BoundaryCandidate {
                timestamp: session.end_time,
                boundary_type: BoundaryType::End,
                source_ontology: "health_sleep".to_string(),
                fidelity: 0.95,
                weight: 90,
                metadata: serde_json::json!({
                    "type": "wake_up",
                    "source_stream_id": session.source_stream_id
                }),
            });
        }

        tracing::debug!(
            "Sleep detector: Found {} sessions, {} boundaries",
            sessions.len(),
            boundaries.len()
        );

        Ok(boundaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_ontology() {
        let ont = SleepOntology::descriptor();
        assert_eq!(ont.name, "health_sleep");
        assert_eq!(ont.domain, "health");
        assert_eq!(ont.narrative_role, NarrativeRole::Structure);
        assert_eq!(ont.boundary.fidelity, 0.95);
    }
}
