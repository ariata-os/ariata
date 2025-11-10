//! iOS HealthKit data processor and transforms

pub mod transform;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validate_heart_rate, validate_positive, validate_percentage, validate_timestamp_reasonable},
    storage::{Storage, stream_writer::StreamWriter},
};

pub use transform::{
    HealthKitHeartRateTransform,
    HealthKitHRVTransform,
    HealthKitStepsTransform,
    HealthKitSleepTransform,
    HealthKitWorkoutTransform,
};

/// Process iOS HealthKit data
///
/// Parses and stores health metrics from iOS devices including heart rate, steps,
/// sleep data, workouts, and other HealthKit measurements.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for HealthKit, but kept for API consistency)
/// * `stream_writer` - StreamWriter for writing to S3/object storage
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, stream_writer, record), fields(source = "ios", stream = "healthkit"))]
pub async fn process(
    db: &Database,
    _storage: &Arc<Storage>,
    stream_writer: &Arc<Mutex<StreamWriter>>,
    record: &Value,
) -> Result<()> {
    // Get source_id from device_id in the record
    let device_id = record.get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "ios", device_id).await?;

    // Parse timestamp
    let timestamp = record.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    // Extract health metrics for validation (all optional)
    let heart_rate = record.get("heart_rate").and_then(|v| v.as_f64());
    let hrv = record.get("hrv").and_then(|v| v.as_f64());
    let resting_heart_rate = record.get("resting_heart_rate").and_then(|v| v.as_f64());
    let steps = record.get("steps").and_then(|v| v.as_i64()).map(|v| v as i32);
    let distance = record.get("distance").and_then(|v| v.as_f64());
    let body_fat_percentage = record.get("body_fat_percentage").and_then(|v| v.as_f64());

    // Validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    // Validate health metrics if present
    if let Some(hr) = heart_rate {
        validate_heart_rate(hr)?;
    }
    if let Some(rhr) = resting_heart_rate {
        validate_heart_rate(rhr)?;
    }
    if let Some(hrv_val) = hrv {
        validate_positive("HRV", hrv_val)?;
    }
    if let Some(s) = steps {
        if s < 0 {
            return Err(Error::InvalidInput("Steps cannot be negative".into()));
        }
    }
    if let Some(d) = distance {
        validate_positive("Distance", d)?;
    }
    if let Some(bf) = body_fat_percentage {
        validate_percentage("Body fat percentage", bf)?;
    }

    // Write to S3/object storage via StreamWriter
    // This replaces the previous SQL INSERT INTO stream_ios_healthkit
    {
        let mut writer = stream_writer.lock().await;

        writer.write_record(
            source_id,
            "healthkit",
            record.clone(),
            Some(timestamp_dt),
        ).await?;
    }

    tracing::debug!("Wrote healthkit record to object storage for device {}", device_id);
    Ok(())
}
