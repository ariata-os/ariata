//! Knowledge domain registry

use crate::ontologies::registry::OntologyRegistry;
use crate::ontologies::OntologyDescriptor;

use super::ai_conversation::AiConversationOntology;
use super::document::DocumentOntology;

/// Register all knowledge ontologies
pub fn register_knowledge_ontologies(registry: &mut OntologyRegistry) {
    registry.register(AiConversationOntology::descriptor());
    registry.register(DocumentOntology::descriptor());
}
