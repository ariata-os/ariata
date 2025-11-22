//! Heart rate ontology
//!
//! Heart rate measurements from HealthKit.
//! Future: Could use continuous changepoint detection for zone transitions.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct HeartRateOntology;

impl OntologyDescriptor for HeartRateOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("health_heart_rate")
            .display_name("Heart Rate")
            .description("Heart rate measurements from HealthKit")
            .domain("health")
            .table_name("health_heart_rate")
            .source_streams(vec!["stream_ios_healthkit"])
            .narrative_role(NarrativeRole::Substance)
            // Future: Enable continuous detection for zone transitions
            // .continuous_boundaries(
            //     "bpm",
            //     3.0,  // PELT penalty
            //     5,    // min segment minutes
            //     0.95,
            //     40,
            //     vec!["bpm", "zone"],
            // )
            .no_boundaries() // Not yet implemented
            .build()
    }
}
