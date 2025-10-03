//! OAuth authentication and token management

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, RefreshToken, Scope, TokenResponse,
    TokenUrl, basic::BasicClient,
    reqwest::async_http_client,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};

use crate::{
    error::{Error, Result},
    database::Database,
};

/// OAuth provider configuration
#[derive(Debug, Clone)]
pub struct OAuthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

/// Stored OAuth credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

/// OAuth manager for handling authentication flows
pub struct OAuthManager {
    db: Arc<Database>,
    providers: HashMap<String, OAuthProvider>,
    tokens: Arc<RwLock<HashMap<String, OAuthCredentials>>>,
}

impl OAuthManager {
    /// Create a new OAuth manager
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            providers: HashMap::new(),
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an OAuth provider
    pub fn register_provider(&mut self, provider: OAuthProvider) {
        self.providers.insert(provider.name.clone(), provider);
    }

    /// Configure Google OAuth provider
    pub fn configure_google(&mut self, client_id: String, client_secret: String) {
        self.register_provider(OAuthProvider {
            name: "google".to_string(),
            client_id,
            client_secret,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scopes: vec![
                "https://www.googleapis.com/auth/calendar.readonly".to_string(),
                "https://www.googleapis.com/auth/gmail.readonly".to_string(),
            ],
            redirect_uri: "http://localhost:8000/oauth/callback".to_string(),
        });
    }

    /// Configure Strava OAuth provider
    pub fn configure_strava(&mut self, client_id: String, client_secret: String) {
        self.register_provider(OAuthProvider {
            name: "strava".to_string(),
            client_id,
            client_secret,
            auth_url: "https://www.strava.com/oauth/authorize".to_string(),
            token_url: "https://www.strava.com/oauth/token".to_string(),
            scopes: vec!["activity:read_all".to_string()],
            redirect_uri: "http://localhost:8000/oauth/callback".to_string(),
        });
    }

    /// Generate OAuth authorization URL
    pub fn get_auth_url(&self, provider_name: &str) -> Result<(String, CsrfToken)> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| Error::Other(format!("Unknown provider: {}", provider_name)))?;

        let client = BasicClient::new(
            ClientId::new(provider.client_id.clone()),
            Some(ClientSecret::new(provider.client_secret.clone())),
            AuthUrl::new(provider.auth_url.clone())?,
            Some(TokenUrl::new(provider.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(provider.redirect_uri.clone())?);

        // Generate PKCE challenge for security
        let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(provider.scopes.iter().map(|s| Scope::new(s.clone())))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok((auth_url.to_string(), csrf_token))
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        provider_name: &str,
        code: String,
    ) -> Result<OAuthCredentials> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| Error::Other(format!("Unknown provider: {}", provider_name)))?;

        let client = BasicClient::new(
            ClientId::new(provider.client_id.clone()),
            Some(ClientSecret::new(provider.client_secret.clone())),
            AuthUrl::new(provider.auth_url.clone())?,
            Some(TokenUrl::new(provider.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(provider.redirect_uri.clone())?);

        let token = client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(async_http_client)
            .await
            .map_err(|e| Error::Other(format!("Token exchange failed: {}", e)))?;

        let credentials = OAuthCredentials {
            provider: provider_name.to_string(),
            access_token: token.access_token().secret().to_string(),
            refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
            expires_at: token.expires_in().map(|d| {
                Utc::now() + Duration::seconds(d.as_secs() as i64)
            }),
            scopes: provider.scopes.clone(),
        };

        // Store in memory cache
        let mut tokens = self.tokens.write().await;
        tokens.insert(provider_name.to_string(), credentials.clone());

        // Store in database
        self.store_credentials(&credentials).await?;

        Ok(credentials)
    }

    /// Refresh an access token
    pub async fn refresh_token(&self, provider_name: &str) -> Result<OAuthCredentials> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| Error::Other(format!("Unknown provider: {}", provider_name)))?;

        // Get current credentials
        let tokens = self.tokens.read().await;
        let current = tokens.get(provider_name)
            .ok_or_else(|| Error::Other(format!("No credentials for: {}", provider_name)))?;

        let refresh_token = current.refresh_token.as_ref()
            .ok_or_else(|| Error::Other("No refresh token available".to_string()))?;

        let client = BasicClient::new(
            ClientId::new(provider.client_id.clone()),
            Some(ClientSecret::new(provider.client_secret.clone())),
            AuthUrl::new(provider.auth_url.clone())?,
            Some(TokenUrl::new(provider.token_url.clone())?),
        );

        let token = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
            .request_async(async_http_client)
            .await
            .map_err(|e| Error::Other(format!("Token refresh failed: {}", e)))?;

        let new_credentials = OAuthCredentials {
            provider: provider_name.to_string(),
            access_token: token.access_token().secret().to_string(),
            refresh_token: token.refresh_token()
                .map(|t| t.secret().to_string())
                .or_else(|| Some(refresh_token.clone())),
            expires_at: token.expires_in().map(|d| {
                Utc::now() + Duration::seconds(d.as_secs() as i64)
            }),
            scopes: current.scopes.clone(),
        };

        drop(tokens); // Release read lock

        // Update cache
        let mut tokens = self.tokens.write().await;
        tokens.insert(provider_name.to_string(), new_credentials.clone());

        // Update database
        self.store_credentials(&new_credentials).await?;

        Ok(new_credentials)
    }

    /// Get valid access token (refreshing if needed)
    pub async fn get_valid_token(&self, provider_name: &str) -> Result<String> {
        let tokens = self.tokens.read().await;

        if let Some(creds) = tokens.get(provider_name) {
            // Check if token is expired
            if let Some(expires_at) = creds.expires_at {
                if expires_at <= Utc::now() + Duration::minutes(5) {
                    // Token expires soon, refresh it
                    drop(tokens);
                    let new_creds = self.refresh_token(provider_name).await?;
                    return Ok(new_creds.access_token);
                }
            }
            return Ok(creds.access_token.clone());
        }

        drop(tokens);

        // Try to load from database
        let creds = self.load_credentials(provider_name).await?;

        // Check if needs refresh
        if let Some(expires_at) = creds.expires_at {
            if expires_at <= Utc::now() + Duration::minutes(5) {
                let new_creds = self.refresh_token(provider_name).await?;
                return Ok(new_creds.access_token);
            }
        }

        // Cache the loaded credentials
        let mut tokens = self.tokens.write().await;
        tokens.insert(provider_name.to_string(), creds.clone());

        Ok(creds.access_token)
    }

    /// Store credentials in database
    async fn store_credentials(&self, creds: &OAuthCredentials) -> Result<()> {
        // TODO: Encrypt tokens before storing
        let json = serde_json::to_string(creds)?;

        let query = "
            INSERT INTO oauth_credentials (provider, credentials, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (provider) DO UPDATE
            SET credentials = $2, updated_at = NOW()
        ";

        self.db.execute(query, &[&creds.provider, &json]).await?;
        Ok(())
    }

    /// Load credentials from database
    async fn load_credentials(&self, provider_name: &str) -> Result<OAuthCredentials> {
        let query = "
            SELECT credentials FROM oauth_credentials
            WHERE provider = $1
        ";

        let results = self.db.query(query).await?;

        if results.is_empty() {
            return Err(Error::Other(format!("No credentials for: {}", provider_name)));
        }

        let json_str = results[0].get("credentials")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Invalid credentials format".to_string()))?;

        let creds: OAuthCredentials = serde_json::from_str(json_str)?;
        Ok(creds)
    }

    /// Check which sources need token refresh
    pub async fn get_expiring_tokens(&self, within_minutes: i64) -> Vec<String> {
        let tokens = self.tokens.read().await;
        let threshold = Utc::now() + Duration::minutes(within_minutes);

        tokens.iter()
            .filter(|(_, creds)| {
                creds.expires_at
                    .map(|exp| exp <= threshold)
                    .unwrap_or(false)
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl From<oauth2::url::ParseError> for Error {
    fn from(e: oauth2::url::ParseError) -> Self {
        Error::Other(format!("OAuth URL error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_provider_creation() {
        let provider = OAuthProvider {
            name: "test".to_string(),
            client_id: "client123".to_string(),
            client_secret: "secret456".to_string(),
            auth_url: "https://auth.example.com".to_string(),
            token_url: "https://token.example.com".to_string(),
            scopes: vec!["read".to_string()],
            redirect_uri: "http://localhost/callback".to_string(),
        };

        assert_eq!(provider.name, "test");
        assert_eq!(provider.scopes.len(), 1);
    }
}