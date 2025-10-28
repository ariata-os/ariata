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
        base::{SyncLogger, SyncMode, SyncResult},
        stream::Stream,
    },
};

use super::{client::NotionApiClient, types::SearchResponse};

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
        let started_at = Utc::now();
        let logger = SyncLogger::new(self.db.clone());

        tracing::info!("Starting Notion pages sync");

        let mut all_pages = Vec::new();
        let mut cursor = None;
        let mut records_fetched = 0;

        // Paginate through all pages
        loop {
            let response = self.search_pages(cursor).await?;
            records_fetched += response.results.len();

            // TODO: Write pages to stream_notion_pages table
            // For now, just count them
            all_pages.extend(response.results);

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

        // Log success
        if let Err(e) = logger
            .log_success(self.source_id, "pages", mode, &result)
            .await
        {
            tracing::warn!(error = %e, "Failed to log sync success");
        }

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
