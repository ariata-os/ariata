//! AI Conversation ontology
//!
//! Exported chat sessions from Ariata AI conversations.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct AiConversationOntology;

impl OntologyDescriptor for AiConversationOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("knowledge_ai_conversation")
            .display_name("AI Conversations")
            .description("Chat sessions from Ariata AI assistant")
            .domain("knowledge")
            .table_name("knowledge_ai_conversation")
            .source_streams(vec!["stream_ariata_ai_chat"])
            .narrative_role(NarrativeRole::Substance)
            .no_boundaries()
            .build()
    }
}
