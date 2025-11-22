//! App usage ontology
//!
//! Application focus events from macOS.
//! Provides digital substance (WHAT/HOW) for narrative synthesis.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::ontologies::{
    BoundaryDetector, NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor,
};
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;

pub struct AppUsageOntology;

impl OntologyDescriptor for AppUsageOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("activity_app_usage")
            .display_name("App Usage")
            .description("Application focus events from macOS")
            .domain("activity")
            .table_name("activity_app_usage")
            .source_streams(vec!["stream_mac_apps"])
            .narrative_role(NarrativeRole::Substance)
            // Discrete detection - group app switches into sessions
            .discrete_boundaries(
                "start_time",
                Some("EXTRACT(EPOCH FROM (end_time - start_time))"),
                1,  // 1 minute gap = new session (app switches are rapid)
                30, // Minimum 30 seconds
                0.90,
                60, // Priority ontology - digital substance
                vec!["app_name", "app_category", "window_title"],
            )
            // Substance query for WHAT extraction
            .substance_query(
                r#"
                SELECT
                    app_name,
                    app_category,
                    window_title,
                    start_time,
                    end_time
                FROM data.activity_app_usage
                WHERE start_time <= $2
                  AND end_time >= $1
                ORDER BY (end_time - start_time) DESC
                "#,
                vec!["what"],
                true, // Primary source for WHAT
            )
            .build()
    }
}

#[async_trait]
impl BoundaryDetector for AppUsageOntology {
    fn ontology_name(&self) -> &'static str {
        "activity_app_usage"
    }

    async fn detect_boundaries(
        &self,
        db: &Database,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<BoundaryCandidate>> {
        // Detect session boundaries from app usage events
        // A session ends when there's a >1 minute gap between app focus events
        let events = sqlx::query!(
            r#"
            WITH ordered_events AS (
                SELECT
                    start_time,
                    end_time,
                    app_name,
                    app_category,
                    LAG(end_time) OVER (ORDER BY start_time) as prev_end_time
                FROM data.activity_app_usage
                WHERE start_time < $2 AND end_time > $1
                ORDER BY start_time ASC
            ),
            session_starts AS (
                SELECT start_time, app_name, app_category
                FROM ordered_events
                WHERE prev_end_time IS NULL
                   OR EXTRACT(EPOCH FROM (start_time - prev_end_time)) > 60
            )
            SELECT start_time, app_name, app_category
            FROM session_starts
            ORDER BY start_time ASC
            "#,
            start_time,
            end_time
        )
        .fetch_all(db.pool())
        .await?;

        let mut boundaries = Vec::new();

        for event in &events {
            // Session start boundary
            boundaries.push(BoundaryCandidate {
                timestamp: event.start_time,
                boundary_type: BoundaryType::Begin,
                source_ontology: "activity_app_usage".to_string(),
                fidelity: 0.90,
                weight: 60, // Digital substance
                metadata: serde_json::json!({
                    "type": "session_start",
                    "app_name": event.app_name,
                    "app_category": event.app_category
                }),
            });
        }

        tracing::debug!(
            "App usage detector: Found {} session boundaries",
            boundaries.len()
        );

        Ok(boundaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontologies::DetectionAlgorithm;

    #[test]
    fn test_app_usage_ontology() {
        let ont = AppUsageOntology::descriptor();
        assert_eq!(ont.name, "activity_app_usage");
        assert_eq!(ont.narrative_role, NarrativeRole::Substance);

        // Verify discrete detection is configured
        match &ont.boundary.algorithm {
            DetectionAlgorithm::Discrete { gap_minutes, .. } => {
                assert_eq!(*gap_minutes, 1);
            }
            _ => panic!("Expected Discrete detection algorithm"),
        }
    }
}
