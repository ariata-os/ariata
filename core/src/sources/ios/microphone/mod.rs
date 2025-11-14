//! iOS Microphone data processor

pub mod transform;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validation::validate_timestamp_reasonable},
    storage::{stream_writer::StreamWriter, Storage},
};

pub use transform::MicrophoneTranscriptionTransform;

/// Process iOS Microphone data
///
/// Parses and stores audio level measurements, transcriptions, and audio files from iOS devices.
/// Audio files are stored in MinIO/S3 with metadata written to object storage via StreamWriter.
///
/// # Arguments
/// * `db` - Database connection
/// * `storage` - Storage layer for audio file uploads
/// * `stream_writer` - StreamWriter for writing to S3/object storage
/// * `record` - JSON record from the device
#[tracing::instrument(
    skip(db, storage, stream_writer, record),
    fields(source = "ios", stream = "microphone")
)]
pub async fn process(
    db: &Database,
    storage: &Arc<Storage>,
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

    // Validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    // Handle audio file upload if present (audio files still go to separate S3 location)
    let mut audio_file_key: Option<String> = None;
    let mut audio_file_size: Option<i32> = None;
    let audio_format = record.get("audio_format").and_then(|v| v.as_str());

    if let Some(audio_data_b64) = record.get("audio_data").and_then(|v| v.as_str()) {
        // Decode base64 audio data
        use base64::Engine;
        if let Ok(audio_bytes) = base64::engine::general_purpose::STANDARD.decode(audio_data_b64) {
            let key = format!(
                "ios/microphone/{}/{}.{}",
                device_id,
                uuid::Uuid::new_v4(),
                audio_format.unwrap_or("m4a")
            );

            if storage.upload(&key, audio_bytes.clone()).await.is_ok() {
                audio_file_key = Some(key.clone());
                audio_file_size = Some(audio_bytes.len() as i32);
            }
        }
    }

    // Build complete record including audio file metadata
    let mut record_with_audio = record.clone();
    if let Some(ref key) = audio_file_key {
        if let Some(obj) = record_with_audio.as_object_mut() {
            obj.insert(
                "uploaded_audio_file_key".to_string(),
                serde_json::json!(key),
            );
            obj.insert(
                "uploaded_audio_file_size".to_string(),
                serde_json::json!(audio_file_size),
            );
        }
    }

    // Write to S3/object storage via StreamWriter
    // This replaces the previous SQL INSERT INTO stream_ios_microphone
    {
        let mut writer = stream_writer.lock().await;

        writer.write_record(
            source_id,
            "microphone",
            record_with_audio,
            Some(timestamp_dt),
        )?;
    }

    tracing::debug!(
        "Wrote microphone record to object storage for device {}",
        device_id
    );
    Ok(())
}
