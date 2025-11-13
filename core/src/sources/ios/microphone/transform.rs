//! iOS microphone audio to speech_transcription ontology transformation
//!
//! Transforms raw audio files from stream_ios_microphone into transcribed text
//! in the speech_transcription ontology table using AssemblyAI.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{ChainedTransform, OntologyTransform, TransformResult};
use crate::storage::Storage;
use crate::transcription::AssemblyAIClient;

/// Batch size for database inserts
const BATCH_SIZE: usize = 500;

/// Transform iOS microphone audio to speech_transcription ontology
pub struct MicrophoneTranscriptionTransform {
    transcription_client: AssemblyAIClient,
    storage: Storage,
}

impl MicrophoneTranscriptionTransform {
    /// Create a new microphone transcription transform
    ///
    /// # Arguments
    /// * `api_key` - AssemblyAI API key
    /// * `storage` - Storage backend for downloading audio files
    pub fn new(api_key: String, storage: Storage) -> Self {
        Self {
            transcription_client: AssemblyAIClient::new(api_key),
            storage,
        }
    }
}

#[async_trait]
impl OntologyTransform for MicrophoneTranscriptionTransform {
    fn source_table(&self) -> &str {
        "stream_ios_microphone"
    }

    fn target_table(&self) -> &str {
        "speech_transcription"
    }

    fn domain(&self) -> &str {
        "speech"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;
        let mut chained_transforms = Vec::new();

        // Batch accumulation for inserts
        let mut pending_records: Vec<(Uuid, String, Option<i32>, String, Option<String>, f64, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;

        tracing::info!(
            source_id = %source_id,
            "Starting microphone audio to speech_transcription transformation"
        );

        tracing::debug!("TRANSFORM METHOD INVOKED - reading from S3");

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "microphone_to_transcription";
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "microphone", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched microphone batches from S3"
        );

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(audio_file_key) = record.get("audio_file_key").and_then(|v| v.as_str()) else {
                    continue; // Skip records without audio_file_key
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let duration_seconds = record.get("duration_seconds")
                    .and_then(|v| v.as_i64())
                    .map(|d| d as i32);

            tracing::debug!(
                stream_id = %stream_id,
                audio_file_key = %audio_file_key,
                "Processing audio file for transcription"
            );

            tracing::debug!(
                stream_id = %stream_id,
                "About to generate presigned URL"
            );

            // Transcribe audio using AssemblyAI
            // Strategy: Check env var or URL to determine upload method
            // Set TRANSCRIPTION_USE_DIRECT_UPLOAD=true to force direct upload (for local dev, Docker, private networks)
            let transcription_result = match self.storage.get_presigned_url(
                &audio_file_key,
                std::time::Duration::from_secs(3600)
            ).await {
                Ok(presigned_url) => {
                    // Check if we should use direct upload instead of presigned URL
                    // This is necessary when AssemblyAI can't access the URL (e.g., local dev, private networks)
                    let use_direct_upload = std::env::var("TRANSCRIPTION_USE_DIRECT_UPLOAD")
                        .ok()
                        .and_then(|v| v.parse::<bool>().ok())
                        .unwrap_or_else(|| {
                            // Fallback: detect localhost/127.0.0.1 for backwards compatibility
                            presigned_url.contains("localhost") || presigned_url.contains("127.0.0.1")
                        });

                    if use_direct_upload {
                        // Download file and upload to AssemblyAI directly
                        tracing::debug!(
                            stream_id = %stream_id,
                            "Using direct upload mode (TRANSCRIPTION_USE_DIRECT_UPLOAD=true or localhost detected)"
                        );

                        match self.storage.download(&audio_file_key).await {
                            Ok(audio_bytes) => {
                                tracing::debug!(
                                    stream_id = %stream_id,
                                    file_size = audio_bytes.len(),
                                    "Downloaded audio file, uploading to AssemblyAI"
                                );

                                match self.transcription_client.transcribe_bytes(audio_bytes).await {
                                    Ok(result) => result,
                                    Err(e) => {
                                        tracing::error!(
                                            error = %e,
                                            stream_id = %stream_id,
                                            audio_file_key = %audio_file_key,
                                            "Failed to transcribe audio via direct upload, skipping"
                                        );
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = ?e,
                                    error_display = %e,
                                    stream_id = %stream_id,
                                    audio_file_key = %audio_file_key,
                                    "Failed to download audio file for transcription, skipping"
                                );
                                continue;
                            }
                        }
                    } else {
                        // Production: use presigned URL
                        tracing::debug!(
                            stream_id = %stream_id,
                            "Using presigned URL for AssemblyAI"
                        );

                        match self.transcription_client.transcribe_url(&presigned_url).await {
                            Ok(result) => result,
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    stream_id = %stream_id,
                                    audio_file_key = %audio_file_key,
                                    "Failed to transcribe audio via URL, skipping"
                                );
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        stream_id = %stream_id,
                        audio_file_key = %audio_file_key,
                        "Failed to generate presigned URL, skipping transcription"
                    );
                    continue;
                }
            };

            tracing::debug!(
                stream_id = %stream_id,
                transcript_length = transcription_result.text.len(),
                confidence = transcription_result.confidence,
                "Transcription completed"
            );

            // Create speech_transcription record with real transcript
            let transcription_id = Uuid::new_v4();

            // Accumulate record for batch insert
            pending_records.push((
                transcription_id,
                audio_file_key.to_string(),
                duration_seconds,
                transcription_result.text.clone(),
                transcription_result.language.clone(),
                transcription_result.confidence,
                timestamp,
                stream_id,
                serde_json::json!({}),
            ));

            last_processed_id = Some(stream_id);

            // Chain to semantic parsing transform (when implemented)
            chained_transforms.push(ChainedTransform {
                source_table: "speech_transcription".to_string(),
                target_tables: vec!["semantic_inferences".to_string()],
                domain: "semantic".to_string(),
                source_record_id: transcription_id,
                transform_stage: "semantic_parsing".to_string(),
            });

            // Flush batch if at capacity
            if pending_records.len() >= BATCH_SIZE {
                let insert_start = std::time::Instant::now();
                let batch_result = execute_speech_transcription_batch_insert(db, &pending_records).await;
                batch_insert_total_ms += insert_start.elapsed().as_millis();

                match batch_result {
                    Ok(written) => {
                        records_written += written;
                        tracing::debug!(
                            batch_size = pending_records.len(),
                            written,
                            "Batch insert completed"
                        );
                    }
                    Err(e) => {
                        records_failed += pending_records.len();
                        tracing::error!(
                            error = %e,
                            batch_size = pending_records.len(),
                            "Batch insert failed"
                        );
                    }
                }
                pending_records.clear();
            }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "microphone",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Final flush of remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_speech_transcription_batch_insert(db, &pending_records).await;
            batch_insert_total_ms += insert_start.elapsed().as_millis();

            match batch_result {
                Ok(written) => {
                    records_written += written;
                    tracing::debug!(
                        batch_size = pending_records.len(),
                        written,
                        "Final batch insert completed"
                    );
                }
                Err(e) => {
                    records_failed += pending_records.len();
                    tracing::error!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                }
            }
        }

        let avg_batch_insert_ms = if records_written > 0 {
            batch_insert_total_ms / records_written as u128
        } else {
            0
        };

        tracing::info!(
            records_read,
            records_written,
            records_failed,
            batch_insert_total_ms,
            avg_batch_insert_ms,
            "Completed microphone to speech_transcription transformation"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms,
        })
    }
}

/// Execute batch insert for speech_transcription records
///
/// # Arguments
/// * `db` - Database connection
/// * `records` - Batch of records to insert (id, audio_file_path, duration, text, language, confidence, timestamp, source_stream_id, metadata)
///
/// # Returns
/// Number of records successfully written
async fn execute_speech_transcription_batch_insert(
    db: &Database,
    records: &[(Uuid, String, Option<i32>, String, Option<String>, f64, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    // Build batch insert query using helper
    let columns = vec![
        "id",
        "audio_file_path",
        "audio_duration_seconds",
        "transcript_text",
        "language",
        "confidence_score",
        "speaker_count",
        "speaker_labels",
        "recorded_at",
        "source_stream_id",
        "source_table",
        "source_provider",
        "metadata",
    ];

    let query_str = Database::build_batch_insert_query(
        "elt.speech_transcription",
        &columns,
        "source_stream_id",
        records.len(),
    );

    // Build query with proper parameter binding
    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (id, audio_file_path, duration, text, language, confidence, timestamp, source_stream_id, metadata) in records {
        query = query
            .bind(id)
            .bind(audio_file_path)
            .bind(duration)
            .bind(text)
            .bind(language)
            .bind(confidence)
            .bind(None::<i32>) // speaker_count (not using diarization)
            .bind(None::<serde_json::Value>) // speaker_labels (not using diarization)
            .bind(timestamp)
            .bind(source_stream_id)
            .bind("stream_ios_microphone")
            .bind("ios")
            .bind(metadata);
    }

    // Execute batch insert
    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_metadata() {
        let storage = Storage::local("/tmp/test".to_string()).unwrap();
        let transform = MicrophoneTranscriptionTransform::new(
            "test-key".to_string(),
            storage,
        );

        assert_eq!(transform.source_table(), "stream_ios_microphone");
        assert_eq!(transform.target_table(), "speech_transcription");
        assert_eq!(transform.domain(), "speech");
    }
}
