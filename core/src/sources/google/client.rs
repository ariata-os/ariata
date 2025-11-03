//! Google API client - thin wrapper over OAuthHttpClient
//!
//! This client delegates all OAuth HTTP operations to the base OAuthHttpClient,
//! providing Google-specific configuration and error handling.

use std::sync::Arc;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    sources::base::{OAuthHttpClient, RetryConfig, TokenManager},
};
use super::error_handler::GoogleErrorHandler;

/// Google API client with automatic token refresh and retry logic
///
/// This is a thin wrapper that configures OAuthHttpClient for Google APIs.
/// All HTTP logic (retry, token refresh, error handling) is delegated to the base client.
pub struct GoogleClient {
    http: OAuthHttpClient,
}

impl GoogleClient {
    /// Create a new Google API client
    pub fn new(source_id: Uuid, token_manager: Arc<TokenManager>) -> Self {
        Self {
            http: OAuthHttpClient::new(source_id, token_manager)
                .with_base_url("https://www.googleapis.com")
                .with_retry_config(RetryConfig::default())
                .with_error_handler(Box::new(GoogleErrorHandler)),
        }
    }

    /// Create a client for a specific API version
    ///
    /// # Example
    /// ```no_run
    /// let client = GoogleClient::with_api(source_id, token_manager, "calendar", "v3");
    /// // Base URL: https://www.googleapis.com/calendar/v3
    /// ```
    pub fn with_api(source_id: Uuid, token_manager: Arc<TokenManager>, api: &str, version: &str) -> Self {
        Self {
            http: OAuthHttpClient::new(source_id, token_manager)
                .with_base_url(&format!("https://www.googleapis.com/{api}/{version}"))
                .with_retry_config(RetryConfig::default())
                .with_error_handler(Box::new(GoogleErrorHandler)),
        }
    }

    /// Make an authenticated GET request
    pub async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.http.get(path).await
    }

    /// Make an authenticated GET request with query parameters
    pub async fn get_with_params<T>(
        &self,
        path: &str,
        params: &[(&str, &str)]
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.http.get_with_params(path, params).await
    }

    /// Check if error is a sync token error (410 response)
    ///
    /// Used by Calendar and Gmail APIs for incremental sync
    pub fn is_sync_token_error(error: &Error) -> bool {
        match error {
            Error::Source(msg) => msg.contains("Sync token") || msg.contains("sync token"),
            _ => false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let pool = sqlx::PgPool::connect_lazy("postgres://test").unwrap();
        let token_manager = Arc::new(TokenManager::new_insecure(pool));
        let _client = GoogleClient::new(Uuid::new_v4(), token_manager);
    }

    #[tokio::test]
    async fn test_client_with_api() {
        let pool = sqlx::PgPool::connect_lazy("postgres://test").unwrap();
        let token_manager = Arc::new(TokenManager::new_insecure(pool));
        let _client = GoogleClient::with_api(Uuid::new_v4(), token_manager, "calendar", "v3");
    }

    #[test]
    fn test_sync_token_error_detection() {
        let error = Error::Source("Sync token is no longer valid".to_string());
        assert!(GoogleClient::is_sync_token_error(&error));

        let error = Error::Source("Invalid sync token".to_string());
        assert!(GoogleClient::is_sync_token_error(&error));

        let error = Error::Http("Not found".to_string());
        assert!(!GoogleClient::is_sync_token_error(&error));
    }
}
