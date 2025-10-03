//! Notion pages processing logic

use serde_json::json;

use crate::{
    error::Result,
    sources::SourceRecord,
};

use crate::sources::notion::types::{Page, Parent};

/// Pages processor
pub struct PagesProcessor {
    source_name: String,
}

impl PagesProcessor {
    /// Create a new pages processor
    pub fn new(source_name: &str) -> Self {
        Self {
            source_name: source_name.to_string(),
        }
    }

    /// Process pages into source records
    pub fn process_pages(&self, pages: Vec<Page>) -> Result<Vec<SourceRecord>> {
        let mut records = Vec::with_capacity(pages.len());

        for page in pages {
            records.push(self.page_to_record(page));
        }

        Ok(records)
    }

    /// Convert page to source record
    fn page_to_record(&self, page: Page) -> SourceRecord {
        let parent_info = match &page.parent {
            Parent::Database { database_id } => json!({
                "type": "database",
                "id": database_id
            }),
            Parent::Page { page_id } => json!({
                "type": "page",
                "id": page_id
            }),
            Parent::Workspace { workspace } => json!({
                "type": "workspace",
                "workspace": workspace
            }),
        };

        SourceRecord {
            id: page.id.clone(),
            source: self.source_name.clone(),
            timestamp: page.last_edited_time,
            data: json!({
                "id": page.id,
                "created_time": page.created_time,
                "last_edited_time": page.last_edited_time,
                "archived": page.archived,
                "properties": page.properties,
                "url": page.url,
                "parent": parent_info,
            }),
            metadata: Some(json!({
                "created_by": page.created_by.id,
                "last_edited_by": page.last_edited_by.id,
            })),
        }
    }

    /// Filter archived pages
    pub fn filter_archived(&self, pages: Vec<Page>, include_archived: bool) -> Vec<Page> {
        if include_archived {
            pages
        } else {
            pages.into_iter()
                .filter(|p| !p.archived)
                .collect()
        }
    }
}