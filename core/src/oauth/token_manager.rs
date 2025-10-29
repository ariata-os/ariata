//! Centralized token management for all OAuth sources
//!
//! This module provides a unified interface for managing OAuth tokens across all sources.
//! It integrates with the auth.ariata.com OAuth proxy for token refresh operations.

use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};
use super::encryption::TokenEncryptor;

/// Token refresh response from OAuth proxy
#[derive(Debug, Deserialize)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
}

/// OAuth token information
#[derive(Debug, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub provider: String,
}

/// Configuration for the OAuth proxy
#[derive(Debug, Clone)]
pub struct OAuthProxyConfig {
    /// Base URL of the OAuth proxy (e.g., <https://auth.ariata.com>)
    pub base_url: String,
}

impl Default for OAuthProxyConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("OAUTH_PROXY_URL")
                .unwrap_or_else(|_| "https://auth.ariata.com".to_string()),
        }
    }
}

/// Centralized token manager for all OAuth sources
pub struct TokenManager {
    db: PgPool,
    client: Client,
    proxy_config: OAuthProxyConfig,
    encryptor: TokenEncryptor,
}

impl TokenManager {
    /// Create a new token manager
    ///
    /// Requires ARIATA_ENCRYPTION_KEY environment variable to be set.
    /// For development/testing, use `new_insecure()` instead.
    ///
    /// # Errors
    /// Returns error if encryption key is not set or invalid
    pub fn new(db: PgPool) -> Result<Self> {
        Self::with_config(db, OAuthProxyConfig::default())
    }

    /// Create a new token manager with custom configuration
    ///
    /// # Security
    /// Always requires ARIATA_ENCRYPTION_KEY to be set. Tokens are always encrypted.
    ///
    /// # Errors
    /// Returns error if encryption key is not set or invalid
    pub fn with_config(db: PgPool, proxy_config: OAuthProxyConfig) -> Result<Self> {
        // Always require encryption - no insecure mode
        let encryptor = TokenEncryptor::from_env()
            .map_err(|_| Error::Configuration(
                "ARIATA_ENCRYPTION_KEY environment variable is required. \
                 Generate one with: openssl rand -base64 32".to_string()
            ))?;

        tracing::info!("✅ Token encryption enabled");

        Ok(Self {
            db,
            client: Client::new(),
            proxy_config,
            encryptor,
        })
    }

    /// Create a token manager in insecure mode (for testing only)
    ///
    /// # Warning
    /// This stores tokens in plaintext. NEVER use in production!
    #[cfg(test)]
    pub fn new_insecure(db: PgPool) -> Self {
        tracing::warn!("Creating TokenManager in INSECURE mode for tests - tokens in plaintext");

        // Create a test encryption key
        let test_key = base64::engine::general_purpose::STANDARD
            .encode(b"test-key-for-unit-tests-only!");
        let encryptor = TokenEncryptor::from_base64_key(&test_key)
            .expect("test encryption key should be valid");

        Self {
            db,
            client: Client::new(),
            proxy_config: OAuthProxyConfig::default(),
            encryptor,
        }
    }

    /// Get a valid access token for a source, refreshing if necessary
    pub async fn get_valid_token(&self, source_id: Uuid) -> Result<String> {
        // Load token from database
        let token = self.load_token(source_id).await?;

        // Check if token needs refresh
        if self.needs_refresh(&token) {
            let refreshed = self.refresh_token(source_id, &token).await?;
            Ok(refreshed.access_token)
        } else {
            Ok(token.access_token)
        }
    }

    /// Load token information from the database
    pub async fn load_token(&self, source_id: Uuid) -> Result<OAuthToken> {
        let record = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<DateTime<Utc>>)>(
            r#"
            SELECT
                type,
                access_token,
                refresh_token,
                token_expires_at
            FROM sources
            WHERE id = $1 AND is_active = true
            "#
        )
        .bind(source_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| Error::Database(format!("Source not found: {source_id}")))?;

        let (provider, access_token_encrypted, refresh_token_encrypted, token_expires_at) = record;

        let access_token_encrypted = access_token_encrypted
            .ok_or_else(|| Error::Authentication("No access token found".to_string()))?;

        // Decrypt tokens
        let access_token = self.encryptor.decrypt(&access_token_encrypted)?;

        let refresh_token = if let Some(ref rt) = refresh_token_encrypted {
            Some(self.encryptor.decrypt(rt)?)
        } else {
            None
        };

        Ok(OAuthToken {
            access_token,
            refresh_token,
            expires_at: token_expires_at,
            provider,
        })
    }

    /// Check if a token needs refresh
    pub fn needs_refresh(&self, token: &OAuthToken) -> bool {
        token.expires_at
            .map(|exp| exp <= Utc::now() + Duration::minutes(5))
            .unwrap_or(false)
    }

    /// Refresh an OAuth token through the proxy
    #[tracing::instrument(skip(self, token), fields(source_id = %source_id, provider = %token.provider))]
    pub async fn refresh_token(&self, source_id: Uuid, token: &OAuthToken) -> Result<OAuthToken> {
        let refresh_token = token.refresh_token.as_ref()
            .ok_or_else(|| Error::Authentication("No refresh token available".to_string()))?;

        // Call the OAuth proxy refresh endpoint
        let refresh_url = format!("{}/{}/refresh", self.proxy_config.base_url, token.provider);

        let response = self.client
            .post(&refresh_url)
            .json(&serde_json::json!({
                "refresh_token": refresh_token
            }))
            .send()
            .await
            .map_err(|e| Error::Network(format!("Failed to refresh token: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Check for specific error codes
            if status.as_u16() == 401 {
                return Err(Error::Authentication(
                    "Refresh token is invalid or expired. User needs to re-authenticate.".to_string()
                ));
            }

            return Err(Error::Authentication(
                format!("Token refresh failed ({status}): {error_text}")
            ));
        }

        let refresh_response: TokenRefreshResponse = response.json().await
            .map_err(|e| Error::Network(format!("Invalid refresh response: {e}")))?;

        // Calculate new expiry time
        let expires_at = refresh_response.expires_in
            .map(|seconds| Utc::now() + Duration::seconds(seconds));

        // Encrypt tokens before storing
        let access_token_to_store = self.encryptor.encrypt(&refresh_response.access_token)?;

        // Determine the refresh token to keep (new one if provided, otherwise keep old one)
        let new_refresh_token = refresh_response.refresh_token.clone().or_else(|| token.refresh_token.clone());

        let refresh_token_to_store = if let Some(ref rt) = &new_refresh_token {
            Some(self.encryptor.encrypt(rt)?)
        } else {
            None
        };

        // Update tokens in database
        sqlx::query(
            r#"
            UPDATE sources
            SET
                access_token = $1,
                refresh_token = COALESCE($2, refresh_token),
                token_expires_at = $3,
                updated_at = NOW()
            WHERE id = $4
            "#
        )
        .bind(&access_token_to_store)
        .bind(refresh_token_to_store.as_ref())
        .bind(expires_at)
        .bind(source_id)
        .execute(&self.db)
        .await?;

        Ok(OAuthToken {
            access_token: refresh_response.access_token,
            refresh_token: new_refresh_token,
            expires_at,
            provider: token.provider.clone(),
        })
    }

    /// Store initial OAuth tokens from a callback
    pub async fn store_initial_tokens(
        &self,
        provider: &str,
        source_name: &str,
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    ) -> Result<Uuid> {
        let expires_at = expires_in.map(|seconds| Utc::now() + Duration::seconds(seconds));

        // Encrypt tokens before storing
        let access_token_to_store = self.encryptor.encrypt(&access_token)?;

        let refresh_token_to_store = if let Some(ref rt) = refresh_token {
            Some(self.encryptor.encrypt(rt)?)
        } else {
            None
        };

        let source_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO sources (
                type, name, access_token, refresh_token, token_expires_at, is_active
            ) VALUES (
                $1, $2, $3, $4, $5, true
            )
            ON CONFLICT (name)
            DO UPDATE SET
                access_token = EXCLUDED.access_token,
                refresh_token = COALESCE(EXCLUDED.refresh_token, sources.refresh_token),
                token_expires_at = EXCLUDED.token_expires_at,
                is_active = true,
                error_message = NULL,
                error_at = NULL,
                updated_at = NOW()
            RETURNING id
            "#
        )
        .bind(provider)
        .bind(source_name)
        .bind(access_token_to_store)
        .bind(refresh_token_to_store)
        .bind(expires_at)
        .fetch_one(&self.db)
        .await?;

        Ok(source_id)
    }

    /// Mark a source as having authentication errors
    pub async fn mark_auth_error(&self, source_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sources
            SET
                error_message = $1,
                error_at = NOW(),
                updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(error_message)
        .bind(source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Clear authentication errors for a source
    pub async fn clear_auth_error(&self, source_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sources
            SET
                error_message = NULL,
                error_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_needs_refresh() {
        let pool = PgPool::connect_lazy("postgres://test:test@localhost/test").unwrap();
        let manager = TokenManager::new_insecure(pool);

        // Token expiring in 1 minute - should refresh
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(Utc::now() + Duration::minutes(1)),
            provider: "google".to_string(),
        };
        assert!(manager.needs_refresh(&token));

        // Token expiring in 10 minutes - should not refresh
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(Utc::now() + Duration::minutes(10)),
            provider: "google".to_string(),
        };
        assert!(!manager.needs_refresh(&token));

        // Token with no expiry - should not refresh
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: None,
            provider: "google".to_string(),
        };
        assert!(!manager.needs_refresh(&token));
    }
}