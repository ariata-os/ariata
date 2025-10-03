//! Authentication helpers for sources

use std::sync::Arc;
use crate::{
    error::Result,
    oauth::OAuthManager,
};

/// Authentication type for a source
#[derive(Debug, Clone)]
pub enum AuthType {
    /// OAuth 2.0 authentication
    OAuth(String), // provider name
    /// API key authentication
    ApiKey(String),
    /// No authentication required
    None,
}

/// Helper for handling authentication
pub struct AuthHelper {
    oauth_manager: Option<Arc<OAuthManager>>,
    auth_type: AuthType,
}

impl AuthHelper {
    /// Create a new OAuth-based auth helper
    pub fn oauth(oauth_manager: Arc<OAuthManager>, provider: &str) -> Self {
        Self {
            oauth_manager: Some(oauth_manager),
            auth_type: AuthType::OAuth(provider.to_string()),
        }
    }

    /// Create a new API key-based auth helper
    pub fn api_key(key: String) -> Self {
        Self {
            oauth_manager: None,
            auth_type: AuthType::ApiKey(key),
        }
    }

    /// Create a no-auth helper
    pub fn none() -> Self {
        Self {
            oauth_manager: None,
            auth_type: AuthType::None,
        }
    }

    /// Get authentication token/key
    pub async fn get_token(&self) -> Result<String> {
        match &self.auth_type {
            AuthType::OAuth(provider) => {
                let oauth = self.oauth_manager.as_ref()
                    .ok_or_else(|| crate::error::Error::Other("OAuth manager not initialized".to_string()))?;
                oauth.get_valid_token(provider).await
            }
            AuthType::ApiKey(key) => Ok(key.clone()),
            AuthType::None => Ok(String::new()),
        }
    }

    /// Check if authentication is required
    pub fn requires_auth(&self) -> bool {
        !matches!(self.auth_type, AuthType::None)
    }

    /// Check if this uses OAuth
    pub fn requires_oauth(&self) -> bool {
        matches!(self.auth_type, AuthType::OAuth(_))
    }
}