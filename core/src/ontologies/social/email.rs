//! Email ontology
//!
//! Email messages from Gmail and other providers.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct EmailOntology;

impl OntologyDescriptor for EmailOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("social_email")
            .display_name("Email")
            .description("Email messages from Gmail and other providers")
            .domain("social")
            .table_name("social_email")
            .source_streams(vec!["stream_google_gmail"])
            .narrative_role(NarrativeRole::Substance)
            // Email doesn't produce boundaries currently
            // Could enable discrete detection for email sessions
            .no_boundaries()
            .build()
    }
}
