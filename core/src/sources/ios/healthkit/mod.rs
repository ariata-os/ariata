//! iOS HealthKit data processor and transforms

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
            validate_heart_rate, validate_percentage,
            validate_positive, validate_timestamp_reasonable,
        },
        push_stream::{IngestPayload, PushResult, PushStream},
    },
    storage::stream_writer::StreamWriter,
};

pub use transform::{
    HealthKitHRVTransform, HealthKitHeartRateTransform, HealthKitSleepTransform,
    HealthKitStepsTransform, HealthKitWorkoutTransform,
};

/// iOS HealthKit stream implementing PushStream trait
///
/// Receives health data pushed from iOS devices via /ingest endpoint.
pub struct IosHealthKitStream {
    _db: PgPool,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl IosHealthKitStream {
    /// Create a new IosHealthKitStream
    pub fn new(db: PgPool, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self { _db: db, stream_writer }
    }
}

#[async_trait]
impl PushStream for IosHealthKitStream {
    async fn receive_push(&self, source_id: Uuid, payload: IngestPayload) -> Result<PushResult> {
        // Validate payload
        self.validate_payload(&payload)?;

        let mut result = PushResult::new(payload.records.len());

        // source_id is passed from handler - single source of truth, no duplicate DB query

        // Process each record
        for record in &payload.records {
            // Parse timestamp
            let timestamp = record
                .get("timestamp")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing timestamp in record".into()))?;

            let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
                .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
                .with_timezone(&Utc);
            validate_timestamp_reasonable(timestamp_dt)?;

            // Extract and validate health metrics (all optional)
            if let Some(hr) = record.get("heart_rate").and_then(|v| v.as_f64()) {
                validate_heart_rate(hr)?;
            }
            if let Some(rhr) = record.get("resting_heart_rate").and_then(|v| v.as_f64()) {
                validate_heart_rate(rhr)?;
            }
            if let Some(hrv_val) = record.get("hrv").and_then(|v| v.as_f64()) {
                validate_positive("HRV", hrv_val)?;
            }
            if let Some(s) = record.get("steps").and_then(|v| v.as_i64()).map(|v| v as i32) {
                if s < 0 {
                    return Err(Error::InvalidInput("Steps cannot be negative".into()));
                }
            }
            if let Some(d) = record.get("distance").and_then(|v| v.as_f64()) {
                validate_positive("Distance", d)?;
            }
            if let Some(bf) = record.get("body_fat_percentage").and_then(|v| v.as_f64()) {
                validate_percentage("Body fat percentage", bf)?;
            }

            // Write to object storage via StreamWriter
            {
                let mut writer = self.stream_writer.lock().await;
                writer.write_record(source_id, "healthkit", record.clone(), Some(timestamp_dt))?;
            }

            result.records_written += 1;
        }

        tracing::info!(
            "Processed {} HealthKit records from device {}",
            result.records_written,
            payload.device_id
        );

        Ok(result)
    }

    fn table_name(&self) -> &str {
        "stream_ios_healthkit"
    }

    fn stream_name(&self) -> &str {
        "healthkit"
    }

    fn source_name(&self) -> &str {
        "ios"
    }
}
