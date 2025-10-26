//! macOS application usage data processor

use serde_json::Value;
use std::sync::Arc;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validation::validate_timestamp_reasonable},
    storage::Storage,
};

/// Process Mac application usage data
///
/// Parses and stores app names, bundle IDs, window titles, and usage duration from macOS devices.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for apps, but kept for API consistency)
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, record), fields(source = "mac", stream = "apps"))]
pub async fn process(db: &Database, _storage: &Arc<Storage>, record: &Value) -> Result<()> {
    let device_id = record.get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "mac", device_id).await?;

    let timestamp = record.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    // Validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    let app_name = record.get("app_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing app_name".into()))?;

    let bundle_id = record.get("bundle_id").and_then(|v| v.as_str());
    let app_version = record.get("app_version").and_then(|v| v.as_str());
    let window_title = record.get("window_title").and_then(|v| v.as_str());
    let window_index = record.get("window_index").and_then(|v| v.as_i64()).map(|v| v as i32);
    let duration_seconds = record.get("duration_seconds").and_then(|v| v.as_i64()).map(|v| v as i32);
    let is_frontmost = record.get("is_frontmost").and_then(|v| v.as_bool());
    let category = record.get("category").and_then(|v| v.as_str());

    sqlx::query(
        "INSERT INTO stream_mac_apps
         (source_id, timestamp, app_name, bundle_id, app_version, window_title,
          window_index, duration_seconds, is_frontmost, category, raw_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (source_id, timestamp, app_name)
         DO UPDATE SET
            bundle_id = EXCLUDED.bundle_id,
            app_version = EXCLUDED.app_version,
            window_title = EXCLUDED.window_title,
            window_index = EXCLUDED.window_index,
            duration_seconds = EXCLUDED.duration_seconds,
            is_frontmost = EXCLUDED.is_frontmost,
            category = EXCLUDED.category,
            raw_data = EXCLUDED.raw_data"
    )
    .bind(source_id)
    .bind(timestamp)
    .bind(app_name)
    .bind(bundle_id)
    .bind(app_version)
    .bind(window_title)
    .bind(window_index)
    .bind(duration_seconds)
    .bind(is_frontmost)
    .bind(category)
    .bind(record)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to insert app data: {e}")))?;

    tracing::debug!("Inserted app usage record for device {}", device_id);
    Ok(())
}
