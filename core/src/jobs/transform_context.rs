//! Transform context providing dependencies for transform jobs
//!
//! This module defines the TransformContext which bundles all external dependencies
//! (storage, API keys) needed by transform jobs that interact with external services.

use std::sync::Arc;
use crate::error::{Error, Result};
use crate::storage::Storage;

/// Context providing dependencies for transform jobs
///
/// This context is passed to transform job executors and makes external
/// dependencies available to transforms that need them. Transforms that
/// only need database access can ignore this context.
#[derive(Clone)]
pub struct TransformContext {
    /// Object storage (S3/MinIO) for file access and presigned URLs
    pub storage: Arc<Storage>,

    /// API keys for external services
    pub api_keys: ApiKeys,
}

impl TransformContext {
    /// Create a new transform context
    pub fn new(storage: Storage, api_keys: ApiKeys) -> Self {
        Self {
            storage: Arc::new(storage),
            api_keys,
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
        self.assemblyai
            .as_deref()
            .ok_or_else(|| Error::Configuration(
                "ASSEMBLYAI_API_KEY not set - required for audio transcription".into()
            ))
    }

    /// Get Anthropic API key or return error if not configured
    ///
    /// Use this in transforms that require Claude API access.
    pub fn anthropic_required(&self) -> Result<&str> {
        self.anthropic
            .as_deref()
            .ok_or_else(|| Error::Configuration(
                "ANTHROPIC_API_KEY not set - required for semantic parsing".into()
            ))
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
