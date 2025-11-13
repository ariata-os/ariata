//! Base infrastructure and utilities for all sources

pub mod device;
pub mod error_handler;
pub mod oauth;
pub mod oauth_client;
pub mod sync_mode;
pub mod sync_strategy;
pub mod transform;
pub mod transform_data_source;
pub mod validation;

pub use device::get_or_create_device_source;
pub use error_handler::{DefaultErrorHandler, ErrorClass, ErrorHandler};
pub use oauth::{OAuthProxyConfig, OAuthToken, TokenEncryptor, TokenManager};
pub use oauth_client::{OAuthHttpClient, RetryConfig};
pub use sync_mode::{SyncMode, SyncResult};
pub use sync_strategy::SyncStrategy;
pub use transform::{ChainedTransform, OntologyTransform, TransformResult};
pub use transform_data_source::{DataSourceType, MemoryDataSource, TransformDataSource};
pub use validation::*;