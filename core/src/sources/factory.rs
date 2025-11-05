//! Stream factory for creating stream instances at runtime
//!
//! This module provides a unified way to instantiate any stream based on
//! source type and stream name, handling authentication and configuration loading.

use std::sync::Arc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};
use super::base::TokenManager;

use super::{
    auth::SourceAuth,
    stream::Stream,
};

/// Factory for creating stream instances
///
/// The StreamFactory handles:
/// - Loading source information from the database
/// - Creating appropriate authentication (OAuth2, Device, etc.)
/// - Instantiating the correct stream implementation
///
/// # Example
///
/// ```rust
/// let factory = StreamFactory::new(db.clone());
///
/// // Create a Google Calendar stream
/// let mut stream = factory.create_stream(source_id, "calendar").await?;
///
/// // Load config and sync
/// stream.load_config(&db, source_id).await?;
/// let result = stream.sync(SyncMode::auto()).await?;
/// ```
pub struct StreamFactory {
    db: PgPool,
}

impl StreamFactory {
    /// Create a new stream factory
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Create a stream instance for syncing
    ///
    /// This looks up the source in the database, creates the appropriate
    /// authentication, and instantiates the correct stream implementation.
    ///
    /// # Arguments
    /// * `source_id` - UUID of the source
    /// * `stream_name` - Name of the stream (e.g., "calendar", "gmail")
    ///
    /// # Returns
    /// A boxed Stream trait object ready for syncing
    ///
    /// # Errors
    /// - If the source doesn't exist
    /// - If the source type is unknown
    /// - If the stream name is not supported for this source
    pub async fn create_stream(
        &self,
        source_id: Uuid,
        stream_name: &str,
    ) -> Result<Box<dyn Stream>> {
        // Load source info from database
        let source = self.load_source(source_id).await?;

        // Create auth abstraction
        let auth = self.create_auth(source_id, &source.provider).await?;

        // Create the appropriate stream implementation
        self.create_stream_impl(source_id, &source.provider, stream_name, auth)
            .await
    }

    /// Load source information from the database
    async fn load_source(&self, source_id: Uuid) -> Result<SourceInfo> {
        let result = sqlx::query_as::<_, (String, String)>(
            "SELECT provider, name FROM sources WHERE id = $1 AND is_active = true"
        )
        .bind(source_id)
        .fetch_optional(&self.db)
        .await?;

        match result {
            Some((provider, name)) => Ok(SourceInfo { provider, name }),
            None => Err(Error::Database(format!(
                "Source not found or inactive: {}",
                source_id
            ))),
        }
    }

    /// Create authentication for a source
    async fn create_auth(&self, source_id: Uuid, provider: &str) -> Result<SourceAuth> {
        match provider {
            "google" | "strava" | "notion" => {
                // Create a new TokenManager (requires encryption key)
                let token_manager = Arc::new(TokenManager::new(self.db.clone())?);
                Ok(SourceAuth::oauth2(source_id, token_manager))
            }
            "ios" | "mac" => {
                // Device sources don't use traditional auth - they push data
                // The device_id is the source name
                let source = self.load_source(source_id).await?;
                Ok(SourceAuth::device(source.name))
            }
            "ariata_app" => {
                // Internal source - no external authentication needed
                // Uses device auth pattern with internal name
                Ok(SourceAuth::device("ariata_app_internal".to_string()))
            }
            _ => Err(Error::Other(format!("Unknown provider: {}", provider))),
        }
    }

    /// Create the stream implementation
    ///
    /// This is where we match on provider + stream name and instantiate
    /// the correct struct. As we refactor sources to use the Stream trait,
    /// they'll be added here.
    async fn create_stream_impl(
        &self,
        source_id: Uuid,
        provider: &str,
        stream_name: &str,
        auth: SourceAuth,
    ) -> Result<Box<dyn Stream>> {
        match (provider, stream_name) {
            // Internal streams
            ("ariata_app", "app_export") => {
                use crate::sources::ariata_app::AppChatExportStream;
                Ok(Box::new(AppChatExportStream::new(self.db.clone(), source_id)))
            }

            // Google streams
            ("google", "calendar") => {
                use crate::sources::google::calendar::GoogleCalendarStream;
                Ok(Box::new(GoogleCalendarStream::new(source_id, self.db.clone(), auth)))
            }
            ("google", "gmail") => {
                use crate::sources::google::gmail::GoogleGmailStream;
                Ok(Box::new(GoogleGmailStream::new(source_id, self.db.clone(), auth)))
            }

            // Notion streams
            ("notion", "pages") => {
                use crate::sources::notion::NotionPagesStream;
                Ok(Box::new(NotionPagesStream::new(source_id, self.db.clone(), auth)))
            }

            // iOS streams
            ("ios", "healthkit") | ("ios", "location") | ("ios", "microphone") => {
                // TODO: Refactor iOS processors to implement Stream trait
                Err(Error::Other(
                    "iOS streams not yet refactored to new pattern".to_string(),
                ))
            }

            // Mac streams
            ("mac", "apps") | ("mac", "browser") | ("mac", "imessage") => {
                // TODO: Refactor Mac processors to implement Stream trait
                Err(Error::Other(
                    "Mac streams not yet refactored to new pattern".to_string(),
                ))
            }

            _ => Err(Error::Other(format!(
                "Unknown stream: {}/{}",
                provider, stream_name
            ))),
        }
    }

}

impl Clone for StreamFactory {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

/// Source information loaded from database
struct SourceInfo {
    provider: String,
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_creation() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let _factory = StreamFactory::new(pool);
    }

    #[tokio::test]
    async fn test_factory_clone() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let factory = StreamFactory::new(pool);
        let _factory2 = factory.clone();
    }

    #[tokio::test]
    async fn test_create_auth_oauth2() {
        // Set insecure mode for testing
        std::env::set_var("ARIATA_ALLOW_INSECURE", "true");

        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let factory = StreamFactory::new(pool);

        let auth = factory.create_auth(Uuid::new_v4(), "google").await;
        assert!(auth.is_ok());

        let auth = auth.unwrap();
        assert!(auth.is_oauth());
        assert!(!auth.is_device());

        // Clean up
        std::env::remove_var("ARIATA_ALLOW_INSECURE");
    }

    #[tokio::test]
    async fn test_create_auth_device() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let factory = StreamFactory::new(pool);

        // Note: This will fail without a source in the database
        // That's expected - this tests the code path, not the database
        let result = factory.create_auth(Uuid::new_v4(), "ios").await;

        // Should return error because source doesn't exist
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_auth_unknown_source() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let factory = StreamFactory::new(pool);

        let result = factory.create_auth(Uuid::new_v4(), "unknown_source").await;
        assert!(result.is_err());

        match result {
            Err(Error::Other(msg)) => assert!(msg.contains("Unknown source type")),
            _ => panic!("Expected Error::Other"),
        }
    }

    #[tokio::test]
    async fn test_create_stream_impl_google_calendar() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let factory = StreamFactory::new(pool.clone());

        let source_id = Uuid::new_v4();
        let tm = Arc::new(TokenManager::new_insecure(pool));
        let auth = SourceAuth::oauth2(source_id, tm);

        let result = factory
            .create_stream_impl(source_id, "google", "calendar", auth)
            .await;

        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.stream_name(), "calendar");
        assert_eq!(stream.source_name(), "google");
        assert_eq!(stream.table_name(), "stream_google_calendar");
    }

    #[tokio::test]
    async fn test_create_stream_impl_unknown_stream() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let factory = StreamFactory::new(pool.clone());

        let source_id = Uuid::new_v4();
        let tm = Arc::new(TokenManager::new_insecure(pool));
        let auth = SourceAuth::oauth2(source_id, tm);

        let result = factory
            .create_stream_impl(source_id, "google", "nonexistent", auth)
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Other(msg)) => assert!(msg.contains("Unknown stream")),
            _ => panic!("Expected Error::Other"),
        }
    }
}
