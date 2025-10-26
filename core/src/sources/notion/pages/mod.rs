//! Notion pages stream implementation

pub mod processor;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use serde_json::json;

use crate::{
    error::Result,
    oauth::OAuthManager,
    sources::{DataSource, SourceRecord, SyncState},
};

use super::{
    auth::NotionAuth,
    client::NotionApiClient,
    types::SearchResponse,
};

use processor::PagesProcessor;

/// Notion pages source
pub struct NotionPagesSource {
    auth: NotionAuth,
    client: NotionApiClient,
    processor: PagesProcessor,
}

impl NotionPagesSource {
    /// Create a new Notion pages source
    pub fn new(oauth: Arc<OAuthManager>) -> Self {
        Self {
            auth: NotionAuth::oauth(oauth),
            client: NotionApiClient::new(),
            processor: PagesProcessor::new("notion_pages"),
        }
    }

    /// Create with API key authentication
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            auth: NotionAuth::api_key(api_key),
            client: NotionApiClient::new(),
            processor: PagesProcessor::new("notion_pages"),
        }
    }

    /// Search for pages
    async fn search_pages(&self, cursor: Option<String>) -> Result<SearchResponse> {
        let token = self.auth.get_token().await?;

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

        self.client.post_json("search", &token, &body).await
    }
}

#[async_trait]
impl DataSource for NotionPagesSource {
    fn name(&self) -> &str {
        "notion_pages"
    }

    fn requires_oauth(&self) -> bool {
        self.auth.oauth.is_some()
    }

    async fn fetch(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<SourceRecord>> {
        let mut all_records = Vec::new();
        let mut cursor = None;

        // Paginate through all pages
        loop {
            let response = self.search_pages(cursor).await?;
            let records = self.processor.process_pages(response.results)?;
            all_records.extend(records);

            if !response.has_more {
                break;
            }

            cursor = response.next_cursor;
        }

        // Update sync state
        if !all_records.is_empty() {
            let new_state = SyncState {
                source: self.name().to_string(),
                last_sync: Some(Utc::now()),
                sync_token: None,
                cursor: None,
                checkpoint: None,
            };
            self.update_sync_state(new_state).await?;
        }

        Ok(all_records)
    }

    async fn get_sync_state(&self) -> Result<SyncState> {
        // IMPLEMENTATION GAP: Sync state not persisted for Notion source.
        // This means each sync runs as a full refresh instead of incremental.
        // To implement: Query sources table for last_sync_cursor or add notion_sync_state table.
        Ok(SyncState {
            source: self.name().to_string(),
            last_sync: None,
            sync_token: None,
            cursor: None,
            checkpoint: None,
        })
    }

    async fn update_sync_state(&self, state: SyncState) -> Result<()> {
        // IMPLEMENTATION GAP: Sync state not persisted (see get_sync_state above).
        tracing::debug!("Sync state update (not persisted) for {}: {:?}", self.name(), state);
        Ok(())
    }
}