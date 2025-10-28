//! Notion integration

pub mod client;
pub mod config;
pub mod types;
pub mod pages;
pub mod registry;

pub use config::NotionPagesConfig;
pub use pages::NotionPagesStream;