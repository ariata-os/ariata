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

    /// Not found errors
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

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

    /// Anyhow errors (from config loading, etc.)
    #[error("Error: {0}")]
    Anyhow(#[from] anyhow::Error),

    /// Generic errors
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Get HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            Error::Authentication(_) | Error::Unauthorized(_) => 401,
            Error::NotFound(_) => 404,
            Error::InvalidInput(_) => 400,
            Error::Configuration(_) => 503,
            _ => 500,
        }
    }

    /// Check if error should be logged at ERROR level (server errors)
    /// vs WARN level (client errors)
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Error::Database(_)
                | Error::Storage(_)
                | Error::Configuration(_)
                | Error::Network(_)
                | Error::S3(_)
                | Error::Sql(_)
                | Error::Io(_)
                | Error::Anyhow(_)
                | Error::Other(_)
        )
    }
}

/// Result type alias for Ariata operations
pub type Result<T> = std::result::Result<T, Error>;
