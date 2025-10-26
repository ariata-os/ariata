//! Strava API client

use reqwest::Client;
use serde::de::DeserializeOwned;
use crate::error::{Error, Result};

/// Strava API client
pub struct StravaApiClient {
    client: Client,
    base_url: String,
}

impl Default for StravaApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl StravaApiClient {
    /// Create a new Strava API client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://www.strava.com/api/v3".to_string(),
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
            .map_err(|e| Error::Other(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("Strava API error: {error}")));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {e}")))
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
            .map_err(|e| Error::Other(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("Strava API error: {error}")));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {e}")))
    }
}