//! Error types for Ariata

use thiserror::Error;

/// Main error type for Ariata
#[derive(Debug, Error)]
pub enum Error {
    /// Database-related errors
    #[error("Database error: {0}")]
    Database(String),

    /// Storage-related errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Source-related errors
    #[error("Source error: {0}")]
    Source(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Unauthorized access
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// HTTP errors
    #[error("HTTP error: {0}")]
    Http(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SQL errors
    #[error("SQL error: {0}")]
    Sql(#[from] sqlx::Error),

    /// S3 errors
    #[error("S3 error: {0}")]
    S3(String),

    /// Reqwest HTTP client errors
    #[error("HTTP client error: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// Generic errors
    #[error("{0}")]
    Other(String),
}

/// Result type alias for Ariata operations
pub type Result<T> = std::result::Result<T, Error>;