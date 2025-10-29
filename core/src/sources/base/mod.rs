//! Base infrastructure and utilities for all sources

pub mod device;
pub mod error_handler;
pub mod oauth_client;
pub mod sync_mode;
pub mod sync_strategy;
pub mod validation;

pub use device::get_or_create_device_source;
pub use error_handler::{DefaultErrorHandler, ErrorClass, ErrorHandler};
pub use oauth_client::{OAuthHttpClient, RetryConfig};
pub use sync_mode::{SyncMode, SyncResult};
pub use sync_strategy::SyncStrategy;
pub use validation::*;