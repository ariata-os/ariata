//! Type-safe sync cursor abstraction

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Trait for sync cursors
/// Each provider implements their own cursor type (token, timestamp, etc.)
pub trait SyncCursor: Send + Sync + Serialize + for<'de> Deserialize<'de> + Clone {
    /// Create an initial cursor (for first sync)
    fn initial() -> Self;

    /// Parse from database string representation
    fn from_string(s: &str) -> Result<Self>
    where
        Self: Sized;

    /// Convert to database string representation
    fn to_string(&self) -> String;

    /// Check if cursor has expired (default: never expires)
    fn is_expired(&self) -> bool {
        false
    }

    /// Get cursor type identifier
    fn cursor_type() -> &'static str
    where
        Self: Sized;
}

/// Wrapper for storing cursors in database with type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedCursor {
    pub cursor_type: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
}

impl TypedCursor {
    /// Create a new typed cursor
    pub fn new<T: SyncCursor>(cursor: &T) -> Self {
        Self {
            cursor_type: T::cursor_type().to_string(),
            value: cursor.to_string(),
            created_at: Utc::now(),
        }
    }

    /// Parse typed cursor from database JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Error::Serialization)
    }

    /// Convert to database JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Decode into a specific cursor type
    pub fn decode<T: SyncCursor>(&self) -> Result<T> {
        if self.cursor_type != T::cursor_type() {
            return Err(Error::Configuration(format!(
                "Cursor type mismatch: expected {}, got {}",
                T::cursor_type(),
                self.cursor_type
            )));
        }
        T::from_string(&self.value)
    }
}

// ===== Provider-Specific Implementations =====

/// Google sync token (opaque string from Google APIs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSyncToken(pub String);

impl SyncCursor for GoogleSyncToken {
    fn initial() -> Self {
        Self(String::new())
    }

    fn from_string(s: &str) -> Result<Self> {
        Ok(Self(s.to_string()))
    }

    fn to_string(&self) -> String {
        self.0.clone()
    }

    fn cursor_type() -> &'static str {
        "google_sync_token"
    }
}

/// Notion start_cursor (UUID string)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionStartCursor(pub String);

impl SyncCursor for NotionStartCursor {
    fn initial() -> Self {
        Self(String::new())
    }

    fn from_string(s: &str) -> Result<Self> {
        // Validate UUID format if not empty
        if !s.is_empty() {
            Uuid::parse_str(s)
                .map_err(|e| Error::Configuration(format!("Invalid Notion cursor: {e}")))?;
        }
        Ok(Self(s.to_string()))
    }

    fn to_string(&self) -> String {
        self.0.clone()
    }

    fn cursor_type() -> &'static str {
        "notion_start_cursor"
    }
}

/// Strava pagination (Unix timestamp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StravaTimestampCursor {
    pub before: Option<i64>,  // Unix timestamp
    pub after: Option<i64>,   // Unix timestamp
}

impl SyncCursor for StravaTimestampCursor {
    fn initial() -> Self {
        Self {
            before: None,
            after: None,
        }
    }

    fn from_string(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(Error::Serialization)
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn cursor_type() -> &'static str {
        "strava_timestamp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_sync_token() {
        let token = GoogleSyncToken("COjZlteFEOjZlteFGAUggICAgNicrrgCGg0IABIJCJmJmbwFEABgAQ".to_string());
        assert_eq!(token.to_string(), "COjZlteFEOjZlteFGAUggICAgNicrrgCGg0IABIJCJmJmbwFEABgAQ");
        assert_eq!(GoogleSyncToken::cursor_type(), "google_sync_token");

        let decoded = GoogleSyncToken::from_string(&token.to_string()).unwrap();
        assert_eq!(decoded.0, token.0);
    }

    #[test]
    fn test_typed_cursor() {
        let token = GoogleSyncToken("test_token".to_string());
        let typed = TypedCursor::new(&token);

        assert_eq!(typed.cursor_type, "google_sync_token");
        assert_eq!(typed.value, "test_token");

        let json = typed.to_json();
        let parsed = TypedCursor::from_json(&json).unwrap();
        assert_eq!(parsed.cursor_type, typed.cursor_type);

        let decoded: GoogleSyncToken = parsed.decode().unwrap();
        assert_eq!(decoded.0, token.0);
    }

    #[test]
    fn test_notion_cursor_validation() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        let cursor = NotionStartCursor::from_string(valid_uuid).unwrap();
        assert_eq!(cursor.0, valid_uuid);

        // Empty cursor is valid (initial state)
        let empty = NotionStartCursor::from_string("").unwrap();
        assert_eq!(empty.0, "");

        // Invalid UUID should fail
        let invalid = NotionStartCursor::from_string("not-a-uuid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_strava_timestamp_cursor() {
        let cursor = StravaTimestampCursor {
            before: Some(1609459200),
            after: Some(1577836800),
        };

        let serialized = cursor.to_string();
        let deserialized = StravaTimestampCursor::from_string(&serialized).unwrap();

        assert_eq!(deserialized.before, cursor.before);
        assert_eq!(deserialized.after, cursor.after);
    }

    #[test]
    fn test_cursor_type_mismatch() {
        let token = GoogleSyncToken("test".to_string());
        let typed = TypedCursor::new(&token);

        // Try to decode as wrong type
        let result: Result<NotionStartCursor> = typed.decode();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cursor type mismatch"));
    }
}
