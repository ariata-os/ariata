//! iOS Microphone data processor

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
        base::validation::validate_timestamp_reasonable,
        push_stream::{IngestPayload, PushResult, PushStream},
    },
    storage::{stream_writer::StreamWriter, Storage},
};

pub use transform::MicrophoneTranscriptionTransform;

/// iOS Microphone stream implementing PushStream trait
///
/// Receives microphone/audio data pushed from iOS devices via /ingest endpoint.
pub struct IosMicrophoneStream {
    db: PgPool,
    storage: Arc<Storage>,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl IosMicrophoneStream {
    /// Create a new IosMicrophoneStream
    pub fn new(db: PgPool, storage: Arc<Storage>, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self {
            db,
            storage,
            stream_writer,
        }
    }
}

#[async_trait]
impl PushStream for IosMicrophoneStream {
    async fn receive_push(&self, source_id: Uuid, payload: IngestPayload) -> Result<PushResult> {
        // Validate payload
        self.validate_payload(&payload)?;

        let mut result = PushResult::new(payload.records.len());

        // source_id is passed from handler - single source of truth, no duplicate DB query

        // Process each record
        for record in &payload.records {
            // iOS sends timestamp_start and timestamp_end for microphone chunks
            let timestamp = record
                .get("timestamp_start")
                .or_else(|| record.get("timestamp"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing timestamp in record".into()))?;

            let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
                .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
                .with_timezone(&Utc);
            validate_timestamp_reasonable(timestamp_dt)?;

            // Handle audio file upload if present
            let mut audio_file_key: Option<String> = None;
            let mut audio_file_size: Option<i32> = None;
            let audio_format = record.get("audio_format").and_then(|v| v.as_str());

            if let Some(audio_data_b64) = record.get("audio_data").and_then(|v| v.as_str()) {
                use base64::Engine;
                if let Ok(audio_bytes) =
                    base64::engine::general_purpose::STANDARD.decode(audio_data_b64)
                {
                    let key = format!(
                        "ios/microphone/{}/{}.{}",
                        payload.device_id,
                        Uuid::new_v4(),
                        audio_format.unwrap_or("m4a")
                    );

                    if self.storage.upload(&key, audio_bytes.clone()).await.is_ok() {
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

            // Write to object storage via StreamWriter
            {
                let mut writer = self.stream_writer.lock().await;
                writer.write_record(
                    source_id,
                    "microphone",
                    record_with_audio,
                    Some(timestamp_dt),
                )?;
            }

            result.records_written += 1;
        }

        tracing::info!(
            "Processed {} microphone records from device {}",
            result.records_written,
            payload.device_id
        );

        Ok(result)
    }

    fn table_name(&self) -> &str {
        "stream_ios_microphone"
    }

    fn stream_name(&self) -> &str {
        "microphone"
    }

    fn source_name(&self) -> &str {
        "ios"
    }
}
