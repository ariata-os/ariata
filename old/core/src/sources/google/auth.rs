//! Google OAuth authentication utilities

use std::sync::Arc;
use crate::{
    error::Result,
    oauth::OAuthManager,
};

/// Google OAuth helper
pub struct GoogleAuth {
    oauth: Arc<OAuthManager>,
    provider: String,
}

impl GoogleAuth {
    /// Create a new Google auth helper
    pub fn new(oauth: Arc<OAuthManager>) -> Self {
        Self {
            oauth,
            provider: "google".to_string(),
        }
    }

    /// Get a valid access token for Google APIs
    pub async fn get_token(&self) -> Result<String> {
        self.oauth.get_valid_token(&self.provider).await
    }

    /// Check if user has valid Google OAuth
    pub async fn is_authenticated(&self) -> bool {
        self.get_token().await.is_ok()
    }
}