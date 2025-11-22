//! Social domain registry

use crate::ontologies::registry::OntologyRegistry;
use crate::ontologies::OntologyDescriptor;

use super::email::EmailOntology;
use super::message::MessageOntology;

/// Register all social ontologies
pub fn register_social_ontologies(registry: &mut OntologyRegistry) {
    registry.register(EmailOntology::descriptor());
    registry.register(MessageOntology::descriptor());
}
