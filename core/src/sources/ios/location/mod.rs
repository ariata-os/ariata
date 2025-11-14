//! iOS Location data processor and transforms

pub mod transform;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{
        get_or_create_device_source, validate_latitude, validate_longitude,
        validate_timestamp_reasonable,
    },
    storage::{stream_writer::StreamWriter, Storage},
};

pub use transform::IosLocationTransform;

/// Process iOS Location data
///
/// Parses and stores GPS coordinates, activity type, and movement data from iOS devices.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for location, but kept for API consistency)
/// * `stream_writer` - StreamWriter for writing to S3/object storage
/// * `record` - JSON record from the device
#[tracing::instrument(
    skip(db, _storage, stream_writer, record),
    fields(source = "ios", stream = "location")
)]
pub async fn process(
    db: &Database,
    _storage: &Arc<Storage>,
    stream_writer: &Arc<Mutex<StreamWriter>>,
    record: &Value,
) -> Result<()> {
    let device_id = record
        .get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "ios", device_id).await?;

    let timestamp = record
        .get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    let latitude = record
        .get("latitude")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| Error::Other("Missing latitude".into()))?;

    let longitude = record
        .get("longitude")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| Error::Other("Missing longitude".into()))?;

    // Validate coordinates
    validate_latitude(latitude)?;
    validate_longitude(longitude)?;

    // Parse and validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    // Write to S3/object storage via StreamWriter
    // This replaces the previous SQL INSERT INTO stream_ios_location
    {
        let mut writer = stream_writer.lock().await;

        writer.write_record(source_id, "location", record.clone(), Some(timestamp_dt))?;
    }

    tracing::debug!(
        "Wrote location record to object storage for device {}",
        device_id
    );
    Ok(())
}
