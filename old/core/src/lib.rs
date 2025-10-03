//! Ariata - Open Source Personal Data Ecosystem
//!
//! High-performance data pipeline for personal data collection, storage, and analysis.

pub mod client;
pub mod database;
pub mod storage;
pub mod streams;
pub mod pipeline;
pub mod server;
pub mod error;
pub mod oauth;
pub mod scheduler;
pub mod sources;

// Re-export main types
pub use client::{Ariata, AriataBuilder};
pub use error::{Error, Result};

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