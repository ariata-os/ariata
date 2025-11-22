//! Web browsing ontology
//!
//! Browser history from macOS browsers.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::ontologies::{
    BoundaryDetector, NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor,
};
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;

pub struct WebBrowsingOntology;

impl OntologyDescriptor for WebBrowsingOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("activity_web_browsing")
            .display_name("Web Browsing")
            .description("Browser history from Safari and Chrome")
            .domain("activity")
            .table_name("activity_web_browsing")
            .source_streams(vec!["stream_mac_browser"])
            .narrative_role(NarrativeRole::Substance)
            // Discrete detection - group page visits into sessions
            .discrete_boundaries(
                "timestamp",
                Some("visit_duration_seconds"),
                2,  // 2 minute gap = new browsing session
                10, // Minimum 10 seconds
                0.85,
                50, // Lower weight - supplementary digital substance
                vec!["domain", "page_title"],
            )
            .build()
    }
}

#[async_trait]
impl BoundaryDetector for WebBrowsingOntology {
    fn ontology_name(&self) -> &'static str {
        "activity_web_browsing"
    }

    async fn detect_boundaries(
        &self,
        db: &Database,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<BoundaryCandidate>> {
        // Detect browsing session boundaries
        // A session ends when there's a >2 minute gap between page visits
        let events = sqlx::query!(
            r#"
            WITH ordered_visits AS (
                SELECT
                    timestamp,
                    domain,
                    page_title,
                    LAG(timestamp + (visit_duration_seconds || ' seconds')::interval)
                        OVER (ORDER BY timestamp) as prev_end_time
                FROM data.activity_web_browsing
                WHERE timestamp BETWEEN $1 AND $2
                ORDER BY timestamp ASC
            ),
            session_starts AS (
                SELECT timestamp, domain, page_title
                FROM ordered_visits
                WHERE prev_end_time IS NULL
                   OR EXTRACT(EPOCH FROM (timestamp - prev_end_time)) > 120
            )
            SELECT timestamp, domain, page_title
            FROM session_starts
            ORDER BY timestamp ASC
            "#,
            start_time,
            end_time
        )
        .fetch_all(db.pool())
        .await?;

        let mut boundaries = Vec::new();

        for event in &events {
            boundaries.push(BoundaryCandidate {
                timestamp: event.timestamp,
                boundary_type: BoundaryType::Begin,
                source_ontology: "activity_web_browsing".to_string(),
                fidelity: 0.85,
                weight: 50, // Lower weight - supplementary
                metadata: serde_json::json!({
                    "type": "browsing_session_start",
                    "domain": event.domain,
                    "page_title": event.page_title
                }),
            });
        }

        tracing::debug!(
            "Web browsing detector: Found {} session boundaries",
            boundaries.len()
        );

        Ok(boundaries)
    }
}
