//! Base infrastructure and utilities for all sources

pub mod client;
pub mod auth;
pub mod processor;
pub mod storage;
pub mod sync;
pub mod error_handler;
pub mod oauth_client;

pub use client::HttpClient;
pub use auth::{AuthHelper, AuthType};
pub use processor::BaseProcessor;
pub use storage::StorageHelper;
pub use sync::SyncStateManager;
pub use error_handler::{ErrorHandler, ErrorClass, DefaultErrorHandler};
pub use oauth_client::{OAuthHttpClient, RetryConfig};