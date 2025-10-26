//! macOS iMessage/SMS data processor

use serde_json::Value;
use std::sync::Arc;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validation::validate_timestamp_reasonable},
    storage::Storage,
};

/// Process Mac iMessage and SMS data
///
/// Parses and stores message text, contact information, and metadata from the Messages app.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for iMessage, but kept for API consistency)
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, record), fields(source = "mac", stream = "imessage"))]
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

    let message_text = record.get("message_text").and_then(|v| v.as_str());
    let is_from_me = record.get("is_from_me")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| Error::Other("Missing is_from_me".into()))?;

    let contact_id = record.get("contact_id").and_then(|v| v.as_str());
    let contact_name = record.get("contact_name").and_then(|v| v.as_str());
    let phone_number = record.get("phone_number").and_then(|v| v.as_str());
    let is_group_chat = record.get("is_group_chat").and_then(|v| v.as_bool());
    let is_read = record.get("is_read").and_then(|v| v.as_bool());
    let has_attachment = record.get("has_attachment").and_then(|v| v.as_bool());
    let attachment_count = record.get("attachment_count").and_then(|v| v.as_i64()).map(|v| v as i32);

    // Convert attachment_types array to Vec<String>
    let attachment_types: Option<Vec<String>> = record.get("attachment_types")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    let service = record.get("service").and_then(|v| v.as_str());

    sqlx::query(
        "INSERT INTO stream_mac_imessage
         (source_id, timestamp, message_text, is_from_me, contact_id, contact_name,
          phone_number, is_group_chat, is_read, has_attachment, attachment_count,
          attachment_types, service, raw_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (source_id, timestamp, contact_id, is_from_me)
         DO UPDATE SET
            message_text = EXCLUDED.message_text,
            contact_name = EXCLUDED.contact_name,
            phone_number = EXCLUDED.phone_number,
            is_group_chat = EXCLUDED.is_group_chat,
            is_read = EXCLUDED.is_read,
            has_attachment = EXCLUDED.has_attachment,
            attachment_count = EXCLUDED.attachment_count,
            attachment_types = EXCLUDED.attachment_types,
            service = EXCLUDED.service,
            raw_data = EXCLUDED.raw_data"
    )
    .bind(source_id)
    .bind(timestamp)
    .bind(message_text)
    .bind(is_from_me)
    .bind(contact_id)
    .bind(contact_name)
    .bind(phone_number)
    .bind(is_group_chat)
    .bind(is_read)
    .bind(has_attachment)
    .bind(attachment_count)
    .bind(attachment_types.as_deref())
    .bind(service)
    .bind(record)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to insert iMessage data: {e}")))?;

    tracing::debug!("Inserted iMessage record for device {}", device_id);
    Ok(())
}
