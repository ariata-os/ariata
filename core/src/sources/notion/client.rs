//! Notion API client - thin wrapper over OAuthHttpClient
//!
//! This client delegates all OAuth HTTP operations to the base OAuthHttpClient,
//! providing Notion-specific configuration and error handling.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use super::error_handler::NotionErrorHandler;
use crate::{
    error::Result,
    sources::base::{OAuthHttpClient, RetryConfig, TokenManager},
};

/// Notion API client with automatic token refresh and retry logic
///
/// This is a thin wrapper that configures OAuthHttpClient for Notion APIs.
/// All HTTP logic (retry, token refresh, error handling) is delegated to the base client.
pub struct NotionApiClient {
    http: OAuthHttpClient,
}

impl NotionApiClient {
    /// Create a new Notion API client
    pub fn new(source_id: Uuid, token_manager: Arc<TokenManager>) -> Self {
        Self {
            http: OAuthHttpClient::new(source_id, token_manager)
                .with_base_url("https://api.notion.com/v1")
                .with_retry_config(RetryConfig::default())
                .with_header("Notion-Version", "2022-06-28")
                .with_error_handler(Box::new(NotionErrorHandler)),
        }
    }

    /// Make an authenticated GET request
    pub async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.http.get(path).await
    }

    /// Make an authenticated POST request with JSON body
    pub async fn post_json<T>(&self, path: &str, body: &impl Serialize) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.http.post(path, body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let pool = sqlx::PgPool::connect_lazy("postgres://test").unwrap();
        let token_manager = Arc::new(TokenManager::new_insecure(pool));
        let _client = NotionApiClient::new(Uuid::new_v4(), token_manager);
    }
}
