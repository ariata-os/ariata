//! iOS Location data processor and transforms

pub mod transform;

use serde_json::Value;
use std::sync::Arc;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validate_latitude, validate_longitude, validate_timestamp_reasonable},
    storage::Storage,
};

pub use transform::IosLocationTransform;

/// Process iOS Location data
///
/// Parses and stores GPS coordinates, activity type, and movement data from iOS devices.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for location, but kept for API consistency)
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, record), fields(source = "ios", stream = "location"))]
pub async fn process(db: &Database, _storage: &Arc<Storage>, record: &Value) -> Result<()> {
    let device_id = record.get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "ios", device_id).await?;

    let timestamp = record.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    let latitude = record.get("latitude")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| Error::Other("Missing latitude".into()))?;

    let longitude = record.get("longitude")
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

    let altitude = record.get("altitude").and_then(|v| v.as_f64());
    let speed = record.get("speed").and_then(|v| v.as_f64());
    let course = record.get("course").and_then(|v| v.as_f64());
    let horizontal_accuracy = record.get("horizontal_accuracy").and_then(|v| v.as_f64());
    let vertical_accuracy = record.get("vertical_accuracy").and_then(|v| v.as_f64());
    let activity_type = record.get("activity_type").and_then(|v| v.as_str());
    let activity_confidence = record.get("activity_confidence").and_then(|v| v.as_str());
    let floor_level = record.get("floor_level").and_then(|v| v.as_i64()).map(|v| v as i32);

    sqlx::query(
        "INSERT INTO stream_ios_location
         (source_id, timestamp, latitude, longitude, altitude, speed, course,
          horizontal_accuracy, vertical_accuracy, activity_type, activity_confidence,
          floor_level, raw_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (source_id, timestamp)
         DO UPDATE SET
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            altitude = EXCLUDED.altitude,
            speed = EXCLUDED.speed,
            course = EXCLUDED.course,
            horizontal_accuracy = EXCLUDED.horizontal_accuracy,
            vertical_accuracy = EXCLUDED.vertical_accuracy,
            activity_type = EXCLUDED.activity_type,
            activity_confidence = EXCLUDED.activity_confidence,
            floor_level = EXCLUDED.floor_level,
            raw_data = EXCLUDED.raw_data"
    )
    .bind(source_id)
    .bind(timestamp)
    .bind(latitude)
    .bind(longitude)
    .bind(altitude)
    .bind(speed)
    .bind(course)
    .bind(horizontal_accuracy)
    .bind(vertical_accuracy)
    .bind(activity_type)
    .bind(activity_confidence)
    .bind(floor_level)
    .bind(record)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to insert location data: {e}")))?;

    tracing::debug!("Inserted location record for device {}", device_id);
    Ok(())
}
