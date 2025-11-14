//! Unified authentication abstraction for all source types
//!
//! This module provides a common interface for handling authentication across
//! different source types: OAuth2, Device, API Key, and unauthenticated sources.

use std::sync::Arc;
use uuid::Uuid;

use super::base::TokenManager;
use crate::error::Result;

/// Unified authentication abstraction for all source types
///
/// This enum encapsulates the different authentication patterns used across sources:
/// - OAuth2: Google, Strava, Notion (requires token management and refresh)
/// - Device: iOS, Mac (identified by device_id, no external auth)
/// - ApiKey: Future sources that use API keys
/// - None: Public data sources or sources without authentication
#[derive(Clone)]
pub enum SourceAuth {
    /// OAuth 2.0 authentication with automatic token refresh
    OAuth2 {
        source_id: Uuid,
        token_manager: Arc<TokenManager>,
    },

    /// Device-based authentication (iOS, Mac devices pushing data)
    Device { device_id: String },

    /// API Key authentication
    ApiKey { key: String },

    /// No authentication required
    None,
}

/// Credentials extracted from authentication
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Bearer token for OAuth2 requests
    BearerToken(String),

    /// Device identifier
    DeviceId(String),

    /// API key (implementation-specific where it goes: header, query param, etc)
    ApiKey(String),

    /// No credentials
    None,
}

impl SourceAuth {
    /// Create OAuth2 authentication
    pub fn oauth2(source_id: Uuid, token_manager: Arc<TokenManager>) -> Self {
        Self::OAuth2 {
            source_id,
            token_manager,
        }
    }

    /// Create device authentication
    pub fn device(device_id: impl Into<String>) -> Self {
        Self::Device {
            device_id: device_id.into(),
        }
    }

    /// Create API key authentication
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey { key: key.into() }
    }

    /// Create no authentication
    pub fn none() -> Self {
        Self::None
    }

    /// Get credentials for making authenticated requests
    ///
    /// For OAuth2 sources, this will automatically refresh tokens if needed.
    /// For device/API key sources, this returns the stored credential.
    pub async fn get_credentials(&self) -> Result<Credentials> {
        match self {
            Self::OAuth2 {
                source_id,
                token_manager,
            } => {
                let token = token_manager.get_valid_token(*source_id).await?;
                Ok(Credentials::BearerToken(token))
            }
            Self::Device { device_id } => Ok(Credentials::DeviceId(device_id.clone())),
            Self::ApiKey { key } => Ok(Credentials::ApiKey(key.clone())),
            Self::None => Ok(Credentials::None),
        }
    }

    /// Check if this auth requires OAuth
    pub fn is_oauth(&self) -> bool {
        matches!(self, Self::OAuth2 { .. })
    }

    /// Check if this is device-based auth
    pub fn is_device(&self) -> bool {
        matches!(self, Self::Device { .. })
    }

    /// Get the source_id for OAuth2 sources
    pub fn source_id(&self) -> Option<Uuid> {
        match self {
            Self::OAuth2 { source_id, .. } => Some(*source_id),
            _ => None,
        }
    }

    /// Get the TokenManager for OAuth2 sources
    pub fn token_manager(&self) -> Option<&Arc<TokenManager>> {
        match self {
            Self::OAuth2 { token_manager, .. } => Some(token_manager),
            _ => None,
        }
    }
}

impl std::fmt::Debug for SourceAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OAuth2 { source_id, .. } => f
                .debug_struct("OAuth2")
                .field("source_id", source_id)
                .finish(),
            Self::Device { device_id } => f
                .debug_struct("Device")
                .field("device_id", device_id)
                .finish(),
            Self::ApiKey { .. } => f.debug_struct("ApiKey").finish_non_exhaustive(),
            Self::None => write!(f, "None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[tokio::test]
    async fn test_auth_type_checks() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let tm = Arc::new(TokenManager::new_insecure(pool));
        let auth = SourceAuth::oauth2(Uuid::new_v4(), tm);
        assert!(auth.is_oauth());
        assert!(!auth.is_device());

        let auth = SourceAuth::device("test-device");
        assert!(!auth.is_oauth());
        assert!(auth.is_device());

        let auth = SourceAuth::none();
        assert!(!auth.is_oauth());
        assert!(!auth.is_device());
    }

    #[tokio::test]
    async fn test_source_id_extraction() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let tm = Arc::new(TokenManager::new_insecure(pool));
        let source_id = Uuid::new_v4();
        let auth = SourceAuth::oauth2(source_id, tm);
        assert_eq!(auth.source_id(), Some(source_id));

        let auth = SourceAuth::device("test");
        assert_eq!(auth.source_id(), None);
    }
}
