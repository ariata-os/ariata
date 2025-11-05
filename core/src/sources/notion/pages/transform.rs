//! Notion pages to knowledge_document ontology transformation
//!
//! Transforms raw Notion pages from stream_notion_pages into the normalized
//! knowledge_document ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Transform Notion pages to knowledge_document ontology
pub struct NotionPageTransform;

#[async_trait]
impl OntologyTransform for NotionPageTransform {
    fn source_table(&self) -> &str {
        "stream_notion_pages"
    }

    fn target_table(&self) -> &str {
        "knowledge_document"
    }

    fn domain(&self) -> &str {
        "knowledge"
    }

    #[tracing::instrument(skip(self, db), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting Notion pages to knowledge_document transformation"
        );

        // Query stream_notion_pages for records not yet transformed
        // Use left join to find records that don't exist in knowledge_document
        let rows = sqlx::query(
            r#"
            SELECT
                p.id, p.page_id, p.url,
                p.created_time, p.last_edited_time,
                p.parent_type, p.parent_id,
                p.archived,
                p.properties,
                p.content_markdown,
                p.content_blocks
            FROM elt.stream_notion_pages p
            LEFT JOIN elt.knowledge_document d ON (d.source_stream_id = p.id)
            WHERE p.source_id = $1
              AND d.id IS NULL
              AND p.archived = false
            ORDER BY p.last_edited_time DESC
            LIMIT 1000
            "#,
        )
        .bind(source_id)
        .fetch_all(db.pool())
        .await?;

        tracing::debug!(
            records_to_transform = rows.len(),
            "Fetched untransformed Notion pages"
        );

        for row in rows {
            records_read += 1;

            // Extract fields from row
            let stream_id: Uuid = row.try_get("id")?;
            let page_id: String = row.try_get("page_id")?;
            let url: String = row.try_get("url")?;
            let created_time: DateTime<Utc> = row.try_get("created_time")?;
            let last_edited_time: DateTime<Utc> = row.try_get("last_edited_time")?;
            let parent_type: String = row.try_get("parent_type")?;
            let parent_id: Option<String> = row.try_get("parent_id")?;
            let properties: serde_json::Value = row.try_get("properties")?;
            let content_markdown: Option<String> = row.try_get("content_markdown")?;

            // Extract title from properties (check common title property names)
            let title = extract_title_from_properties(&properties)
                .or_else(|| {
                    // Fallback: extract first heading from content
                    content_markdown.as_ref().and_then(|c| extract_first_heading(c))
                })
                .unwrap_or_else(|| "Untitled".to_string());

            // Determine document type based on parent
            let document_type = match parent_type.as_str() {
                "database" => "notion_database_page",
                "page" => "notion_subpage",
                "workspace" => "notion_page",
                _ => "notion_page",
            };

            // Build metadata with Notion-specific fields
            let metadata = serde_json::json!({
                "notion_page_id": page_id,
                "notion_url": url,
                "parent_type": parent_type,
                "parent_id": parent_id,
                "properties": properties,
            });

            // Insert into knowledge_document
            let result = sqlx::query(
                r#"
                INSERT INTO elt.knowledge_document (
                    title, content, document_type,
                    external_id, external_url,
                    created_time, last_modified_time,
                    source_stream_id, source_table, source_provider,
                    metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
                )
                ON CONFLICT (source_stream_id) DO NOTHING
                "#,
            )
            .bind(&title) // title
            .bind(&content_markdown) // content
            .bind(document_type) // document_type
            .bind(&page_id) // external_id
            .bind(&url) // external_url
            .bind(created_time) // created_time
            .bind(last_edited_time) // last_modified_time
            .bind(stream_id) // source_stream_id
            .bind("stream_notion_pages") // source_table
            .bind("notion") // source_provider
            .bind(&metadata) // metadata
            .execute(db.pool())
            .await;

            match result {
                Ok(_) => {
                    records_written += 1;
                    last_processed_id = Some(stream_id);
                }
                Err(e) => {
                    tracing::warn!(
                        page_id = %page_id,
                        stream_id = %stream_id,
                        error = %e,
                        "Failed to transform Notion page"
                    );
                    records_failed += 1;
                }
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            "Notion pages to knowledge_document transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![], // Notion transform doesn't chain to other transforms
        })
    }
}

/// Extract title from Notion page properties
fn extract_title_from_properties(properties: &serde_json::Value) -> Option<String> {
    // Common title property names in Notion
    let title_keys = ["Name", "Title", "title", "name", "Page"];

    for key in &title_keys {
        if let Some(prop) = properties.get(key) {
            // Notion properties have different types - try to extract text
            if let Some(title_array) = prop.get("title") {
                if let Some(title_arr) = title_array.as_array() {
                    if let Some(first_title) = title_arr.first() {
                        if let Some(plain_text) = first_title.get("plain_text") {
                            if let Some(text) = plain_text.as_str() {
                                if !text.trim().is_empty() {
                                    return Some(text.to_string());
                                }
                            }
                        }
                    }
                }
            }

            // Also try rich_text type properties
            if let Some(rich_text_array) = prop.get("rich_text") {
                if let Some(rt_arr) = rich_text_array.as_array() {
                    if let Some(first_rt) = rt_arr.first() {
                        if let Some(plain_text) = first_rt.get("plain_text") {
                            if let Some(text) = plain_text.as_str() {
                                if !text.trim().is_empty() {
                                    return Some(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extract first heading from markdown content
fn extract_first_heading(markdown: &str) -> Option<String> {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            // Remove leading # and trim
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_metadata() {
        let transform = NotionPageTransform;
        assert_eq!(transform.source_table(), "stream_notion_pages");
        assert_eq!(transform.target_table(), "knowledge_document");
        assert_eq!(transform.domain(), "knowledge");
    }

    #[test]
    fn test_extract_first_heading() {
        let markdown = "Some intro text\n\n# Main Heading\n\nMore content";
        assert_eq!(
            extract_first_heading(markdown),
            Some("Main Heading".to_string())
        );

        let markdown_no_heading = "Just some text\nwithout headings";
        assert_eq!(extract_first_heading(markdown_no_heading), None);

        let markdown_h2 = "## Second Level\n\nContent";
        assert_eq!(
            extract_first_heading(markdown_h2),
            Some("Second Level".to_string())
        );
    }

    #[test]
    fn test_extract_title_from_properties() {
        // Test title property
        let props = serde_json::json!({
            "Title": {
                "title": [
                    {
                        "plain_text": "My Page Title"
                    }
                ]
            }
        });
        assert_eq!(
            extract_title_from_properties(&props),
            Some("My Page Title".to_string())
        );

        // Test Name property with rich_text
        let props = serde_json::json!({
            "Name": {
                "rich_text": [
                    {
                        "plain_text": "Project Name"
                    }
                ]
            }
        });
        assert_eq!(
            extract_title_from_properties(&props),
            Some("Project Name".to_string())
        );

        // Test empty properties
        let props = serde_json::json!({});
        assert_eq!(extract_title_from_properties(&props), None);
    }
}
