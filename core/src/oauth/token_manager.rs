//! Centralized token management for all OAuth sources
//!
//! This module provides a unified interface for managing OAuth tokens across all sources.
//! It integrates with the auth.ariata.com OAuth proxy for token refresh operations.

use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{Error, Result};

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
    /// Base URL of the OAuth proxy (e.g., https://auth.ariata.com)
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
}

impl TokenManager {
    /// Create a new token manager
    pub fn new(db: PgPool) -> Self {
        Self::with_config(db, OAuthProxyConfig::default())
    }

    /// Create a new token manager with custom configuration
    pub fn with_config(db: PgPool, proxy_config: OAuthProxyConfig) -> Self {
        Self {
            db,
            client: Client::new(),
            proxy_config,
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
        .ok_or_else(|| Error::Database(format!("Source not found: {}", source_id)))?;

        let (provider, access_token, refresh_token, token_expires_at) = record;

        let access_token = access_token
            .ok_or_else(|| Error::Authentication("No access token found".to_string()))?;

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
            .map_err(|e| Error::Network(format!("Failed to refresh token: {}", e)))?;

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
                format!("Token refresh failed ({}): {}", status, error_text)
            ));
        }

        let refresh_response: TokenRefreshResponse = response.json().await
            .map_err(|e| Error::Network(format!("Invalid refresh response: {}", e)))?;

        // Calculate new expiry time
        let expires_at = refresh_response.expires_in
            .map(|seconds| Utc::now() + Duration::seconds(seconds));

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
        .bind(&refresh_response.access_token)
        .bind(refresh_response.refresh_token.as_ref().or(Some(refresh_token)))
        .bind(expires_at)
        .bind(source_id)
        .execute(&self.db)
        .await?;

        Ok(OAuthToken {
            access_token: refresh_response.access_token,
            refresh_token: refresh_response.refresh_token.or_else(|| token.refresh_token.clone()),
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
        .bind(access_token)
        .bind(refresh_token)
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

    #[test]
    fn test_needs_refresh() {
        let pool = PgPool::connect_lazy("postgres://test:test@localhost/test").unwrap();
        let manager = TokenManager::new(pool);

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