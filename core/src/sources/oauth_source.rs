//! Composable OAuth source trait for all OAuth-based sources
//!
//! This module provides a common interface and shared functionality
//! for all sources that use OAuth authentication.

use async_trait::async_trait;
use reqwest::Client;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    oauth::token_manager::TokenManager,
};

/// Common trait for all OAuth-based sources
#[async_trait]
pub trait OAuthSource: Send + Sync {
    /// Get the source ID
    fn source_id(&self) -> Uuid;

    /// Get the provider name (e.g., "google", "strava", "notion")
    fn provider(&self) -> &str;

    /// Get the token manager
    fn token_manager(&self) -> &TokenManager;

    /// Get a valid access token, refreshing if necessary
    async fn get_valid_token(&self) -> Result<String> {
        self.token_manager()
            .get_valid_token(self.source_id())
            .await
    }

    /// Make an authenticated HTTP request with automatic token refresh
    /// Note: The request builder must be cloneable for retry logic to work
    async fn authenticated_request(
        &self,
        _client: &Client,
        request_builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        // Get valid token
        let token = self.get_valid_token().await?;

        // Clone the request builder so we can retry if needed
        let retry_builder = request_builder.try_clone()
            .ok_or_else(|| Error::Other("Request builder not cloneable - use try_clone()".to_string()))?;

        // Make request with token
        let response = request_builder
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Request failed: {}", e)))?;

        // Check if we got a 401 (unauthorized)
        if response.status().as_u16() == 401 {
            // Try to refresh token
            let token_manager = self.token_manager();
            let current_token = token_manager.load_token(self.source_id()).await?;

            if current_token.refresh_token.is_some() {
                // Refresh and retry
                let refreshed = token_manager
                    .refresh_token(self.source_id(), &current_token)
                    .await?;

                // Retry with the cloned request builder
                let response = retry_builder
                    .bearer_auth(&refreshed.access_token)
                    .send()
                    .await
                    .map_err(|e| Error::Network(format!("Retry request failed: {}", e)))?;

                if response.status().is_success() {
                    return Ok(response);
                }
            }

            // Mark auth error if refresh failed or no refresh token
            token_manager
                .mark_auth_error(
                    self.source_id(),
                    "Authentication failed. User needs to re-authenticate.",
                )
                .await?;

            return Err(Error::Authentication(
                "Authentication failed. Please reconnect your account.".to_string(),
            ));
        }

        Ok(response)
    }

    /// Handle authentication errors by marking the source
    async fn handle_auth_error(&self, error: &str) -> Result<()> {
        self.token_manager()
            .mark_auth_error(self.source_id(), error)
            .await
    }

    /// Clear any authentication errors
    async fn clear_auth_error(&self) -> Result<()> {
        self.token_manager()
            .clear_auth_error(self.source_id())
            .await
    }
}

/// Base implementation for OAuth sources
pub struct BaseOAuthSource {
    source_id: Uuid,
    provider: String,
    token_manager: Arc<TokenManager>,
}

impl BaseOAuthSource {
    /// Create a new base OAuth source
    pub fn new(source_id: Uuid, provider: String, token_manager: Arc<TokenManager>) -> Self {
        Self {
            source_id,
            provider,
            token_manager,
        }
    }
}

#[async_trait]
impl OAuthSource for BaseOAuthSource {
    fn source_id(&self) -> Uuid {
        self.source_id
    }

    fn provider(&self) -> &str {
        &self.provider
    }

    fn token_manager(&self) -> &TokenManager {
        &self.token_manager
    }
}

/// Builder for creating OAuth sources
pub struct OAuthSourceBuilder {
    db: PgPool,
    proxy_url: Option<String>,
}

impl OAuthSourceBuilder {
    /// Create a new builder
    pub fn new(db: PgPool) -> Self {
        Self { db, proxy_url: None }
    }

    /// Set a custom OAuth proxy URL
    pub fn with_proxy_url(mut self, url: String) -> Self {
        self.proxy_url = Some(url);
        self
    }

    /// Build a token manager
    pub fn build_token_manager(self) -> Arc<TokenManager> {
        let config = if let Some(url) = self.proxy_url {
            crate::oauth::token_manager::OAuthProxyConfig { base_url: url }
        } else {
            crate::oauth::token_manager::OAuthProxyConfig::default()
        };

        Arc::new(TokenManager::with_config(self.db, config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestOAuthSource {
        base: BaseOAuthSource,
    }

    #[async_trait]
    impl OAuthSource for TestOAuthSource {
        fn source_id(&self) -> Uuid {
            self.base.source_id()
        }

        fn provider(&self) -> &str {
            self.base.provider()
        }

        fn token_manager(&self) -> &TokenManager {
            self.base.token_manager()
        }
    }

    #[tokio::test]
    async fn test_oauth_source_creation() {
        let source_id = Uuid::new_v4();
        // Use connect_lazy for test
        let pool = PgPool::connect_lazy("postgres://test:test@localhost/test").unwrap();
        let token_manager = Arc::new(TokenManager::new(pool));

        let source = BaseOAuthSource::new(
            source_id,
            "google".to_string(),
            token_manager,
        );

        assert_eq!(source.source_id(), source_id);
        assert_eq!(source.provider(), "google");
    }
}