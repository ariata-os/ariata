//! Error handling abstractions for OAuth HTTP clients
//!
//! This module provides customizable error handling for different OAuth providers.
//! Each provider can implement their own error classification and retry logic.

use reqwest::StatusCode;

/// Classification of HTTP errors for retry logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Authentication error (401) - token might need refresh
    AuthError,

    /// Rate limiting (429) - need to back off and retry
    RateLimit,

    /// Sync token invalid (usually 410) - specific to incremental sync APIs
    SyncTokenError,

    /// Server error (5xx) - temporary, should retry
    ServerError,

    /// Client error (4xx) - permanent, should not retry
    ClientError,

    /// Network/connection error - should retry
    NetworkError,
}

/// Provider-specific error handling logic
pub trait ErrorHandler: Send + Sync {
    /// Determine if a request should be retried based on the error
    ///
    /// # Arguments
    /// * `status` - HTTP status code
    /// * `attempt` - Current retry attempt (0-indexed)
    /// * `max_retries` - Maximum number of retries configured
    ///
    /// # Returns
    /// `true` if the request should be retried, `false` otherwise
    fn should_retry(&self, status: StatusCode, attempt: u32, max_retries: u32) -> bool {
        if attempt >= max_retries {
            return false;
        }

        // Default retry logic for common cases
        matches!(status.as_u16(), 401 | 429 | 500..=599)
    }

    /// Classify an error based on status code and response body
    ///
    /// # Arguments
    /// * `status` - HTTP status code
    /// * `body` - Response body (for detailed error inspection)
    ///
    /// # Returns
    /// The classified error type
    fn classify_error(&self, status: StatusCode, body: &str) -> ErrorClass {
        match status.as_u16() {
            401 => ErrorClass::AuthError,
            429 => ErrorClass::RateLimit,
            400..=499 => ErrorClass::ClientError,
            500..=599 => ErrorClass::ServerError,
            _ => ErrorClass::ClientError,
        }
    }

    /// Check if an error message indicates a sync token error
    ///
    /// This is used for incremental sync APIs that use sync tokens.
    /// Different providers have different error messages for invalid tokens.
    fn is_sync_token_error(&self, _status: StatusCode, _body: &str) -> bool {
        false
    }
}

/// Default error handler with sensible retry logic
pub struct DefaultErrorHandler;

impl ErrorHandler for DefaultErrorHandler {
    fn should_retry(&self, status: StatusCode, attempt: u32, max_retries: u32) -> bool {
        if attempt >= max_retries {
            return false;
        }

        // Retry on auth errors (will trigger token refresh), rate limits, and server errors
        matches!(status.as_u16(), 401 | 429 | 500..=599)
    }

    fn classify_error(&self, status: StatusCode, _body: &str) -> ErrorClass {
        match status.as_u16() {
            401 => ErrorClass::AuthError,
            429 => ErrorClass::RateLimit,
            400..=499 => ErrorClass::ClientError,
            500..=599 => ErrorClass::ServerError,
            _ => ErrorClass::ClientError,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_error_handler_classification() {
        let handler = DefaultErrorHandler;

        assert_eq!(
            handler.classify_error(StatusCode::UNAUTHORIZED, ""),
            ErrorClass::AuthError
        );

        assert_eq!(
            handler.classify_error(StatusCode::TOO_MANY_REQUESTS, ""),
            ErrorClass::RateLimit
        );

        assert_eq!(
            handler.classify_error(StatusCode::BAD_REQUEST, ""),
            ErrorClass::ClientError
        );

        assert_eq!(
            handler.classify_error(StatusCode::INTERNAL_SERVER_ERROR, ""),
            ErrorClass::ServerError
        );
    }

    #[test]
    fn test_default_should_retry() {
        let handler = DefaultErrorHandler;

        // Should retry on 401, 429, 5xx
        assert!(handler.should_retry(StatusCode::UNAUTHORIZED, 0, 3));
        assert!(handler.should_retry(StatusCode::TOO_MANY_REQUESTS, 0, 3));
        assert!(handler.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 0, 3));

        // Should not retry on 4xx (except 401, 429)
        assert!(!handler.should_retry(StatusCode::BAD_REQUEST, 0, 3));
        assert!(!handler.should_retry(StatusCode::NOT_FOUND, 0, 3));

        // Should not retry when max retries reached
        assert!(!handler.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 3, 3));
    }

    #[test]
    fn test_default_sync_token_error() {
        let handler = DefaultErrorHandler;

        // Default handler doesn't detect sync token errors
        assert!(!handler.is_sync_token_error(StatusCode::GONE, "Sync token invalid"));
    }
}
