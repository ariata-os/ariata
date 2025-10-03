//! Notion authentication utilities

use std::sync::Arc;
use crate::{
    error::Result,
    oauth::OAuthManager,
};

/// Notion auth helper (OAuth or API key)
pub struct NotionAuth {
    pub oauth: Option<Arc<OAuthManager>>,
    api_key: Option<String>,
}

impl NotionAuth {
    /// Create OAuth-based auth
    pub fn oauth(oauth: Arc<OAuthManager>) -> Self {
        Self {
            oauth: Some(oauth),
            api_key: None,
        }
    }

    /// Create API key-based auth
    pub fn api_key(key: String) -> Self {
        Self {
            oauth: None,
            api_key: Some(key),
        }
    }

    /// Get authentication token
    pub async fn get_token(&self) -> Result<String> {
        if let Some(key) = &self.api_key {
            return Ok(key.clone());
        }

        if let Some(oauth) = &self.oauth {
            return oauth.get_valid_token("notion").await;
        }

        Err(crate::error::Error::Other("No authentication configured".to_string()))
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.get_token().await.is_ok()
    }
}