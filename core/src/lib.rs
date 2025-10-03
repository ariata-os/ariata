//! Ariata - Open Source Personal Data Ecosystem
//!
//! High-performance data pipeline for personal data collection, storage, and analysis.

pub mod api;
pub mod client;
pub mod database;
pub mod error;
pub mod oauth;
pub mod pipeline;
pub mod scheduler;
pub mod server;
pub mod sources;
pub mod storage;

// Re-export main types
pub use client::{Ariata, AriataBuilder};
pub use error::{Error, Result};

// Re-export library API functions
pub use api::{
    create_google_source_with_refresh_token, exchange_google_oauth_code, generate_google_oauth_url,
    sync_google_calendar, SyncStats,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
