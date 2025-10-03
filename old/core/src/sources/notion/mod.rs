//! Notion integration

pub mod auth;
pub mod client;
pub mod types;
pub mod pages;

use std::sync::Arc;
use crate::oauth::OAuthManager;

pub use pages::NotionPagesSource;

/// Create a new Notion pages source
pub fn pages_source(oauth: Arc<OAuthManager>) -> NotionPagesSource {
    NotionPagesSource::new(oauth)
}