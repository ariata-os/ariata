//! Ontology descriptor types and builder
//!
//! Defines the unified configuration for each ontology table, consolidating:
//! - Table metadata (name, domain)
//! - Boundary detection configuration (previously in timeline/boundaries/registry.rs)
//! - Narrative role (previously in timeline/synthesis/evidence.rs)
//! - Substance extraction queries (previously hardcoded in timeline/synthesis/substance.rs)

use serde::{Deserialize, Serialize};

/// Role of an ontology in narrative construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NarrativeRole {
    /// Container: The physical/spatial context (WHERE)
    /// Examples: location_visit
    Container,

    /// Structure: The semantic/intentional framework (WHY)
    /// Examples: praxis_calendar, health_sleep
    Structure,

    /// Substance: The digital/physical actions (WHAT/HOW)
    /// Examples: activity_app_usage, browser_activity, voice_activity
    Substance,
}

impl NarrativeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            NarrativeRole::Container => "container",
            NarrativeRole::Structure => "structure",
            NarrativeRole::Substance => "substance",
        }
    }
}

/// Detection algorithm for boundary detection
///
/// Determines how event boundaries are extracted from ontology data.
#[derive(Debug, Clone)]
pub enum DetectionAlgorithm {
    /// Continuous numerical time series (heart rate, HRV, audio dB)
    /// Uses PELT changepoint detection for statistical mean/variance shifts
    Continuous {
        /// Column containing the numerical value
        column: &'static str,
        /// PELT penalty parameter (higher = fewer changepoints)
        /// Typical range: 1.0-5.0
        penalty: f64,
        /// Minimum segment duration in minutes
        min_segment_minutes: i64,
    },

    /// Discrete event stream (app switches, page visits, messages)
    /// Uses temporal gap clustering to group events into sessions
    Discrete {
        /// Column containing the event timestamp
        timestamp_col: &'static str,
        /// Optional duration column (can be SQL expression)
        duration_col: Option<&'static str>,
        /// Gap threshold in minutes - inactivity > N min = new session
        gap_minutes: i64,
        /// Minimum session duration in seconds
        min_duration_seconds: i64,
    },

    /// Pre-defined temporal intervals (calendar events, sleep sessions)
    /// Direct extraction of existing begin/end boundaries
    Interval {
        /// Column containing start timestamp
        start_col: &'static str,
        /// Column containing end timestamp
        end_col: &'static str,
        /// SQL WHERE clause filters: (column, condition)
        filters: Vec<(&'static str, &'static str)>,
    },

    /// No boundary detection for this ontology
    /// Used for derived/lookup tables that don't produce boundaries
    None,
}

/// Boundary detection configuration
#[derive(Debug, Clone)]
pub struct BoundaryConfig {
    /// Detection algorithm based on data characteristics
    pub algorithm: DetectionAlgorithm,

    /// Confidence score (0.0-1.0)
    ///
    /// Tier 1 (physical reality): 0.90-0.95
    /// - Cannot be faked (GPS, microphone, heart rate)
    ///
    /// Tier 2 (digital signals): 0.80-0.89
    /// - Measurable but indirectly inferred (browser activity)
    ///
    /// Tier 3 (declared intent): 0.70-0.79
    /// - User-declared plans (calendar, tasks)
    pub fidelity: f64,

    /// Significance weight for boundary aggregation (0-100)
    ///
    /// Higher weights create stronger narrative boundaries:
    /// - Location (100): Place changes are always significant
    /// - Calendar (80): Defines intentional scenes
    /// - App usage (60): Category changes signal cognitive shifts
    pub weight: i32,

    /// Metadata fields to extract from source table
    pub metadata_fields: Vec<&'static str>,
}

impl Default for BoundaryConfig {
    fn default() -> Self {
        Self {
            algorithm: DetectionAlgorithm::None,
            fidelity: 0.5,
            weight: 50,
            metadata_fields: vec![],
        }
    }
}

/// Substance extraction query configuration
///
/// Defines how to extract narrative substance (who/where/what/how/why)
/// from this ontology during narrative synthesis.
#[derive(Debug, Clone, Default)]
pub struct SubstanceQuery {
    /// SQL query template for extracting substance
    /// Use $1 for start_time, $2 for end_time
    pub query: Option<&'static str>,

    /// Which narrative dimension this provides (where, who, what, how, why)
    pub provides: Vec<&'static str>,

    /// Whether this is the primary source for its dimension
    pub is_primary: bool,
}

/// A registered ontology definition
#[derive(Debug, Clone)]
pub struct Ontology {
    /// Unique ontology name (e.g., "health_sleep", "praxis_calendar")
    pub name: &'static str,

    /// Human-readable display name
    pub display_name: &'static str,

    /// Description of what this ontology stores
    pub description: &'static str,

    /// Domain grouping (e.g., "health", "location", "social")
    pub domain: &'static str,

    /// Database table name (in data schema)
    pub table_name: &'static str,

    /// Source streams that feed into this ontology
    pub source_streams: Vec<&'static str>,

    /// Role in narrative construction
    pub narrative_role: NarrativeRole,

    /// Boundary detection configuration
    pub boundary: BoundaryConfig,

    /// Substance extraction configuration
    pub substance: SubstanceQuery,
}

/// Builder for Ontology
pub struct OntologyBuilder {
    name: &'static str,
    display_name: &'static str,
    description: &'static str,
    domain: &'static str,
    table_name: &'static str,
    source_streams: Vec<&'static str>,
    narrative_role: NarrativeRole,
    boundary: BoundaryConfig,
    substance: SubstanceQuery,
}

impl OntologyBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            display_name: name,
            description: "",
            domain: "",
            table_name: name,
            source_streams: vec![],
            narrative_role: NarrativeRole::Substance,
            boundary: BoundaryConfig::default(),
            substance: SubstanceQuery::default(),
        }
    }

    pub fn display_name(mut self, name: &'static str) -> Self {
        self.display_name = name;
        self
    }

    pub fn description(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    pub fn domain(mut self, domain: &'static str) -> Self {
        self.domain = domain;
        self
    }

    pub fn table_name(mut self, name: &'static str) -> Self {
        self.table_name = name;
        self
    }

    pub fn source_streams(mut self, streams: Vec<&'static str>) -> Self {
        self.source_streams = streams;
        self
    }

    pub fn narrative_role(mut self, role: NarrativeRole) -> Self {
        self.narrative_role = role;
        self
    }

    /// Configure boundary detection with discrete gap clustering
    pub fn discrete_boundaries(
        mut self,
        timestamp_col: &'static str,
        duration_col: Option<&'static str>,
        gap_minutes: i64,
        min_duration_seconds: i64,
        fidelity: f64,
        weight: i32,
        metadata_fields: Vec<&'static str>,
    ) -> Self {
        self.boundary = BoundaryConfig {
            algorithm: DetectionAlgorithm::Discrete {
                timestamp_col,
                duration_col,
                gap_minutes,
                min_duration_seconds,
            },
            fidelity,
            weight,
            metadata_fields,
        };
        self
    }

    /// Configure boundary detection with interval extraction
    pub fn interval_boundaries(
        mut self,
        start_col: &'static str,
        end_col: &'static str,
        filters: Vec<(&'static str, &'static str)>,
        fidelity: f64,
        weight: i32,
        metadata_fields: Vec<&'static str>,
    ) -> Self {
        self.boundary = BoundaryConfig {
            algorithm: DetectionAlgorithm::Interval {
                start_col,
                end_col,
                filters,
            },
            fidelity,
            weight,
            metadata_fields,
        };
        self
    }

    /// Configure boundary detection with continuous changepoint detection
    pub fn continuous_boundaries(
        mut self,
        column: &'static str,
        penalty: f64,
        min_segment_minutes: i64,
        fidelity: f64,
        weight: i32,
        metadata_fields: Vec<&'static str>,
    ) -> Self {
        self.boundary = BoundaryConfig {
            algorithm: DetectionAlgorithm::Continuous {
                column,
                penalty,
                min_segment_minutes,
            },
            fidelity,
            weight,
            metadata_fields,
        };
        self
    }

    /// Configure no boundary detection
    pub fn no_boundaries(mut self) -> Self {
        self.boundary = BoundaryConfig {
            algorithm: DetectionAlgorithm::None,
            ..Default::default()
        };
        self
    }

    /// Configure substance extraction
    pub fn substance_query(
        mut self,
        query: &'static str,
        provides: Vec<&'static str>,
        is_primary: bool,
    ) -> Self {
        self.substance = SubstanceQuery {
            query: Some(query),
            provides,
            is_primary,
        };
        self
    }

    pub fn build(self) -> Ontology {
        Ontology {
            name: self.name,
            display_name: self.display_name,
            description: self.description,
            domain: self.domain,
            table_name: self.table_name,
            source_streams: self.source_streams,
            narrative_role: self.narrative_role,
            boundary: self.boundary,
            substance: self.substance,
        }
    }
}

/// Trait for ontology modules to implement
pub trait OntologyDescriptor {
    /// Get the ontology definition
    fn descriptor() -> Ontology;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontology_builder() {
        let ontology = OntologyBuilder::new("health_sleep")
            .display_name("Sleep Sessions")
            .description("Sleep tracking from HealthKit")
            .domain("health")
            .table_name("health_sleep")
            .source_streams(vec!["stream_ios_healthkit"])
            .narrative_role(NarrativeRole::Structure)
            .interval_boundaries(
                "start_time",
                "end_time",
                vec![],
                0.95,
                90,
                vec!["sleep_quality_score", "total_duration_minutes"],
            )
            .build();

        assert_eq!(ontology.name, "health_sleep");
        assert_eq!(ontology.domain, "health");
        assert_eq!(ontology.narrative_role, NarrativeRole::Structure);
        assert_eq!(ontology.boundary.fidelity, 0.95);
        assert_eq!(ontology.boundary.weight, 90);
    }

    #[test]
    fn test_narrative_role_as_str() {
        assert_eq!(NarrativeRole::Container.as_str(), "container");
        assert_eq!(NarrativeRole::Structure.as_str(), "structure");
        assert_eq!(NarrativeRole::Substance.as_str(), "substance");
    }
}
