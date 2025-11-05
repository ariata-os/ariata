//! iOS microphone audio to speech_transcription ontology transformation
//!
//! Transforms raw audio files from stream_ios_microphone into transcribed text
//! in the speech_transcription ontology table using AssemblyAI.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::database::Database;
use crate::error::{Error, Result};
use crate::sources::base::{ChainedTransform, OntologyTransform, TransformResult};
use crate::storage::Storage;
use crate::transcription::AssemblyAIClient;

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

    #[tracing::instrument(skip(self, db), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;
        let mut chained_transforms = Vec::new();

        tracing::info!(
            source_id = %source_id,
            "Starting microphone audio to speech_transcription transformation"
        );

        tracing::debug!("TRANSFORM METHOD INVOKED - about to query database");

        // Query stream_ios_microphone for records not yet transcribed
        // Use left join to find records that don't exist in speech_transcription
        let rows = sqlx::query(
            r#"
            SELECT
                m.id, m.timestamp, m.audio_file_key, m.duration_seconds,
                m.decibels, m.sample_rate
            FROM elt.stream_ios_microphone m
            LEFT JOIN elt.speech_transcription t ON (t.source_stream_id = m.id)
            WHERE m.source_id = $1
              AND m.audio_file_key IS NOT NULL
              AND t.id IS NULL
            ORDER BY m.timestamp ASC
            LIMIT 100
            "#,
        )
        .bind(source_id)
        .fetch_all(db.pool())
        .await?;

        tracing::debug!(
            records_to_transform = rows.len(),
            "Fetched untranscribed microphone records"
        );

        for row in rows {
            records_read += 1;

            // Extract fields from row
            let stream_id: Uuid = row.try_get("id")?;
            let timestamp: DateTime<Utc> = row.try_get("timestamp")?;
            let audio_file_key: String = row.try_get("audio_file_key")?;
            let duration_seconds: Option<i32> = row.try_get("duration_seconds")?;

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

            sqlx::query(
                r#"
                INSERT INTO elt.speech_transcription (
                    id,
                    audio_file_path,
                    audio_duration_seconds,
                    transcript_text,
                    language,
                    confidence_score,
                    speaker_count,
                    speaker_labels,
                    recorded_at,
                    source_stream_id,
                    source_table,
                    source_provider,
                    metadata
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(transcription_id)
            .bind(&audio_file_key)
            .bind(duration_seconds)
            .bind(&transcription_result.text)
            .bind(transcription_result.language.as_ref())
            .bind(Some(transcription_result.confidence))
            .bind(None::<i32>)  // speaker_count (not using diarization)
            .bind(None::<serde_json::Value>)  // speaker_labels (not using diarization)
            .bind(timestamp)
            .bind(stream_id)
            .bind(self.source_table())
            .bind("ios")
            .bind(serde_json::json!({}))
            .execute(db.pool())
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    stream_id = %stream_id,
                    "Failed to insert speech_transcription record"
                );
                Error::Database(e.to_string())
            })?;

            records_written += 1;
            last_processed_id = Some(stream_id);

            // Chain to semantic parsing transform (when implemented)
            chained_transforms.push(ChainedTransform {
                source_table: "speech_transcription".to_string(),
                target_tables: vec!["semantic_inferences".to_string()],
                domain: "semantic".to_string(),
                source_record_id: transcription_id,
                transform_stage: "semantic_parsing".to_string(),
            });
        }

        tracing::info!(
            records_read,
            records_written,
            records_failed,
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
