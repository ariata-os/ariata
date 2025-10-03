//! HTTP client utilities shared across sources

use reqwest::Client;
use serde::de::DeserializeOwned;
use crate::error::{Error, Result};

/// Shared HTTP client with common functionality
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Perform a GET request with bearer auth and return parsed JSON
    pub async fn get_json<T>(&self, url: &str, token: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self.client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| Error::Other(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("API error: {}", error)));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {}", e)))
    }

    /// Perform a GET request with bearer auth and query parameters
    pub async fn get_json_with_params<T>(&self, url: &str, token: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self.client
            .get(url)
            .bearer_auth(token)
            .query(params)
            .send()
            .await
            .map_err(|e| Error::Other(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("API error: {}", error)));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {}", e)))
    }

    /// Check if error indicates invalid sync token
    pub fn is_sync_token_error(error_text: &str) -> bool {
        error_text.contains("Sync token is no longer valid") ||
        error_text.contains("Invalid sync token") ||
        error_text.contains("410")
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}