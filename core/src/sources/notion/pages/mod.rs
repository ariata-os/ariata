//! Notion pages stream implementation

// Processor no longer needed - we write directly to database
// pub mod processor;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::Result,
    oauth::token_manager::TokenManager,
    sources::base::{
        DataSource, HealthStatus, NotionStartCursor, SyncCursor, SyncLogger, SyncMode, SyncResult,
        TypedCursor,
    },
};

use super::{
    client::NotionApiClient,
    types::{Page, SearchResponse},
};

/// Notion pages source
pub struct NotionPagesSource {
    source_id: Uuid,
    db: PgPool,
    token_manager: Arc<TokenManager>,
    client: NotionApiClient,
}

impl NotionPagesSource {
    /// Create a new Notion pages source
    pub fn new(source_id: Uuid, db: PgPool, token_manager: Arc<TokenManager>) -> Self {
        Self {
            source_id,
            db,
            token_manager,
            client: NotionApiClient::new(),
        }
    }

    /// Search for pages with optional cursor
    async fn search_pages(&self, cursor: Option<&NotionStartCursor>) -> Result<SearchResponse> {
        let token = self
            .token_manager
            .get_valid_token(self.source_id)
            .await?;

        let mut body = json!({
            "filter": {
                "property": "object",
                "value": "page"
            },
            "page_size": 100,
        });

        if let Some(cursor) = cursor {
            body["start_cursor"] = json!(cursor.0);
        }

        self.client.post_json("search", &token, &body).await
    }

    /// Store a page in the database
    async fn store_page(&self, page: Page) -> Result<bool> {
        // Extract key properties
        let title = page
            .properties
            .get("title")
            .or_else(|| page.properties.get("Name"))
            .and_then(|v| {
                v.get("title")
                    .or_else(|| v.get("rich_text"))
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|obj| obj.get("plain_text"))
                    .and_then(|s| s.as_str())
            })
            .unwrap_or("")
            .to_string();

        // Insert into database
        sqlx::query(
            r#"
            INSERT INTO stream_notion_pages (
                source_id, page_id, title, url, created_time, last_edited_time,
                archived, properties, synced_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (source_id, page_id)
            DO UPDATE SET
                title = EXCLUDED.title,
                last_edited_time = EXCLUDED.last_edited_time,
                archived = EXCLUDED.archived,
                properties = EXCLUDED.properties,
                synced_at = EXCLUDED.synced_at,
                updated_at = NOW()
            "#,
        )
        .bind(self.source_id)
        .bind(&page.id)
        .bind(&title)
        .bind(&page.url)
        .bind(page.created_time)
        .bind(page.last_edited_time)
        .bind(page.archived)
        .bind(&page.properties)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;

        Ok(true)
    }

    /// Get the last cursor from the database
    async fn get_last_cursor(&self) -> Result<Option<NotionStartCursor>> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT cursor_metadata FROM sources WHERE id = $1",
        )
        .bind(self.source_id)
        .fetch_one(&self.db)
        .await?;

        if let Some(json) = row.0 {
            let typed = TypedCursor::from_json(&json)?;
            Ok(Some(typed.decode::<NotionStartCursor>()?))
        } else {
            Ok(None)
        }
    }

    /// Save the cursor to the database
    async fn save_cursor(&self, cursor: &NotionStartCursor) -> Result<()> {
        let typed = TypedCursor::new(cursor);

        sqlx::query(
            "UPDATE sources SET cursor_metadata = $1, cursor_type = $2, last_sync_at = $3 WHERE id = $4",
        )
        .bind(typed.to_json())
        .bind(NotionStartCursor::cursor_type())
        .bind(Utc::now())
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get the last sync timestamp
    async fn get_last_sync_at(&self) -> Result<Option<chrono::DateTime<Utc>>> {
        let row = sqlx::query_as::<_, (Option<chrono::DateTime<Utc>>,)>(
            "SELECT last_sync_at FROM sources WHERE id = $1",
        )
        .bind(self.source_id)
        .fetch_one(&self.db)
        .await?;

        Ok(row.0)
    }

    /// Internal sync implementation
    async fn sync_internal(&self, mode: &SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();
        let mut records_fetched = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut next_cursor: Option<NotionStartCursor> = None;

        match mode {
            SyncMode::Incremental { cursor } => {
                // Try to use cursor from mode, or load from database
                let start_cursor = if let Some(cursor_str) = cursor {
                    Some(NotionStartCursor::from_string(cursor_str)?)
                } else {
                    self.get_last_cursor().await?
                };

                // Paginate through pages
                let mut current_cursor = start_cursor;
                loop {
                    let response = self.search_pages(current_cursor.as_ref()).await?;

                    for page in response.results {
                        records_fetched += 1;

                        match self.store_page(page).await {
                            Ok(true) => records_written += 1,
                            Ok(false) => records_failed += 1,
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to store page");
                                records_failed += 1;
                            }
                        }
                    }

                    if !response.has_more {
                        break;
                    }

                    if let Some(cursor_str) = response.next_cursor {
                        let cursor = NotionStartCursor(cursor_str);
                        current_cursor = Some(cursor.clone());
                        next_cursor = Some(cursor);
                    } else {
                        break;
                    }
                }

                // Save final cursor
                if let Some(ref cursor) = next_cursor {
                    self.save_cursor(cursor).await?;
                }
            }
            SyncMode::FullRefresh => {
                // Full refresh - start from beginning
                let mut current_cursor: Option<NotionStartCursor> = None;

                loop {
                    let response = self.search_pages(current_cursor.as_ref()).await?;

                    for page in response.results {
                        records_fetched += 1;

                        match self.store_page(page).await {
                            Ok(true) => records_written += 1,
                            Ok(false) => records_failed += 1,
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to store page");
                                records_failed += 1;
                            }
                        }
                    }

                    if !response.has_more {
                        break;
                    }

                    if let Some(cursor_str) = response.next_cursor {
                        let cursor = NotionStartCursor(cursor_str);
                        current_cursor = Some(cursor.clone());
                        next_cursor = Some(cursor);
                    } else {
                        break;
                    }
                }

                // Save final cursor for future incremental syncs
                if let Some(ref cursor) = next_cursor {
                    self.save_cursor(cursor).await?;
                }
            }
        }

        let completed_at = Utc::now();

        Ok(SyncResult {
            records_fetched,
            records_written,
            records_failed,
            next_cursor: next_cursor.map(|c| c.to_string()),
            started_at,
            completed_at,
        })
    }
}

// Implement DataSource trait
#[async_trait]
impl DataSource for NotionPagesSource {
    fn source_id(&self) -> Uuid {
        self.source_id
    }

    fn source_type(&self) -> &str {
        "notion"
    }

    fn stream_name(&self) -> &str {
        "pages"
    }

    fn validate_config(_config: &Value) -> Result<()>
    where
        Self: Sized,
    {
        // Notion pages doesn't require specific config validation
        Ok(())
    }

    fn required_scopes(&self) -> Vec<String> {
        // Notion uses integration tokens, not traditional OAuth scopes
        vec![]
    }

    async fn sync(&self, mode: &SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();
        let logger = SyncLogger::new(self.db.clone());

        tracing::info!(mode = ?mode, "Starting Notion pages sync");

        // Execute the sync
        match self.sync_internal(mode).await {
            Ok(result) => {
                // Log success to database
                if let Err(e) = logger.log_success(self.source_id, mode, &result).await {
                    tracing::warn!(error = %e, "Failed to log sync success");
                }

                Ok(result)
            }
            Err(e) => {
                // Log failure to database
                if let Err(log_err) = logger
                    .log_failure(self.source_id, mode, started_at, &e)
                    .await
                {
                    tracing::warn!(error = %log_err, "Failed to log sync failure");
                }

                Err(e)
            }
        }
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        // Check if we can get a valid token
        let token_check = self
            .token_manager
            .get_valid_token(self.source_id)
            .await;

        let last_sync = self.get_last_sync_at().await.unwrap_or(None);

        match token_check {
            Ok(_) => Ok(HealthStatus {
                is_healthy: true,
                message: "Notion integration token is valid".to_string(),
                last_check: Utc::now(),
                last_successful_sync: last_sync,
                error_count: 0,
            }),
            Err(e) => Ok(HealthStatus {
                is_healthy: false,
                message: format!("Notion integration token error: {e}"),
                last_check: Utc::now(),
                last_successful_sync: last_sync,
                error_count: 1,
            }),
        }
    }

    fn sync_schedule(&self) -> String {
        // Sync every 30 minutes (Notion data doesn't change as frequently)
        "0 */30 * * * *".to_string()
    }

    async fn should_sync(&self) -> bool {
        // Check if integration token is valid
        match self.token_manager.get_valid_token(self.source_id).await {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!(
                    source_id = %self.source_id,
                    error = %e,
                    "Notion sync skipped: no valid integration token"
                );
                false
            }
        }
    }
}
