//! macOS screen time duration data processor

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validation::validate_timestamp_reasonable},
    storage::{stream_writer::StreamWriter, Storage},
};

/// Process Mac screen time duration data
///
/// Parses and stores focused app sessions with duration tracking from macOS devices.
/// Records include app name, bundle ID, window title, start/end times, and session duration.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for screen_time, but kept for API consistency)
/// * `stream_writer` - StreamWriter for writing to S3/object storage
/// * `record` - JSON record from the device with format:
///   ```json
///   {
///     "device_id": "abc123",
///     "timestamp": "2025-01-14T10:30:00Z",
///     "records": [
///       {
///         "app_name": "Chrome",
///         "bundle_id": "com.google.Chrome",
///         "window_title": "Document Title",
///         "started_at": "2025-01-14T10:15:00Z",
///         "ended_at": "2025-01-14T10:30:00Z",
///         "duration_seconds": 900,
///         "event_type": "focus_session"
///       }
///     ]
///   }
///   ```
#[tracing::instrument(
    skip(db, _storage, stream_writer, record),
    fields(source = "mac", stream = "screen_time")
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

    let source_id = get_or_create_device_source(db, "mac", device_id).await?;

    let timestamp = record
        .get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    // Validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    // Validate records array exists
    let _records = record
        .get("records")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::Other("Missing or invalid records array".into()))?;

    // Write to S3/object storage via StreamWriter
    {
        let mut writer = stream_writer.lock().await;

        writer.write_record(source_id, "screen_time", record.clone(), Some(timestamp_dt))?;
    }

    tracing::debug!(
        "Wrote screen time record to object storage for device {}",
        device_id
    );
    Ok(())
}
