//! iOS Microphone data processor

pub mod transform;

use serde_json::Value;
use std::sync::Arc;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::{get_or_create_device_source, validation::validate_timestamp_reasonable},
    storage::Storage,
};

pub use transform::MicrophoneTranscriptionTransform;

/// Process iOS Microphone data
///
/// Parses and stores audio level measurements, transcriptions, and audio files from iOS devices.
/// Audio files are stored in MinIO/S3 with metadata in PostgreSQL.
///
/// # Arguments
/// * `db` - Database connection
/// * `storage` - Storage layer for audio file uploads
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, storage, record), fields(source = "ios", stream = "microphone"))]
pub async fn process(db: &Database, storage: &Arc<Storage>, record: &Value) -> Result<()> {
    let device_id = record.get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "ios", device_id).await?;

    let timestamp = record.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    // Validate timestamp
    let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
        .with_timezone(&chrono::Utc);
    validate_timestamp_reasonable(timestamp_dt)?;

    let decibels = record.get("decibels").and_then(|v| v.as_f64());
    let average_power = record.get("average_power").and_then(|v| v.as_f64());
    let peak_power = record.get("peak_power").and_then(|v| v.as_f64());
    let transcription = record.get("transcription").and_then(|v| v.as_str());
    let transcription_confidence = record.get("transcription_confidence").and_then(|v| v.as_f64());
    let language = record.get("language").and_then(|v| v.as_str());
    let duration_seconds = record.get("duration_seconds").and_then(|v| v.as_i64()).map(|v| v as i32);
    let sample_rate = record.get("sample_rate").and_then(|v| v.as_i64()).map(|v| v as i32);

    // Handle audio file upload if present
    let mut audio_file_key: Option<String> = None;
    let mut audio_file_size: Option<i32> = None;
    let audio_format = record.get("audio_format").and_then(|v| v.as_str());

    if let Some(audio_data_b64) = record.get("audio_data").and_then(|v| v.as_str()) {
        // Decode base64 audio data
        use base64::Engine;
        if let Ok(audio_bytes) = base64::engine::general_purpose::STANDARD.decode(audio_data_b64) {
            let key = format!("ios/microphone/{}/{}.{}",
                device_id,
                uuid::Uuid::new_v4(),
                audio_format.unwrap_or("m4a")
            );

            if storage.upload(&key, audio_bytes.clone()).await.is_ok() {
                audio_file_key = Some(key);
                audio_file_size = Some(audio_bytes.len() as i32);
            }
        }
    }

    sqlx::query(
        "INSERT INTO stream_ios_microphone
         (source_id, timestamp, decibels, average_power, peak_power, transcription,
          transcription_confidence, language, duration_seconds, sample_rate,
          audio_file_key, audio_file_size, audio_format, raw_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (source_id, timestamp)
         DO UPDATE SET
            decibels = EXCLUDED.decibels,
            average_power = EXCLUDED.average_power,
            peak_power = EXCLUDED.peak_power,
            transcription = EXCLUDED.transcription,
            transcription_confidence = EXCLUDED.transcription_confidence,
            language = EXCLUDED.language,
            duration_seconds = EXCLUDED.duration_seconds,
            sample_rate = EXCLUDED.sample_rate,
            audio_file_key = EXCLUDED.audio_file_key,
            audio_file_size = EXCLUDED.audio_file_size,
            audio_format = EXCLUDED.audio_format,
            raw_data = EXCLUDED.raw_data"
    )
    .bind(source_id)
    .bind(timestamp)
    .bind(decibels)
    .bind(average_power)
    .bind(peak_power)
    .bind(transcription)
    .bind(transcription_confidence)
    .bind(language)
    .bind(duration_seconds)
    .bind(sample_rate)
    .bind(audio_file_key)
    .bind(audio_file_size)
    .bind(audio_format)
    .bind(record)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to insert microphone data: {e}")))?;

    tracing::debug!("Inserted microphone record for device {}", device_id);
    Ok(())
}
