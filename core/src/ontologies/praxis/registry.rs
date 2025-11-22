//! Praxis domain registry

use crate::ontologies::registry::OntologyRegistry;
use crate::ontologies::OntologyDescriptor;

use super::calendar::CalendarOntology;

/// Register all praxis ontologies
pub fn register_praxis_ontologies(registry: &mut OntologyRegistry) {
    // Register ontology descriptors
    registry.register(CalendarOntology::descriptor());

    // Register boundary detectors
    registry.register_detector(Box::new(CalendarOntology));
}
