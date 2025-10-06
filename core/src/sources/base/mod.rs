//! Base infrastructure and utilities for all sources

pub mod client;
pub mod auth;
pub mod sync_mode;
pub mod sync_logger;
pub mod error_handler;
pub mod oauth_client;
pub mod pagination;
pub mod data_source;
pub mod sync_cursor;

pub use client::HttpClient;
pub use auth::{AuthHelper, AuthType};
pub use sync_mode::{SyncMode, SyncResult};
pub use sync_logger::{SyncLogger, SyncLog};
pub use error_handler::{ErrorHandler, ErrorClass, DefaultErrorHandler};
pub use oauth_client::{OAuthHttpClient, RetryConfig};
pub use pagination::{Paginated, Paginator, paginate};
pub use data_source::{DataSource, HealthStatus, parse_stream_table_name};
pub use sync_cursor::{SyncCursor, TypedCursor, GoogleSyncToken, NotionStartCursor, StravaTimestampCursor};