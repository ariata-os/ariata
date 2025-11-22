//! Document ontology
//!
//! Documents from Notion and other knowledge management tools.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct DocumentOntology;

impl OntologyDescriptor for DocumentOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("knowledge_document")
            .display_name("Documents")
            .description("Pages from Notion and other document sources")
            .domain("knowledge")
            .table_name("knowledge_document")
            .source_streams(vec!["stream_notion_pages"])
            .narrative_role(NarrativeRole::Substance)
            .no_boundaries()
            .build()
    }
}
