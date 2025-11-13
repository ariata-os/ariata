//! Notion pages stream implementation

pub mod transform;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    error::Result,
    sources::{
        auth::SourceAuth,
        base::{SyncMode, SyncResult},
        stream::Stream,
    },
    storage::stream_writer::StreamWriter,
};

use super::{client::NotionApiClient, types::{SearchResponse, Page, Parent, BlockChildrenResponse, Block}};

/// Notion pages stream
pub struct NotionPagesStream {
    source_id: Uuid,
    client: NotionApiClient,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl NotionPagesStream {
    /// Create a new Notion pages stream with SourceAuth and StreamWriter
    pub fn new(source_id: Uuid, _db: PgPool, stream_writer: Arc<Mutex<StreamWriter>>, auth: SourceAuth) -> Self {
        // Extract token manager from auth
        let token_manager = auth
            .token_manager()
            .expect("NotionPagesStream requires OAuth2 auth")
            .clone();

        let client = NotionApiClient::new(source_id, token_manager);

        Self {
            source_id,
            client,
            stream_writer,
        }
    }

    /// Sync with explicit mode
    #[tracing::instrument(skip(self), fields(source_id = %self.source_id))]
    pub async fn sync_with_mode(&self, mode: &SyncMode) -> Result<SyncResult> {
        // Notion pages only support full refresh, not incremental sync
        // If incremental mode is requested, log a warning and use full refresh instead
        let _effective_mode = match mode {
            SyncMode::Incremental { .. } => {
                tracing::warn!(
                    "Notion pages stream does not support incremental sync. Using full refresh instead."
                );
                SyncMode::FullRefresh
            }
            SyncMode::FullRefresh => SyncMode::FullRefresh,
        };

        let started_at = Utc::now();

        tracing::info!("Starting Notion pages sync");

        let mut all_pages = Vec::new();
        let mut cursor = None;
        let mut records_fetched = 0;

        // Paginate through all pages
        loop {
            let response = self.search_pages(cursor).await?;
            records_fetched += response.results.len();

            // Write pages to stream_notion_pages table
            for page in &response.results {
                match self.upsert_page(page).await {
                    Ok(_) => {
                        all_pages.push(page.clone());
                    }
                    Err(e) => {
                        tracing::warn!(
                            page_id = %page.id,
                            error = %e,
                            "Failed to write page to database"
                        );
                        // Continue with other pages even if one fails
                    }
                }
            }

            if !response.has_more {
                break;
            }

            cursor = response.next_cursor;
        }

        let records_written = all_pages.len();
        let completed_at = Utc::now();
        let result = SyncResult {
            records_fetched,
            records_written,
            records_failed: 0,
            next_cursor: None,
            started_at,
            completed_at,
            records: None, // Notion uses database, not direct transform
            archive_job_id: None,
        };

        // Logging is handled by job executor
        Ok(result)
    }

    /// Search for pages
    async fn search_pages(&self, cursor: Option<String>) -> Result<SearchResponse> {
        let mut body = json!({
            "filter": {
                "property": "object",
                "value": "page"
            },
            "page_size": 100,
        });

        if let Some(cursor) = cursor {
            body["start_cursor"] = json!(cursor);
        }

        self.client.post_json("search", &body).await
    }

    /// Fetch all blocks for a page (with pagination)
    async fn fetch_page_blocks(&self, page_id: &str) -> Result<Vec<Block>> {
        let mut all_blocks = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let path = if let Some(ref cursor_val) = cursor {
                format!("blocks/{}/children?page_size=100&start_cursor={}", page_id, cursor_val)
            } else {
                format!("blocks/{}/children?page_size=100", page_id)
            };

            let response: BlockChildrenResponse = self.client.get(&path).await?;

            all_blocks.extend(response.results);

            if !response.has_more {
                break;
            }

            cursor = response.next_cursor;
        }

        Ok(all_blocks)
    }

    /// Convert blocks to markdown text
    fn blocks_to_markdown(&self, blocks: &[Block]) -> String {
        let mut markdown = String::new();

        for block in blocks {
            match block.block_type.as_str() {
                "paragraph" => {
                    if let Some(content) = &block.paragraph {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str(&text);
                            markdown.push_str("\n\n");
                        }
                    }
                }
                "heading_1" => {
                    if let Some(content) = &block.heading_1 {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("# ");
                            markdown.push_str(&text);
                            markdown.push_str("\n\n");
                        }
                    }
                }
                "heading_2" => {
                    if let Some(content) = &block.heading_2 {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("## ");
                            markdown.push_str(&text);
                            markdown.push_str("\n\n");
                        }
                    }
                }
                "heading_3" => {
                    if let Some(content) = &block.heading_3 {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("### ");
                            markdown.push_str(&text);
                            markdown.push_str("\n\n");
                        }
                    }
                }
                "bulleted_list_item" => {
                    if let Some(content) = &block.bulleted_list_item {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("- ");
                            markdown.push_str(&text);
                            markdown.push('\n');
                        }
                    }
                }
                "numbered_list_item" => {
                    if let Some(content) = &block.numbered_list_item {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("1. ");
                            markdown.push_str(&text);
                            markdown.push('\n');
                        }
                    }
                }
                "to_do" => {
                    if let Some(content) = &block.to_do {
                        let text = self.rich_text_to_string(&content.rich_text);
                        let checkbox = if content.checked.unwrap_or(false) { "[x]" } else { "[ ]" };
                        if !text.is_empty() {
                            markdown.push_str("- ");
                            markdown.push_str(checkbox);
                            markdown.push(' ');
                            markdown.push_str(&text);
                            markdown.push('\n');
                        }
                    }
                }
                "code" => {
                    if let Some(content) = &block.code {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("```");
                            markdown.push_str(&content.language);
                            markdown.push('\n');
                            markdown.push_str(&text);
                            markdown.push_str("\n```\n\n");
                        }
                    }
                }
                "quote" => {
                    if let Some(content) = &block.quote {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("> ");
                            markdown.push_str(&text);
                            markdown.push_str("\n\n");
                        }
                    }
                }
                "callout" => {
                    if let Some(content) = &block.callout {
                        let text = self.rich_text_to_string(&content.rich_text);
                        if !text.is_empty() {
                            markdown.push_str("> 💡 ");
                            markdown.push_str(&text);
                            markdown.push_str("\n\n");
                        }
                    }
                }
                "child_page" => {
                    if let Some(content) = &block.child_page {
                        markdown.push_str("📄 ");
                        markdown.push_str(&content.title);
                        markdown.push_str("\n\n");
                    }
                }
                _ => {
                    // Unsupported block types - just note them
                    tracing::debug!("Unsupported block type: {}", block.block_type);
                }
            }
        }

        markdown.trim().to_string()
    }

    /// Convert rich text array to plain string with basic formatting
    fn rich_text_to_string(&self, rich_text: &[super::types::RichText]) -> String {
        rich_text
            .iter()
            .map(|rt| {
                let mut text = rt.plain_text.clone();

                // Apply markdown formatting based on annotations
                if rt.annotations.bold {
                    text = format!("**{}**", text);
                }
                if rt.annotations.italic {
                    text = format!("*{}*", text);
                }
                if rt.annotations.code {
                    text = format!("`{}`", text);
                }
                if rt.annotations.strikethrough {
                    text = format!("~~{}~~", text);
                }

                // Add link if present
                if let Some(href) = &rt.href {
                    text = format!("[{}]({})", text, href);
                }

                text
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Upsert a page into the database
    async fn upsert_page(&self, page: &Page) -> Result<()> {
        // Extract parent information
        let (parent_type, parent_id) = match &page.parent {
            Parent::Database { database_id } => ("database", Some(database_id.clone())),
            Parent::Page { page_id } => ("page", Some(page_id.clone())),
            Parent::Workspace { .. } => ("workspace", None),
        };

        // Fetch page blocks (content)
        let blocks = match self.fetch_page_blocks(&page.id).await {
            Ok(blocks) => blocks,
            Err(e) => {
                tracing::warn!(
                    page_id = %page.id,
                    error = %e,
                    "Failed to fetch blocks for page, storing without content"
                );
                Vec::new()
            }
        };

        // Convert blocks to markdown
        let content_markdown = if !blocks.is_empty() {
            Some(self.blocks_to_markdown(&blocks))
        } else {
            None
        };

        // Build complete record with all parsed fields for storage
        let properties_json = serde_json::to_value(&page.properties)?;
        let content_blocks_json = if !blocks.is_empty() {
            Some(serde_json::to_value(&blocks)?)
        } else {
            None
        };

        let record = serde_json::json!({
            "page_id": page.id,
            "url": page.url,
            "created_time": page.created_time,
            "last_edited_time": page.last_edited_time,
            "created_by_id": page.created_by.id,
            "created_by_name": page.created_by.name,
            "last_edited_by_id": page.last_edited_by.id,
            "last_edited_by_name": page.last_edited_by.name,
            "parent_type": parent_type,
            "parent_id": parent_id,
            "archived": page.archived,
            "properties": properties_json,
            "content_markdown": content_markdown,
            "content_blocks": content_blocks_json,
            "raw_page": page,
            "synced_at": Utc::now(),
        });

        // Write to S3/object storage via StreamWriter
        {
            let mut writer = self.stream_writer.lock().await;
            writer.write_record(
                self.source_id,
                "pages",
                record,
                Some(page.last_edited_time),
            )?;
        }

        tracing::debug!(page_id = %page.id, "Wrote Notion page to object storage");
        Ok(())
    }
}

// Implement Stream trait
#[async_trait]
impl Stream for NotionPagesStream {
    async fn sync(&self, mode: SyncMode) -> Result<SyncResult> {
        self.sync_with_mode(&mode).await
    }

    fn table_name(&self) -> &str {
        "stream_notion_pages"
    }

    fn stream_name(&self) -> &str {
        "pages"
    }

    fn source_name(&self) -> &str {
        "notion"
    }

    async fn load_config(&mut self, _db: &PgPool, _source_id: Uuid) -> Result<()> {
        // Notion doesn't have stream-specific config yet
        Ok(())
    }

    fn supports_incremental(&self) -> bool {
        false // Notion API doesn't provide incremental sync
    }

    fn supports_full_refresh(&self) -> bool {
        true
    }
}
