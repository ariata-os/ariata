//! Google-specific error handling
//!
//! Implements custom error classification for Google APIs, including
//! special handling for sync token errors (410) used in Calendar and Gmail APIs.

use crate::sources::base::error_handler::{ErrorClass, ErrorHandler};
use reqwest::StatusCode;

/// Google API error handler
///
/// Handles Google-specific error cases:
/// - 410 (Gone) errors indicate invalid sync tokens for Calendar/Gmail
/// - 403 errors can indicate quota/permission issues
/// - Retry logic follows Google's best practices
pub struct GoogleErrorHandler;

impl ErrorHandler for GoogleErrorHandler {
    fn should_retry(&self, status: StatusCode, attempt: u32, max_retries: u32) -> bool {
        if attempt >= max_retries {
            return false;
        }

        match status.as_u16() {
            // Retry auth errors (will trigger token refresh)
            401 => true,

            // Retry rate limits with backoff
            429 => true,

            // Don't retry 410 (sync token invalid) - caller should handle
            410 => false,

            // Don't retry quota exceeded
            403 => false,

            // Retry server errors
            500..=599 => true,

            // Don't retry other errors
            _ => false,
        }
    }

    fn classify_error(&self, status: StatusCode, body: &str) -> ErrorClass {
        match status.as_u16() {
            401 => ErrorClass::AuthError,
            429 => ErrorClass::RateLimit,
            410 => ErrorClass::SyncTokenError,
            403 => {
                // Check if it's a quota error
                if body.contains("quotaExceeded") || body.contains("rateLimitExceeded") {
                    ErrorClass::RateLimit
                } else {
                    ErrorClass::ClientError
                }
            }
            400..=499 => ErrorClass::ClientError,
            500..=599 => ErrorClass::ServerError,
            _ => ErrorClass::ClientError,
        }
    }

    fn is_sync_token_error(&self, status: StatusCode, body: &str) -> bool {
        // Google returns 400 OR 410 for invalid sync tokens
        if matches!(status.as_u16(), 400 | 410) {
            // Check body for sync token indicators
            let body_lower = body.to_lowercase();
            return body_lower.contains("sync token")
                || body_lower.contains("invalid sync token")
                || body_lower.contains("sync_token");
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_error_classification() {
        let handler = GoogleErrorHandler;

        assert_eq!(
            handler.classify_error(StatusCode::UNAUTHORIZED, ""),
            ErrorClass::AuthError
        );

        assert_eq!(
            handler.classify_error(StatusCode::TOO_MANY_REQUESTS, ""),
            ErrorClass::RateLimit
        );

        assert_eq!(
            handler.classify_error(StatusCode::GONE, "Sync token invalid"),
            ErrorClass::SyncTokenError
        );

        assert_eq!(
            handler.classify_error(StatusCode::INTERNAL_SERVER_ERROR, ""),
            ErrorClass::ServerError
        );
    }

    #[test]
    fn test_google_should_retry() {
        let handler = GoogleErrorHandler;

        // Should retry auth, rate limit, server errors
        assert!(handler.should_retry(StatusCode::UNAUTHORIZED, 0, 3));
        assert!(handler.should_retry(StatusCode::TOO_MANY_REQUESTS, 0, 3));
        assert!(handler.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 0, 3));

        // Should NOT retry 410 (sync token errors) - let caller handle
        assert!(!handler.should_retry(StatusCode::GONE, 0, 3));

        // Should NOT retry quota errors
        assert!(!handler.should_retry(StatusCode::FORBIDDEN, 0, 3));

        // Should NOT retry when max retries reached
        assert!(!handler.should_retry(StatusCode::INTERNAL_SERVER_ERROR, 3, 3));
    }

    #[test]
    fn test_google_sync_token_detection() {
        let handler = GoogleErrorHandler;

        // 410 status code with sync token message
        assert!(handler.is_sync_token_error(StatusCode::GONE, "sync token invalid"));

        // 400 status code with sync token message (Google also returns this)
        assert!(
            handler.is_sync_token_error(StatusCode::BAD_REQUEST, "Sync token is no longer valid")
        );

        // 400 with sync_token in body
        assert!(handler.is_sync_token_error(
            StatusCode::BAD_REQUEST,
            "{\"error\": \"invalid sync_token\"}"
        ));

        // Not a sync token error - 400 without sync token message
        assert!(!handler.is_sync_token_error(StatusCode::BAD_REQUEST, "Invalid request"));

        // Not a sync token error - different status
        assert!(!handler.is_sync_token_error(StatusCode::FORBIDDEN, "Permission denied"));
    }

    #[test]
    fn test_quota_error_classification() {
        let handler = GoogleErrorHandler;

        // Quota errors should be classified as rate limits
        assert_eq!(
            handler.classify_error(
                StatusCode::FORBIDDEN,
                "{\"error\":{\"errors\":[{\"reason\":\"quotaExceeded\"}]}}"
            ),
            ErrorClass::RateLimit
        );

        // Other 403 errors should be client errors
        assert_eq!(
            handler.classify_error(StatusCode::FORBIDDEN, "Permission denied"),
            ErrorClass::ClientError
        );
    }
}
