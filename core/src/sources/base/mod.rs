//! Base infrastructure and utilities for all sources

use serde::{de::DeserializeOwned, Serialize};

pub mod device;
pub mod error_handler;
pub mod oauth;
pub mod oauth_client;
pub mod sync_mode;
pub mod sync_strategy;
pub mod transform;
pub mod transform_data_source;
pub mod validation;

/// Trait for stream configuration serialization
///
/// Provides default implementations for converting between Rust structs
/// and JSON values for database storage. Any type that implements
/// `Serialize + DeserializeOwned` automatically gets these methods.
pub trait ConfigSerializable: Serialize + DeserializeOwned {
    /// Deserialize config from JSON value (from database)
    fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// Serialize config to JSON value (for database storage)
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }
}

// Blanket implementation: any type that can be serialized/deserialized gets these methods
impl<T: Serialize + DeserializeOwned> ConfigSerializable for T {}

pub use device::get_or_create_device_source;
pub use error_handler::{DefaultErrorHandler, ErrorClass, ErrorHandler};
pub use oauth::{OAuthProxyConfig, OAuthToken, TokenEncryptor, TokenManager};
pub use oauth_client::{OAuthHttpClient, RetryConfig};
pub use sync_mode::{SyncMode, SyncResult};
pub use sync_strategy::SyncStrategy;
pub use transform::{ChainedTransform, OntologyTransform, TransformResult};
pub use transform_data_source::{DataSourceType, MemoryDataSource, TransformDataSource};
pub use validation::*;
