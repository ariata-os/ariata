//! Transform context providing dependencies for transform jobs
//!
//! This module defines the TransformContext which bundles all external dependencies
//! (storage, API keys, stream reader) needed by transform jobs.

use crate::error::{Error, Result};
use crate::sources::base::TransformDataSource;
use crate::storage::{stream_writer::StreamWriter, Storage};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Context providing dependencies for transform jobs
///
/// This context is passed to transform job executors and makes external
/// dependencies available to transforms that need them.
///
/// # Data Source
///
/// The `memory_data_source` is optional because it's only set when creating
/// transform jobs that use the hot path (direct memory access). For sync jobs
/// and ingest endpoints, the initial context is created without a data source,
/// and the transform trigger logic creates a new context with the actual data source.
///
/// # Metadata
///
/// The `metadata` field allows passing configuration parameters to transforms.
/// This is useful for testing with seed data or adjusting transform behavior.
///
/// Example:
/// ```json
/// {
///   "lookback_hours": 168  // Process last 7 days for location clustering
/// }
/// ```
#[derive(Clone)]
pub struct TransformContext {
    /// Object storage (S3/MinIO) for file access and presigned URLs
    pub storage: Arc<Storage>,

    /// Stream writer for writing stream data to object storage
    pub stream_writer: Arc<Mutex<StreamWriter>>,

    /// In-memory data source for direct transforms (hot path)
    ///
    /// This is `None` for initial contexts created by sync/ingest jobs,
    /// and `Some(...)` when the transform job is actually executed.
    pub memory_data_source: Option<Arc<dyn TransformDataSource>>,

    /// API keys for external services
    pub api_keys: ApiKeys,

    /// Configuration metadata for transforms
    ///
    /// Use this to pass parameters like lookback windows, thresholds, etc.
    /// Defaults to empty JSON object.
    pub metadata: serde_json::Value,
}

impl TransformContext {
    /// Create a new transform context without a data source
    ///
    /// Use this for sync jobs and ingest endpoints. The data source will be
    /// set later by `create_transform_job_for_stream` when the transform is triggered.
    pub fn new(
        storage: Arc<Storage>,
        stream_writer: Arc<Mutex<StreamWriter>>,
        api_keys: ApiKeys,
    ) -> Self {
        Self {
            storage,
            stream_writer,
            memory_data_source: None,
            api_keys,
            metadata: serde_json::json!({}),
        }
    }

    /// Create a context with an in-memory data source (hot path)
    ///
    /// Use this when you have records immediately available and want to
    /// trigger a direct transform without S3 round-trip.
    pub fn with_data_source(
        storage: Arc<Storage>,
        stream_writer: Arc<Mutex<StreamWriter>>,
        memory_data_source: Arc<dyn TransformDataSource>,
        api_keys: ApiKeys,
    ) -> Self {
        Self {
            storage,
            stream_writer,
            memory_data_source: Some(memory_data_source),
            api_keys,
            metadata: serde_json::json!({}),
        }
    }

    /// Create a context with custom metadata
    ///
    /// Use this to pass configuration parameters to transforms.
    /// Useful for testing with seed data or adjusting transform behavior.
    ///
    /// # Example
    /// ```ignore
    /// let metadata = serde_json::json!({
    ///     "lookback_hours": 168  // Process last 7 days
    /// });
    /// let context = TransformContext::with_metadata(storage, stream_writer, api_keys, metadata);
    /// ```
    pub fn with_metadata(
        storage: Arc<Storage>,
        stream_writer: Arc<Mutex<StreamWriter>>,
        api_keys: ApiKeys,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            storage,
            stream_writer,
            memory_data_source: None,
            api_keys,
            metadata,
        }
    }

    /// Get the data source for transforms if available
    ///
    /// Returns `None` if no data source has been set (e.g., for cold path transforms
    /// that need to read from S3).
    pub fn get_data_source(&self) -> Option<Arc<dyn TransformDataSource>> {
        if let Some(ref data_source) = self.memory_data_source {
            tracing::info!("Using memory data source for direct transform (hot path)");
            Some(data_source.clone())
        } else {
            tracing::debug!("No memory data source available (cold path or not set)");
            None
        }
    }
}

/// API keys for external services
///
/// All keys are optional since not all transforms need all keys.
/// Use the `*_required()` methods to get a key or return an error
/// if it's not configured.
#[derive(Clone)]
pub struct ApiKeys {
    /// AssemblyAI API key for audio transcription
    pub assemblyai: Option<String>,

    /// Anthropic API key for Claude semantic parsing
    pub anthropic: Option<String>,
}

impl ApiKeys {
    /// Load API keys from environment variables
    ///
    /// Missing keys are set to None - this allows the system to start
    /// even if some keys are not configured. Transforms that require
    /// specific keys will error at runtime if the key is missing.
    pub fn from_env() -> Self {
        Self {
            assemblyai: std::env::var("ASSEMBLYAI_API_KEY").ok(),
            anthropic: std::env::var("ANTHROPIC_API_KEY").ok(),
        }
    }

    /// Get AssemblyAI API key or return error if not configured
    ///
    /// Use this in transforms that require AssemblyAI access.
    ///
    /// # Example
    /// ```ignore
    /// let api_key = context.api_keys.assemblyai_required()?;
    /// let client = AssemblyAIClient::new(api_key.to_string());
    /// ```
    pub fn assemblyai_required(&self) -> Result<&str> {
        self.assemblyai.as_deref().ok_or_else(|| {
            Error::Configuration(
                "ASSEMBLYAI_API_KEY not set - required for audio transcription".into(),
            )
        })
    }

    /// Get Anthropic API key or return error if not configured
    ///
    /// Use this in transforms that require Claude API access.
    pub fn anthropic_required(&self) -> Result<&str> {
        self.anthropic.as_deref().ok_or_else(|| {
            Error::Configuration("ANTHROPIC_API_KEY not set - required for semantic parsing".into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_keys_from_env() {
        // Set test environment variable
        std::env::set_var("ASSEMBLYAI_API_KEY", "test-assemblyai-key");
        std::env::set_var("ANTHROPIC_API_KEY", "test-anthropic-key");

        let keys = ApiKeys::from_env();

        assert_eq!(keys.assemblyai, Some("test-assemblyai-key".to_string()));
        assert_eq!(keys.anthropic, Some("test-anthropic-key".to_string()));

        // Cleanup
        std::env::remove_var("ASSEMBLYAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_api_keys_missing_from_env() {
        // Ensure vars are not set
        std::env::remove_var("ASSEMBLYAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");

        let keys = ApiKeys::from_env();

        assert_eq!(keys.assemblyai, None);
        assert_eq!(keys.anthropic, None);
    }

    #[test]
    fn test_assemblyai_required_success() {
        let keys = ApiKeys {
            assemblyai: Some("test-key".to_string()),
            anthropic: None,
        };

        let result = keys.assemblyai_required();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-key");
    }

    #[test]
    fn test_assemblyai_required_missing() {
        let keys = ApiKeys {
            assemblyai: None,
            anthropic: None,
        };

        let result = keys.assemblyai_required();
        assert!(result.is_err());

        match result {
            Err(Error::Configuration(msg)) => {
                assert!(msg.contains("ASSEMBLYAI_API_KEY"));
            }
            _ => panic!("Expected Configuration error"),
        }
    }

    #[test]
    fn test_anthropic_required_success() {
        let keys = ApiKeys {
            assemblyai: None,
            anthropic: Some("test-claude-key".to_string()),
        };

        let result = keys.anthropic_required();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-claude-key");
    }
}
