//! Message ontology
//!
//! SMS and iMessage data from macOS.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct MessageOntology;

impl OntologyDescriptor for MessageOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("social_message")
            .display_name("Messages")
            .description("SMS and iMessage conversations")
            .domain("social")
            .table_name("social_message")
            .source_streams(vec!["stream_mac_imessage"])
            .narrative_role(NarrativeRole::Substance)
            // Could enable discrete detection for conversation sessions
            .no_boundaries()
            .build()
    }
}
