//! Activity domain registry

use crate::ontologies::registry::OntologyRegistry;
use crate::ontologies::OntologyDescriptor;

use super::app_usage::AppUsageOntology;
use super::web_browsing::WebBrowsingOntology;

/// Register all activity ontologies
pub fn register_activity_ontologies(registry: &mut OntologyRegistry) {
    // Register ontology descriptors
    registry.register(AppUsageOntology::descriptor());
    registry.register(WebBrowsingOntology::descriptor());

    // Register boundary detectors
    registry.register_detector(Box::new(AppUsageOntology));
    registry.register_detector(Box::new(WebBrowsingOntology));
}
