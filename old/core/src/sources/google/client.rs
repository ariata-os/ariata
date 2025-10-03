//! Google API client utilities

use reqwest::Client;
use serde::de::DeserializeOwned;
use crate::error::{Error, Result};

/// Google API client
pub struct GoogleApiClient {
    client: Client,
    base_url: String,
}

impl GoogleApiClient {
    /// Create a new Google API client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://www.googleapis.com".to_string(),
        }
    }

    /// Create a client for a specific API version
    pub fn with_api(api: &str, version: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("https://www.googleapis.com/{}/{}", api, version),
        }
    }

    /// Make an authenticated GET request
    pub async fn get<T>(&self, path: &str, token: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));

        let response = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| Error::Other(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("Google API error: {}", error)));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {}", e)))
    }

    /// Make an authenticated GET request with query parameters
    pub async fn get_with_params<T>(
        &self,
        path: &str,
        token: &str,
        params: &[(&str, &str)]
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));

        let response = self.client
            .get(&url)
            .bearer_auth(token)
            .query(params)
            .send()
            .await
            .map_err(|e| Error::Other(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("Google API error: {}", error)));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {}", e)))
    }

    /// Check if error is a sync token error
    pub fn is_sync_token_error(error: &str) -> bool {
        error.contains("Sync token is no longer valid") ||
        error.contains("Invalid sync token") ||
        error.contains("410")
    }
}