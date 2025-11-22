//! Location point ontology
//!
//! Raw GPS coordinates from iOS location services.
//! This is the source data that gets clustered into visits.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct LocationPointOntology;

impl OntologyDescriptor for LocationPointOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("location_point")
            .display_name("Location Points")
            .description("Raw GPS coordinates from device location services")
            .domain("location")
            .table_name("location_point")
            .source_streams(vec!["stream_ios_location"])
            .narrative_role(NarrativeRole::Container)
            // Continuous changepoint detection on speed
            // Lower weight (65) since these are high-frequency signals that
            // shouldn't dominate calendar/location-visit boundaries in aggregation
            .continuous_boundaries(
                "speed_meters_per_second", // column
                1.0,                        // PELT penalty
                5,                          // min segment minutes
                0.90,                       // fidelity
                65,                         // weight
                vec!["accuracy_meters"],    // metadata fields
            )
            .build()
    }
}
