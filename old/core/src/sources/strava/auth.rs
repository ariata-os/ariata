//! Strava OAuth authentication utilities

use std::sync::Arc;
use crate::{
    error::Result,
    oauth::OAuthManager,
};

/// Strava OAuth helper
pub struct StravaAuth {
    oauth: Arc<OAuthManager>,
    provider: String,
}

impl StravaAuth {
    /// Create a new Strava auth helper
    pub fn new(oauth: Arc<OAuthManager>) -> Self {
        Self {
            oauth,
            provider: "strava".to_string(),
        }
    }

    /// Get a valid access token for Strava API
    pub async fn get_token(&self) -> Result<String> {
        self.oauth.get_valid_token(&self.provider).await
    }

    /// Check if user has valid Strava OAuth
    pub async fn is_authenticated(&self) -> bool {
        self.get_token().await.is_ok()
    }
}