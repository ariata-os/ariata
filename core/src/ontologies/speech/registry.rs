//! Speech domain registry

use crate::ontologies::registry::OntologyRegistry;
use crate::ontologies::OntologyDescriptor;

use super::transcription::TranscriptionOntology;

/// Register all speech ontologies
pub fn register_speech_ontologies(registry: &mut OntologyRegistry) {
    registry.register(TranscriptionOntology::descriptor());
}
