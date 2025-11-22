//! Boundary detector trait for ontologies
//!
//! Each ontology that participates in narrative timeline construction
//! implements this trait to detect event boundaries from its data.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::timeline::boundaries::BoundaryCandidate;
use crate::Result;

/// Trait for detecting event boundaries from ontology data
///
/// Each ontology implements this trait with its specific detection logic.
/// The detection can use generic algorithms (interval, continuous, discrete)
/// as helpers, but owns the full detection logic including:
/// - Database queries
/// - Metadata extraction
/// - Fidelity and weight assignment
///
/// # Example
///
/// ```ignore
/// use async_trait::async_trait;
///
/// pub struct SleepOntology;
///
/// #[async_trait]
/// impl BoundaryDetector for SleepOntology {
///     fn ontology_name(&self) -> &'static str {
///         "health_sleep"
///     }
///
///     async fn detect_boundaries(
///         &self,
///         db: &Database,
///         start_time: DateTime<Utc>,
///         end_time: DateTime<Utc>,
///     ) -> Result<Vec<BoundaryCandidate>> {
///         // Custom detection logic
///     }
/// }
/// ```
#[async_trait]
pub trait BoundaryDetector: Send + Sync {
    /// The ontology name this detector is for
    fn ontology_name(&self) -> &'static str;

    /// Detect event boundaries within the given time window
    ///
    /// Returns a list of boundary candidates with:
    /// - Timestamp of the boundary
    /// - Type (begin/end)
    /// - Source ontology name
    /// - Fidelity score (0.0-1.0)
    /// - Weight for aggregation (0-100)
    /// - Metadata specific to this ontology
    async fn detect_boundaries(
        &self,
        db: &Database,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<BoundaryCandidate>>;
}
