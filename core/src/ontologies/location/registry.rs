//! Location domain registry

use crate::ontologies::registry::OntologyRegistry;
use crate::ontologies::OntologyDescriptor;

use super::point::LocationPointOntology;
use super::visit::LocationVisitOntology;

/// Register all location ontologies
pub fn register_location_ontologies(registry: &mut OntologyRegistry) {
    // Register ontology descriptors
    registry.register(LocationPointOntology::descriptor());
    registry.register(LocationVisitOntology::descriptor());

    // Register boundary detectors
    registry.register_detector(Box::new(LocationVisitOntology));
}
