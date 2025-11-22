//! Calendar ontology
//!
//! Calendar events from Google Calendar and other providers.
//! Provides semantic structure (WHY) for narrative synthesis.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::ontologies::{
    BoundaryDetector, NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor,
};
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;

pub struct CalendarOntology;

impl OntologyDescriptor for CalendarOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("praxis_calendar")
            .display_name("Calendar Events")
            .description("Scheduled events from Google Calendar")
            .domain("praxis")
            .table_name("praxis_calendar")
            .source_streams(vec!["stream_google_calendar"])
            .narrative_role(NarrativeRole::Structure)
            // Interval detection - calendar has pre-defined start/end
            .interval_boundaries(
                "start_time",
                "end_time",
                vec![], // No filters - all calendar events are relevant
                0.70,   // Lower fidelity - people deviate from calendar
                80,     // High weight - defines intentional scenes
                vec!["title", "calendar_name", "location_name"],
            )
            // Substance queries for WHO and WHY extraction
            .substance_query(
                r#"
                SELECT
                    title,
                    attendee_person_ids
                FROM data.praxis_calendar
                WHERE start_time <= $2
                  AND end_time >= $1
                ORDER BY (end_time - start_time) DESC
                LIMIT 1
                "#,
                vec!["who", "why"],
                true, // Primary source for WHO and WHY
            )
            .build()
    }
}

#[async_trait]
impl BoundaryDetector for CalendarOntology {
    fn ontology_name(&self) -> &'static str {
        "praxis_calendar"
    }

    async fn detect_boundaries(
        &self,
        db: &Database,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<BoundaryCandidate>> {
        let events = sqlx::query!(
            r#"
            SELECT start_time, end_time, title, calendar_name
            FROM data.praxis_calendar
            WHERE start_time < $2 AND end_time > $1
            ORDER BY start_time ASC
            "#,
            start_time,
            end_time
        )
        .fetch_all(db.pool())
        .await?;

        let mut boundaries = Vec::new();

        for event in &events {
            // Event start boundary
            boundaries.push(BoundaryCandidate {
                timestamp: event.start_time,
                boundary_type: BoundaryType::Begin,
                source_ontology: "praxis_calendar".to_string(),
                fidelity: 0.70, // Moderate fidelity - people often deviate from calendar plans
                weight: 80,     // Semantic structure - set by registry
                metadata: serde_json::json!({
                    "type": "event_start",
                    "title": event.title,
                    "calendar": event.calendar_name
                }),
            });

            // Event end boundary
            boundaries.push(BoundaryCandidate {
                timestamp: event.end_time,
                boundary_type: BoundaryType::End,
                source_ontology: "praxis_calendar".to_string(),
                fidelity: 0.70,
                weight: 80,
                metadata: serde_json::json!({
                    "type": "event_end",
                    "title": event.title,
                    "calendar": event.calendar_name
                }),
            });
        }

        tracing::debug!(
            "Calendar detector: Found {} events, {} boundaries",
            events.len(),
            boundaries.len()
        );

        Ok(boundaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_ontology() {
        let ont = CalendarOntology::descriptor();
        assert_eq!(ont.name, "praxis_calendar");
        assert_eq!(ont.narrative_role, NarrativeRole::Structure);
        assert_eq!(ont.boundary.fidelity, 0.70);
        assert_eq!(ont.boundary.weight, 80);
    }
}
