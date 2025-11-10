//! Data models for stream object storage metadata

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Metadata for a stream data object stored in S3/MinIO
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StreamObject {
    pub id: Uuid,
    pub source_id: Uuid,
    pub stream_name: String,
    pub s3_key: String,
    pub record_count: i32,
    pub size_bytes: i64,
    pub min_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub max_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Transform checkpoint tracking which objects have been processed
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StreamTransformCheckpoint {
    pub id: Uuid,
    pub source_id: Uuid,
    pub stream_name: String,
    pub transform_name: String,
    pub last_processed_s3_key: Option<String>,
    pub last_processed_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub last_processed_object_id: Option<Uuid>,
    pub records_processed: i64,
    pub objects_processed: i64,
    pub last_run_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// S3 key builder for consistent object naming
pub struct StreamKeyBuilder {
    source_id: Uuid,
    stream_name: String,
    date: NaiveDate,
}

impl StreamKeyBuilder {
    /// Create a new key builder
    pub fn new(source_id: Uuid, stream_name: impl Into<String>, date: NaiveDate) -> Self {
        Self {
            source_id,
            stream_name: stream_name.into(),
            date,
        }
    }

    /// Build S3 key with current timestamp
    ///
    /// Pattern: `streams/{source_id}/{stream_name}/date={YYYY-MM-DD}/records_{unix_timestamp}.jsonl`
    ///
    /// Example: `streams/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl`
    pub fn build(&self) -> String {
        let timestamp = chrono::Utc::now().timestamp();
        format!(
            "streams/{}/{}/date={}/records_{}.jsonl",
            self.source_id,
            self.stream_name,
            self.date.format("%Y-%m-%d"),
            timestamp
        )
    }

    /// Build S3 key with explicit timestamp
    pub fn build_with_timestamp(&self, timestamp: i64) -> String {
        format!(
            "streams/{}/{}/date={}/records_{}.jsonl",
            self.source_id,
            self.stream_name,
            self.date.format("%Y-%m-%d"),
            timestamp
        )
    }

    /// Build prefix for listing all objects for a source/stream
    ///
    /// Pattern: `streams/{source_id}/{stream_name}/`
    pub fn build_stream_prefix(&self) -> String {
        format!("streams/{}/{}/", self.source_id, self.stream_name)
    }

    /// Build prefix for listing all objects for a source/stream/date
    ///
    /// Pattern: `streams/{source_id}/{stream_name}/date={YYYY-MM-DD}/`
    pub fn build_date_prefix(&self) -> String {
        format!(
            "streams/{}/{}/date={}/",
            self.source_id,
            self.stream_name,
            self.date.format("%Y-%m-%d")
        )
    }
}

/// Parser for extracting metadata from S3 keys
pub struct StreamKeyParser {
    key: String,
}

impl StreamKeyParser {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }

    /// Extract source_id from key
    ///
    /// Example: `streams/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl`
    /// Returns: `550e8400-e29b-41d4-a716-446655440000`
    pub fn source_id(&self) -> Option<Uuid> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 2 || parts[0] != "streams" {
            return None;
        }
        Uuid::parse_str(parts[1]).ok()
    }

    /// Extract stream name from key
    ///
    /// Example: Returns: `healthkit`
    pub fn stream_name(&self) -> Option<String> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 3 || parts[0] != "streams" {
            return None;
        }
        Some(parts[2].to_string())
    }

    /// Extract date from key
    ///
    /// Example: Returns: `2025-01-15`
    pub fn date(&self) -> Option<NaiveDate> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 4 || parts[0] != "streams" {
            return None;
        }

        // Parse "date=2025-01-15" format
        let date_part = parts[3];
        if !date_part.starts_with("date=") {
            return None;
        }

        let date_str = date_part.strip_prefix("date=")?;
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
    }

    /// Extract timestamp from key
    ///
    /// Example: Returns: `1736899200`
    pub fn timestamp(&self) -> Option<i64> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 5 || parts[0] != "streams" {
            return None;
        }

        // Parse "records_1736899200.jsonl" format
        let filename = parts[4];
        if !filename.starts_with("records_") || !filename.ends_with(".jsonl") {
            return None;
        }

        let timestamp_str = filename
            .strip_prefix("records_")?
            .strip_suffix(".jsonl")?;
        timestamp_str.parse().ok()
    }

    /// Extract all metadata from key
    pub fn parse_all(&self) -> Option<(Uuid, String, NaiveDate, i64)> {
        Some((
            self.source_id()?,
            self.stream_name()?,
            self.date()?,
            self.timestamp()?,
        ))
    }

    /// Static helper to parse date from S3 key (for use in encryption key derivation)
    ///
    /// # Arguments
    /// * `key` - S3 key to parse
    ///
    /// # Returns
    /// NaiveDate extracted from the key
    pub fn parse_date_from_key(key: &str) -> crate::error::Result<NaiveDate> {
        let parser = Self::new(key);
        parser.date().ok_or_else(|| {
            crate::error::Error::Other(format!("Failed to parse date from S3 key: {}", key))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_key_builder() {
        let source_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let builder = StreamKeyBuilder::new(source_id, "healthkit", date);

        let key = builder.build_with_timestamp(1736899200);
        assert_eq!(
            key,
            "streams/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl"
        );

        let prefix = builder.build_stream_prefix();
        assert_eq!(
            prefix,
            "streams/550e8400-e29b-41d4-a716-446655440000/healthkit/"
        );

        let date_prefix = builder.build_date_prefix();
        assert_eq!(
            date_prefix,
            "streams/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/"
        );
    }

    #[test]
    fn test_stream_key_parser() {
        let key = "streams/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl";
        let parser = StreamKeyParser::new(key);

        assert_eq!(
            parser.source_id().unwrap(),
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(parser.stream_name().unwrap(), "healthkit");
        assert_eq!(
            parser.date().unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
        assert_eq!(parser.timestamp().unwrap(), 1736899200);

        let (source_id, stream_name, date, timestamp) = parser.parse_all().unwrap();
        assert_eq!(
            source_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(stream_name, "healthkit");
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
        assert_eq!(timestamp, 1736899200);
    }

    #[test]
    fn test_stream_key_parser_invalid() {
        let parser = StreamKeyParser::new("invalid/key/format");
        assert!(parser.source_id().is_none());
        assert!(parser.stream_name().is_none());
        assert!(parser.date().is_none());
        assert!(parser.timestamp().is_none());
    }
}
