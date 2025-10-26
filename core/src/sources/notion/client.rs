//! Notion API client

use reqwest::{Client, header};
use serde::de::DeserializeOwned;
use crate::error::{Error, Result};

/// Notion API client
pub struct NotionApiClient {
    client: Client,
    base_url: String,
}

impl Default for NotionApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NotionApiClient {
    /// Create a new Notion API client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.notion.com/v1".to_string(),
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
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("Notion-Version", "2022-06-28")
            .send()
            .await
            .map_err(|e| Error::Other(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("Notion API error: {error}")));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {e}")))
    }

    /// Make an authenticated POST request for searches
    pub async fn post_json<T>(&self, path: &str, token: &str, body: &serde_json::Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));

        let response = self.client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("Notion-Version", "2022-06-28")
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Other(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!("Notion API error: {error}")));
        }

        response.json::<T>().await
            .map_err(|e| Error::Other(format!("Failed to parse response: {e}")))
    }
}