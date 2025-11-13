//! macOS iMessage/SMS data processor

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validation::validate_timestamp_reasonable},
    storage::{Storage, stream_writer::StreamWriter},
};

/// Process Mac iMessage and SMS data
///
/// Parses and stores message text, contact information, and metadata from the Messages app.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for iMessage, but kept for API consistency)
/// * `stream_writer` - StreamWriter for writing to S3/object storage
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, stream_writer, record), fields(source = "mac", stream = "imessage"))]
pub async fn process(
    db: &Database,
    _storage: &Arc<Storage>,
    stream_writer: &Arc<Mutex<StreamWriter>>,
    record: &Value,
) -> Result<()> {
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

    // Write to S3/object storage via StreamWriter
    // This replaces the previous SQL INSERT INTO stream_mac_imessage
    {
        let mut writer = stream_writer.lock().await;

        writer.write_record(
            source_id,
            "imessage",
            record.clone(),
            Some(timestamp_dt),
        )?;
    }

    tracing::debug!("Wrote iMessage record to object storage for device {}", device_id);
    Ok(())
}
