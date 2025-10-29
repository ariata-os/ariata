//! Notion pages stream implementation

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::Result,
    sources::{
        auth::SourceAuth,
        base::{SyncMode, SyncResult},
        stream::Stream,
    },
};

use super::{client::NotionApiClient, types::{SearchResponse, Page, Parent}};

/// Notion pages stream
pub struct NotionPagesStream {
    source_id: Uuid,
    client: NotionApiClient,
    db: PgPool,
}

impl NotionPagesStream {
    /// Create a new Notion pages stream with SourceAuth
    pub fn new(source_id: Uuid, db: PgPool, auth: SourceAuth) -> Self {
        // Extract token manager from auth
        let token_manager = auth
            .token_manager()
            .expect("NotionPagesStream requires OAuth2 auth")
            .clone();

        let client = NotionApiClient::new(source_id, token_manager);

        Self {
            source_id,
            client,
            db,
        }
    }

    /// Sync with explicit mode
    #[tracing::instrument(skip(self), fields(source_id = %self.source_id))]
    pub async fn sync_with_mode(&self, mode: &SyncMode) -> Result<SyncResult> {
        // Notion pages only support full refresh, not incremental sync
        // If incremental mode is requested, log a warning and use full refresh instead
        let effective_mode = match mode {
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

    /// Upsert a page into the database
    async fn upsert_page(&self, page: &Page) -> Result<()> {
        // Extract parent information
        let (parent_type, parent_id) = match &page.parent {
            Parent::Database { database_id } => ("database", Some(database_id.clone())),
            Parent::Page { page_id } => ("page", Some(page_id.clone())),
            Parent::Workspace { .. } => ("workspace", None),
        };

        // Serialize properties and full page as JSONB
        let properties_json = serde_json::to_value(&page.properties)?;
        let raw_json = serde_json::to_value(&page)?;

        sqlx::query!(
            r#"
            INSERT INTO stream_notion_pages (
                source_id,
                page_id,
                url,
                created_time,
                last_edited_time,
                created_by_id,
                created_by_name,
                last_edited_by_id,
                last_edited_by_name,
                parent_type,
                parent_id,
                archived,
                properties,
                raw_json,
                synced_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW()
            )
            ON CONFLICT (source_id, page_id)
            DO UPDATE SET
                url = EXCLUDED.url,
                last_edited_time = EXCLUDED.last_edited_time,
                last_edited_by_id = EXCLUDED.last_edited_by_id,
                last_edited_by_name = EXCLUDED.last_edited_by_name,
                parent_type = EXCLUDED.parent_type,
                parent_id = EXCLUDED.parent_id,
                archived = EXCLUDED.archived,
                properties = EXCLUDED.properties,
                raw_json = EXCLUDED.raw_json,
                synced_at = NOW(),
                updated_at = NOW()
            "#,
            self.source_id,
            page.id,
            page.url,
            page.created_time,
            page.last_edited_time,
            page.created_by.id,
            page.created_by.name,
            page.last_edited_by.id,
            page.last_edited_by.name,
            parent_type,
            parent_id,
            page.archived,
            properties_json,
            raw_json,
        )
        .execute(&self.db)
        .await?;

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
