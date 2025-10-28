//! Universal OAuth HTTP client with automatic token refresh and retry logic
//!
//! This module provides a composable base layer for all OAuth-based API clients.
//! It handles:
//! - Automatic token refresh on 401 errors
//! - Exponential backoff retry for rate limits and server errors
//! - Provider-specific error handling via ErrorHandler trait
//! - Request cloning for safe retries
//!
//! # Example
//!
//! ```rust,no_run
//! use ariata::sources::base::{OAuthHttpClient, RetryConfig, DefaultErrorHandler};
//! use std::sync::Arc;
//!
//! let client = OAuthHttpClient::new(source_id, token_manager)
//!     .with_base_url("https://api.example.com/v1")
//!     .with_retry_config(RetryConfig::default())
//!     .with_error_handler(Box::new(DefaultErrorHandler));
//!
//! // Automatic token refresh, retry, and error handling
//! let response: MyApiResponse = client.get("endpoint").await?;
//! ```

use std::sync::Arc;
use std::time::Duration;
use reqwest::{Client, RequestBuilder, Response, StatusCode, header::HeaderMap};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    oauth::token_manager::TokenManager,
};
use super::error_handler::{ErrorHandler, ErrorClass, DefaultErrorHandler};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,

    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,

    /// Whether to retry on 401 (auth) errors
    pub retry_on_401: bool,

    /// Whether to retry on 429 (rate limit) errors
    pub retry_on_429: bool,

    /// Whether to retry on 5xx (server) errors
    pub retry_on_5xx: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,  // 1 second
            max_backoff_ms: 30000,      // 30 seconds
            retry_on_401: true,
            retry_on_429: true,
            retry_on_5xx: true,
        }
    }
}

impl RetryConfig {
    /// Create a config with no retries
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Create a config optimized for rate-limited APIs
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_backoff_ms: 500,
            max_backoff_ms: 60000,
            ..Default::default()
        }
    }
}

/// Universal OAuth HTTP client with automatic token management and retry logic
pub struct OAuthHttpClient {
    source_id: Uuid,
    token_manager: Arc<TokenManager>,
    base_url: String,
    client: Client,
    config: RetryConfig,
    custom_headers: HeaderMap,
    error_handler: Box<dyn ErrorHandler>,
}

impl OAuthHttpClient {
    /// Create a new OAuth HTTP client
    ///
    /// # Arguments
    /// * `source_id` - UUID of the source for token lookups
    /// * `token_manager` - Shared token manager for OAuth token operations
    pub fn new(source_id: Uuid, token_manager: Arc<TokenManager>) -> Self {
        // Configure HTTP client with timeouts to prevent infinite hangs
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))  // TCP connection timeout
            .timeout(Duration::from_secs(60))           // Total request timeout
            .build()
            .expect("Failed to build HTTP client");

        Self {
            source_id,
            token_manager,
            base_url: String::new(),
            client,
            config: RetryConfig::default(),
            custom_headers: HeaderMap::new(),
            error_handler: Box::new(DefaultErrorHandler),
        }
    }

    /// Set the base URL for API requests
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    /// Set custom retry configuration
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a custom header to all requests
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        use reqwest::header::{HeaderName, HeaderValue};

        let header_name: HeaderName = key.parse().expect("Invalid header key");
        let header_value: HeaderValue = value.parse().expect("Invalid header value");
        self.custom_headers.insert(header_name, header_value);
        self
    }

    /// Set a custom error handler for provider-specific logic
    pub fn with_error_handler(mut self, handler: Box<dyn ErrorHandler>) -> Self {
        self.error_handler = handler;
        self
    }

    /// Make an authenticated GET request
    pub async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
        let request = self.client.get(&url);
        let response = self.execute_with_retry(request).await?;
        self.parse_response(response).await
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
        let url = self.build_url(path);
        let request = self.client.get(&url).query(params);
        let response = self.execute_with_retry(request).await?;
        self.parse_response(response).await
    }

    /// Make an authenticated POST request with JSON body
    pub async fn post<T>(&self, path: &str, body: &impl Serialize) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
        let request = self.client.post(&url).json(body);
        let response = self.execute_with_retry(request).await?;
        self.parse_response(response).await
    }

    /// Make an authenticated PUT request with JSON body
    pub async fn put<T>(&self, path: &str, body: &impl Serialize) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
        let request = self.client.put(&url).json(body);
        let response = self.execute_with_retry(request).await?;
        self.parse_response(response).await
    }

    /// Make an authenticated DELETE request
    pub async fn delete<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
        let request = self.client.delete(&url);
        let response = self.execute_with_retry(request).await?;
        self.parse_response(response).await
    }

    /// Execute a request with automatic retry and token refresh
    async fn execute_with_retry(&self, request_builder: RequestBuilder) -> Result<Response> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            // Get a valid token (TokenManager handles caching and refresh)
            let token = self.token_manager.get_valid_token(self.source_id).await?;

            // Clone the request builder for this attempt
            let mut request = request_builder
                .try_clone()
                .ok_or_else(|| Error::Other("Request builder not cloneable - ensure body is cloneable".to_string()))?
                .bearer_auth(&token);

            // Apply custom headers
            if !self.custom_headers.is_empty() {
                request = request.headers(self.custom_headers.clone());
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();

                    // Success - return response
                    if status.is_success() {
                        return Ok(response);
                    }

                    // Get response body for error classification
                    let error_body = response.text().await.unwrap_or_default();

                    // Classify the error
                    let error_class = self.error_handler.classify_error(status, &error_body);

                    // Handle sync token errors first (before retry logic)
                    if matches!(error_class, ErrorClass::SyncTokenError) {
                        // Sync token invalid - let caller handle this
                        return Err(Error::Source(format!("Sync token invalid: {error_body}")));
                    }

                    // Check if we should retry
                    if self.error_handler.should_retry(status, attempt, self.config.max_retries) {
                        match error_class {
                            ErrorClass::AuthError => {
                                // Token might be invalid - TokenManager will refresh on next attempt
                                if attempt < self.config.max_retries - 1 {
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    continue;
                                }
                            }
                            ErrorClass::RateLimit => {
                                // Rate limited - back off exponentially
                                if attempt < self.config.max_retries - 1 {
                                    let wait_time = self.calculate_backoff(attempt);
                                    tokio::time::sleep(wait_time).await;
                                    continue;
                                }
                            }
                            ErrorClass::ServerError => {
                                // Server error - back off and retry
                                if attempt < self.config.max_retries - 1 {
                                    let wait_time = self.calculate_backoff(attempt);
                                    tokio::time::sleep(wait_time).await;
                                    continue;
                                }
                            }
                            ErrorClass::SyncTokenError => {
                                // Already handled above
                            }
                            ErrorClass::ClientError | ErrorClass::NetworkError => {
                                // Don't retry client errors
                            }
                        }
                    }

                    // No retry or max retries reached
                    return Err(self.format_error(status, &error_body));
                }
                Err(e) => {
                    // Network error - retry with backoff
                    last_error = Some(e);
                    if attempt < self.config.max_retries - 1 {
                        let wait_time = self.calculate_backoff(attempt);
                        tokio::time::sleep(wait_time).await;
                        continue;
                    }
                }
            }
        }

        // All retries exhausted
        Err(Error::Network(format!(
            "Request failed after {} retries: {:?}",
            self.config.max_retries,
            last_error
        )))
    }

    /// Parse JSON response
    async fn parse_response<T>(&self, response: Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {e}")))
    }

    /// Build full URL from path
    fn build_url(&self, path: &str) -> String {
        if self.base_url.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), path.trim_start_matches('/'))
        }
    }

    /// Calculate exponential backoff time
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let backoff_ms = (self.config.initial_backoff_ms * 2_u64.pow(attempt))
            .min(self.config.max_backoff_ms);
        Duration::from_millis(backoff_ms)
    }

    /// Format error message based on status and body
    fn format_error(&self, status: StatusCode, body: &str) -> Error {
        Error::Http(format!("API error ({status}): {body}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backoff_calculation() {
        let pool = sqlx::PgPool::connect_lazy("postgres://test").unwrap();
        let token_manager = Arc::new(TokenManager::new(pool));
        let client = OAuthHttpClient::new(Uuid::new_v4(), token_manager);

        assert_eq!(client.calculate_backoff(0), Duration::from_secs(1));
        assert_eq!(client.calculate_backoff(1), Duration::from_secs(2));
        assert_eq!(client.calculate_backoff(2), Duration::from_secs(4));
        assert_eq!(client.calculate_backoff(3), Duration::from_secs(8));
        assert_eq!(client.calculate_backoff(4), Duration::from_secs(16));
        assert_eq!(client.calculate_backoff(5), Duration::from_secs(30)); // Max
        assert_eq!(client.calculate_backoff(10), Duration::from_secs(30)); // Still max
    }

    #[tokio::test]
    async fn test_build_url() {
        let pool = sqlx::PgPool::connect_lazy("postgres://test").unwrap();
        let token_manager = Arc::new(TokenManager::new(pool));
        let client = OAuthHttpClient::new(Uuid::new_v4(), token_manager)
            .with_base_url("https://api.example.com/v1");

        assert_eq!(client.build_url("users"), "https://api.example.com/v1/users");
        assert_eq!(client.build_url("/users"), "https://api.example.com/v1/users");
        assert_eq!(client.build_url("users/me"), "https://api.example.com/v1/users/me");
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 1000);
        assert_eq!(config.max_backoff_ms, 30000);
        assert!(config.retry_on_401);
        assert!(config.retry_on_429);
        assert!(config.retry_on_5xx);
    }

    #[test]
    fn test_retry_config_no_retry() {
        let config = RetryConfig::no_retry();
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff_ms, 500);
        assert_eq!(config.max_backoff_ms, 60000);
    }
}
