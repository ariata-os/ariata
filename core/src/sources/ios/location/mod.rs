//! iOS Location data processor and transforms

pub mod transform;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    sources::{
        base::{
            validate_latitude, validate_longitude,
            validate_timestamp_reasonable,
        },
        push_stream::{IngestPayload, PushResult, PushStream},
    },
    storage::stream_writer::StreamWriter,
};

pub use transform::IosLocationTransform;

/// iOS Location stream implementing PushStream trait
///
/// Receives location data pushed from iOS devices via /ingest endpoint.
pub struct IosLocationStream {
    db: PgPool,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl IosLocationStream {
    /// Create a new IosLocationStream
    pub fn new(db: PgPool, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self { db, stream_writer }
    }
}

#[async_trait]
impl PushStream for IosLocationStream {
    async fn receive_push(&self, source_id: Uuid, payload: IngestPayload) -> Result<PushResult> {
        // Validate payload
        self.validate_payload(&payload)?;

        let mut result = PushResult::new(payload.records.len());

        // source_id is passed from handler - single source of truth, no duplicate DB query

        // Process each record
        for record in &payload.records {
            // Extract required fields
            let timestamp = record
                .get("timestamp")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing timestamp in record".into()))?;

            let latitude = record
                .get("latitude")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| Error::Other("Missing latitude in record".into()))?;

            let longitude = record
                .get("longitude")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| Error::Other("Missing longitude in record".into()))?;

            // Validate coordinates
            validate_latitude(latitude)?;
            validate_longitude(longitude)?;

            // Parse and validate timestamp
            let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
                .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
                .with_timezone(&Utc);
            validate_timestamp_reasonable(timestamp_dt)?;

            // Write to object storage via StreamWriter
            {
                let mut writer = self.stream_writer.lock().await;
                writer.write_record(source_id, "location", record.clone(), Some(timestamp_dt))?;
            }

            result.records_written += 1;
        }

        tracing::info!(
            "Processed {} location records from device {}",
            result.records_written,
            payload.device_id
        );

        Ok(result)
    }

    fn table_name(&self) -> &str {
        "stream_ios_location"
    }

    fn stream_name(&self) -> &str {
        "location"
    }

    fn source_name(&self) -> &str {
        "ios"
    }
}
