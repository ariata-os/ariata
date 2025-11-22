use crate::database::Database;
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Type of boundary detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BoundaryType {
    /// Start of an activity/state
    Begin,
    /// End of an activity/state
    End,
}

impl std::fmt::Display for BoundaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundaryType::Begin => write!(f, "begin"),
            BoundaryType::End => write!(f, "end"),
        }
    }
}

/// A detected event boundary - a temporal marker where something changed
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventBoundary {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub boundary_type: String, // "begin" or "end"
    pub source_ontology: String, // "sleep", "location_visit", "praxis_calendar", etc.
    pub fidelity: f64, // 0.0-1.0 confidence score
    pub weight: i32, // Significance weight for aggregation
    pub is_primary: Option<bool>, // Marked during aggregation as strongest boundary
    pub metadata: serde_json::Value, // Source-specific data
    pub created_at: DateTime<Utc>,
}

impl EventBoundary {
    /// Create a new event boundary in the database
    pub async fn create(
        db: &Database,
        timestamp: DateTime<Utc>,
        boundary_type: BoundaryType,
        source_ontology: &str,
        fidelity: f64,
        weight: i32,
        metadata: serde_json::Value,
    ) -> Result<Self> {
        let boundary = sqlx::query_as!(
            EventBoundary,
            r#"
            INSERT INTO data.event_boundaries
            (timestamp, boundary_type, source_ontology, fidelity, weight, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (timestamp, source_ontology, boundary_type)
            DO UPDATE SET
                fidelity = EXCLUDED.fidelity,
                weight = EXCLUDED.weight,
                metadata = EXCLUDED.metadata
            RETURNING *
            "#,
            timestamp,
            boundary_type.to_string(),
            source_ontology,
            fidelity,
            weight,
            metadata
        )
        .fetch_one(db.pool())
        .await?;

        Ok(boundary)
    }

    /// Find boundaries within a time range
    pub async fn find_in_range(
        db: &Database,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Self>> {
        let boundaries = sqlx::query_as!(
            EventBoundary,
            r#"
            SELECT * FROM data.event_boundaries
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
            start,
            end
        )
        .fetch_all(db.pool())
        .await?;

        Ok(boundaries)
    }

    /// Find the most recent boundary before a given time
    pub async fn find_latest_before(
        db: &Database,
        before: DateTime<Utc>,
    ) -> Result<Option<Self>> {
        let boundary = sqlx::query_as!(
            EventBoundary,
            r#"
            SELECT * FROM data.event_boundaries
            WHERE timestamp < $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            before
        )
        .fetch_optional(db.pool())
        .await?;

        Ok(boundary)
    }
}
