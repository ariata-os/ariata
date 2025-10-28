//! Notion-specific error handling
//!
//! Implements custom error classification for Notion APIs,
//! including rate limiting and OAuth token errors.

use crate::sources::base::error_handler::{ErrorClass, ErrorHandler};
use reqwest::StatusCode;

/// Notion API error handler
///
/// Handles Notion-specific error cases:
/// - 401 errors indicate invalid or expired OAuth tokens
/// - 429 errors indicate rate limiting
/// - Notion uses standard HTTP error codes
pub struct NotionErrorHandler;

impl ErrorHandler for NotionErrorHandler {
    fn should_retry(&self, status: StatusCode, attempt: u32, max_retries: u32) -> bool {
        if attempt >= max_retries {
            return false;
        }

        match status.as_u16() {
            // Retry auth errors (will trigger token refresh)
            401 => true,

            // Retry rate limits with backoff
            429 => true,

            // Retry server errors
            500..=599 => true,

            // Don't retry other errors
            _ => false,
        }
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
    fn test_notion_error_classification() {
        let handler = NotionErrorHandler;

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
    fn test_notion_should_retry() {
        let handler = NotionErrorHandler;

        // Should retry auth, rate limit, server errors
        assert!(handler.should_retry(StatusCode::UNAUTHORIZED, 0, 3));
        assert!(handler.should_retry(StatusCode::TOO_MANY_REQUESTS, 0, 3));
        assert!(handler.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 0, 3));

        // Should NOT retry client errors
        assert!(!handler.should_retry(StatusCode::BAD_REQUEST, 0, 3));
        assert!(!handler.should_retry(StatusCode::NOT_FOUND, 0, 3));

        // Should NOT retry when max retries reached
        assert!(!handler.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 3, 3));
    }
}
