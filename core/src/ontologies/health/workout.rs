//! Workout ontology
//!
//! Workout sessions from HealthKit with activity type and duration.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct WorkoutOntology;

impl OntologyDescriptor for WorkoutOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("health_workout")
            .display_name("Workouts")
            .description("Workout sessions from HealthKit")
            .domain("health")
            .table_name("health_workout")
            .source_streams(vec!["stream_ios_healthkit"])
            .narrative_role(NarrativeRole::Substance)
            // Could use interval detection for workout sessions
            .no_boundaries() // TODO: Enable when needed
            .build()
    }
}
