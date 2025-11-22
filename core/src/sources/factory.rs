//! Stream factory for creating stream instances at runtime
//!
//! This module provides a unified way to instantiate any stream based on
//! source type and stream name, handling authentication and configuration loading.

use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::base::TokenManager;
use crate::error::{Error, Result};
use crate::storage::{stream_writer::StreamWriter, Storage};

use super::{auth::SourceAuth, stream_type::StreamType};

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
/// let factory = StreamFactory::new(db.clone(), storage.clone(), stream_writer.clone());
///
/// // Create a Google Calendar stream (returns StreamType::Pull)
/// let mut stream_type = factory.create_stream_typed(source_id, "calendar").await?;
///
/// // Get mutable access to the PullStream
/// if let Some(pull_stream) = stream_type.as_pull_mut() {
///     pull_stream.load_config(&db, source_id).await?;
///     let result = pull_stream.sync_pull(SyncMode::incremental(None)).await?;
/// }
/// ```
pub struct StreamFactory {
    db: PgPool,
    storage: Arc<Storage>,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl StreamFactory {
    /// Create a new stream factory
    pub fn new(db: PgPool, storage: Arc<Storage>, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self {
            db,
            storage,
            stream_writer,
        }
    }

    /// Create a stream instance with the new StreamType enum
    ///
    /// This looks up the source in the database, creates the appropriate
    /// authentication, and instantiates the correct stream implementation
    /// wrapped in either Pull or Push variant.
    ///
    /// # Arguments
    /// * `source_id` - UUID of the source
    /// * `stream_name` - Name of the stream (e.g., "calendar", "gmail", "apps")
    ///
    /// # Returns
    /// A StreamType enum (either Pull or Push) containing the stream
    ///
    /// # Errors
    /// - If the source doesn't exist
    /// - If the source type is unknown
    /// - If the stream name is not supported for this source
    pub async fn create_stream_typed(
        &self,
        source_id: Uuid,
        stream_name: &str,
    ) -> Result<StreamType> {
        // Load source info from database
        let source = self.load_source(source_id).await?;

        // Create auth abstraction
        let auth = self.create_auth(source_id, &source.source).await?;

        // Create the appropriate stream implementation
        self.create_stream_typed_impl(source_id, &source.source, stream_name, auth)
            .await
    }

    /// Load source information from the database
    async fn load_source(&self, source_id: Uuid) -> Result<SourceInfo> {
        let result = sqlx::query_as::<_, (String, String)>(
            "SELECT source, name FROM source_connections WHERE id = $1 AND is_active = true",
        )
        .bind(source_id)
        .fetch_optional(&self.db)
        .await?;

        match result {
            Some((source, name)) => Ok(SourceInfo { source, name }),
            None => Err(Error::Database(format!(
                "Source not found or inactive: {}",
                source_id
            ))),
        }
    }

    /// Create authentication for a source
    async fn create_auth(&self, source_id: Uuid, provider: &str) -> Result<SourceAuth> {
        match provider {
            "google" | "notion" | "plaid" => {
                // OAuth2 sources - create TokenManager for token refresh
                let token_manager = Arc::new(TokenManager::new(self.db.clone())?);
                Ok(SourceAuth::oauth2(source_id, token_manager))
            }
            "ios" | "mac" => {
                // Device sources don't use traditional auth - they push data
                // The device_id is the source name
                let source = self.load_source(source_id).await?;
                Ok(SourceAuth::device(source.name))
            }
            "ariata" => {
                // Internal source - no external authentication needed
                // Uses device auth pattern with internal name
                Ok(SourceAuth::device("ariata_internal".to_string()))
            }
            _ => Err(Error::Other(format!("Unknown provider: {}", provider))),
        }
    }

    /// Create the stream implementation with StreamType enum
    ///
    /// This is where we match on provider + stream name and instantiate
    /// the correct struct wrapped in StreamType::Pull or StreamType::Push.
    ///
    /// All streams now implement either PullStream (backend-initiated) or
    /// PushStream (client-initiated) traits for clear architectural boundaries.
    async fn create_stream_typed_impl(
        &self,
        _source_id: Uuid,
        provider: &str,
        stream_name: &str,
        _auth: SourceAuth,
    ) -> Result<StreamType> {
        match (provider, stream_name) {
            // Pull streams (backend-initiated from external APIs)
            ("google", "calendar") => {
                use crate::sources::google::calendar::GoogleCalendarStream;
                Ok(StreamType::Pull(Box::new(GoogleCalendarStream::new(
                    _source_id,
                    self.db.clone(),
                    self.stream_writer.clone(),
                    _auth,
                ))))
            }
            ("google", "gmail") => {
                use crate::sources::google::gmail::GoogleGmailStream;
                Ok(StreamType::Pull(Box::new(GoogleGmailStream::new(
                    _source_id,
                    self.db.clone(),
                    self.stream_writer.clone(),
                    _auth,
                ))))
            }
            ("notion", "pages") => {
                use crate::sources::notion::NotionPagesStream;
                Ok(StreamType::Pull(Box::new(NotionPagesStream::new(
                    _source_id,
                    self.db.clone(),
                    self.stream_writer.clone(),
                    _auth,
                ))))
            }
            ("ariata", "app_export") => {
                use crate::sources::ariata::AppChatExportStream;
                Ok(StreamType::Pull(Box::new(AppChatExportStream::new(
                    self.db.clone(),
                    _source_id,
                    self.stream_writer.clone(),
                ))))
            }

            // Push streams (client-initiated from devices)
            ("mac", "apps") => {
                use crate::sources::mac::MacAppsStream;
                Ok(StreamType::Push(Box::new(MacAppsStream::new(
                    self.db.clone(),
                    self.stream_writer.clone(),
                ))))
            }
            ("mac", "imessage") => {
                use crate::sources::mac::MacIMessageStream;
                Ok(StreamType::Push(Box::new(MacIMessageStream::new(
                    self.db.clone(),
                    self.stream_writer.clone(),
                ))))
            }
            ("mac", "browser") => {
                use crate::sources::mac::MacBrowserStream;
                Ok(StreamType::Push(Box::new(MacBrowserStream::new(
                    self.db.clone(),
                    self.stream_writer.clone(),
                ))))
            }
            ("ios", "location") => {
                use crate::sources::ios::IosLocationStream;
                Ok(StreamType::Push(Box::new(IosLocationStream::new(
                    self.db.clone(),
                    self.stream_writer.clone(),
                ))))
            }
            ("ios", "healthkit") => {
                use crate::sources::ios::IosHealthKitStream;
                Ok(StreamType::Push(Box::new(IosHealthKitStream::new(
                    self.db.clone(),
                    self.stream_writer.clone(),
                ))))
            }
            ("ios", "microphone") => {
                use crate::sources::ios::IosMicrophoneStream;
                Ok(StreamType::Push(Box::new(IosMicrophoneStream::new(
                    self.db.clone(),
                    self.storage.clone(),
                    self.stream_writer.clone(),
                ))))
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
            storage: self.storage.clone(),
            stream_writer: self.stream_writer.clone(),
        }
    }
}

/// Source information loaded from database
struct SourceInfo {
    source: String,
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stream_writer::StreamWriter;

    #[tokio::test]
    async fn test_factory_creation() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let _factory = StreamFactory::new(pool, storage, stream_writer);
    }

    #[tokio::test]
    async fn test_factory_clone() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let factory = StreamFactory::new(pool, storage, stream_writer);
        let _factory2 = factory.clone();
    }

    #[tokio::test]
    async fn test_create_auth_oauth2() {
        // Set insecure mode for testing
        std::env::set_var("ARIATA_ALLOW_INSECURE", "true");

        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let factory = StreamFactory::new(pool, storage, stream_writer);

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
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let factory = StreamFactory::new(pool, storage, stream_writer);

        // Note: This will fail without a source in the database
        // That's expected - this tests the code path, not the database
        let result = factory.create_auth(Uuid::new_v4(), "ios").await;

        // Should return error because source doesn't exist
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_auth_unknown_source() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let factory = StreamFactory::new(pool, storage, stream_writer);

        let result = factory.create_auth(Uuid::new_v4(), "unknown_source").await;
        assert!(result.is_err());

        match result {
            Err(Error::Other(msg)) => assert!(msg.contains("Unknown provider")),
            _ => panic!("Expected Error::Other"),
        }
    }

    #[tokio::test]
    async fn test_create_stream_typed_impl_google_calendar() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let factory = StreamFactory::new(pool.clone(), storage, stream_writer.clone());

        let source_id = Uuid::new_v4();
        let tm = Arc::new(TokenManager::new_insecure(pool));
        let auth = SourceAuth::oauth2(source_id, tm);

        let result = factory
            .create_stream_typed_impl(source_id, "google", "calendar", auth)
            .await;

        assert!(result.is_ok());
        let stream_type = result.unwrap();
        assert_eq!(stream_type.stream_name(), "calendar");
        assert_eq!(stream_type.source_name(), "google");
        assert_eq!(stream_type.table_name(), "stream_google_calendar");
        assert!(stream_type.is_pull());
    }

    #[tokio::test]
    async fn test_create_stream_typed_impl_unknown_stream() {
        let pool = PgPool::connect_lazy("postgres://test").unwrap();
        let storage = Arc::new(Storage::local("./test_data".to_string()).unwrap());
        let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
        let factory = StreamFactory::new(pool.clone(), storage, stream_writer.clone());

        let source_id = Uuid::new_v4();
        let tm = Arc::new(TokenManager::new_insecure(pool));
        let auth = SourceAuth::oauth2(source_id, tm);

        let result = factory
            .create_stream_typed_impl(source_id, "google", "nonexistent", auth)
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Other(msg)) => assert!(msg.contains("Unknown stream")),
            _ => panic!("Expected Error::Other"),
        }
    }
}
