//! Data models for stream object storage metadata

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::types::Decimal;
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

mod decimal_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use sqlx::types::Decimal;
    use std::str::FromStr;

    pub fn serialize<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(d) => {
                let float: f64 = d.to_string().parse().unwrap_or(0.0);
                serializer.serialize_some(&float)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<f64> = Option::deserialize(deserializer)?;
        Ok(opt.map(|f| Decimal::from_str(&f.to_string()).unwrap_or_else(|_| Decimal::from_str("0").unwrap())))
    }
}

/// User profile - biographical metadata (singleton table)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    // Identity
    pub full_name: Option<String>,
    pub preferred_name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    // Physical/Biometric
    #[serde(with = "decimal_serde")]
    pub height_cm: Option<Decimal>,
    #[serde(with = "decimal_serde")]
    pub weight_kg: Option<Decimal>,
    pub ethnicity: Option<String>,
    // Home Address
    pub home_street: Option<String>,
    pub home_city: Option<String>,
    pub home_state: Option<String>,
    pub home_postal_code: Option<String>,
    pub home_country: Option<String>,
    // Work/Occupation
    pub occupation: Option<String>,
    pub employer: Option<String>,
    // Audit
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Assistant profile - AI assistant preferences (singleton table)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AssistantProfile {
    pub id: Uuid,
    pub assistant_name: Option<String>,
    pub default_agent_id: Option<String>,
    pub default_model_id: Option<String>,
    pub enabled_tools: Option<serde_json::Value>,
    pub ui_preferences: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// S3 key builder for consistent object naming
pub struct StreamKeyBuilder {
    provider: String,
    source_id: Uuid,
    stream_name: String,
    date: NaiveDate,
}

impl StreamKeyBuilder {
    /// Create a new key builder
    pub fn new(provider: impl Into<String>, source_id: Uuid, stream_name: impl Into<String>, date: NaiveDate) -> Self {
        Self {
            provider: provider.into(),
            source_id,
            stream_name: stream_name.into(),
            date,
        }
    }

    /// Build S3 key with current timestamp
    ///
    /// Pattern: `streams/{provider}/{source_id}/{stream_name}/date={YYYY-MM-DD}/records_{unix_timestamp}.jsonl`
    ///
    /// Example: `streams/ios/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl`
    pub fn build(&self) -> String {
        let timestamp = chrono::Utc::now().timestamp();
        format!(
            "streams/{}/{}/{}/date={}/records_{}.jsonl",
            self.provider,
            self.source_id,
            self.stream_name,
            self.date.format("%Y-%m-%d"),
            timestamp
        )
    }

    /// Build S3 key with explicit timestamp
    pub fn build_with_timestamp(&self, timestamp: i64) -> String {
        format!(
            "streams/{}/{}/{}/date={}/records_{}.jsonl",
            self.provider,
            self.source_id,
            self.stream_name,
            self.date.format("%Y-%m-%d"),
            timestamp
        )
    }

    /// Build prefix for listing all objects for a source/stream
    ///
    /// Pattern: `streams/{provider}/{source_id}/{stream_name}/`
    pub fn build_stream_prefix(&self) -> String {
        format!("streams/{}/{}/{}/", self.provider, self.source_id, self.stream_name)
    }

    /// Build prefix for listing all objects for a source/stream/date
    ///
    /// Pattern: `streams/{provider}/{source_id}/{stream_name}/date={YYYY-MM-DD}/`
    pub fn build_date_prefix(&self) -> String {
        format!(
            "streams/{}/{}/{}/date={}/",
            self.provider,
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

    /// Extract provider from key
    ///
    /// Example: `streams/ios/550e8400.../healthkit/date=2025-01-15/records_1736899200.jsonl`
    /// Returns: `ios`
    pub fn provider(&self) -> Option<String> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 2 || parts[0] != "streams" {
            return None;
        }
        Some(parts[1].to_string())
    }

    /// Extract source_id from key
    ///
    /// Example: `streams/ios/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl`
    /// Returns: `550e8400-e29b-41d4-a716-446655440000`
    pub fn source_id(&self) -> Option<Uuid> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 3 || parts[0] != "streams" {
            return None;
        }
        Uuid::parse_str(parts[2]).ok()
    }

    /// Extract stream name from key
    ///
    /// Example: Returns: `healthkit`
    pub fn stream_name(&self) -> Option<String> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 4 || parts[0] != "streams" {
            return None;
        }
        Some(parts[3].to_string())
    }

    /// Extract date from key
    ///
    /// Example: Returns: `2025-01-15`
    pub fn date(&self) -> Option<NaiveDate> {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() < 5 || parts[0] != "streams" {
            return None;
        }

        // Parse "date=2025-01-15" format
        let date_part = parts[4];
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
        if parts.len() < 6 || parts[0] != "streams" {
            return None;
        }

        // Parse "records_1736899200.jsonl" format
        let filename = parts[5];
        if !filename.starts_with("records_") || !filename.ends_with(".jsonl") {
            return None;
        }

        let timestamp_str = filename.strip_prefix("records_")?.strip_suffix(".jsonl")?;
        timestamp_str.parse().ok()
    }

    /// Extract all metadata from key
    pub fn parse_all(&self) -> Option<(String, Uuid, String, NaiveDate, i64)> {
        Some((
            self.provider()?,
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

        let builder = StreamKeyBuilder::new("ios", source_id, "healthkit", date);

        let key = builder.build_with_timestamp(1736899200);
        assert_eq!(
            key,
            "streams/ios/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl"
        );

        let prefix = builder.build_stream_prefix();
        assert_eq!(
            prefix,
            "streams/ios/550e8400-e29b-41d4-a716-446655440000/healthkit/"
        );

        let date_prefix = builder.build_date_prefix();
        assert_eq!(
            date_prefix,
            "streams/ios/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/"
        );
    }

    #[test]
    fn test_stream_key_parser() {
        let key = "streams/ios/550e8400-e29b-41d4-a716-446655440000/healthkit/date=2025-01-15/records_1736899200.jsonl";
        let parser = StreamKeyParser::new(key);

        assert_eq!(parser.provider().unwrap(), "ios");
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

        let (provider, source_id, stream_name, date, timestamp) = parser.parse_all().unwrap();
        assert_eq!(provider, "ios");
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
