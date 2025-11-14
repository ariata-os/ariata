use crate::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ASSEMBLYAI_API_BASE: &str = "https://api.assemblyai.com/v2";

/// AssemblyAI transcription client
pub struct AssemblyAIClient {
    client: Client,
    api_key: String,
}

/// Request to submit audio for transcription
#[derive(Debug, Serialize)]
struct TranscriptRequest {
    audio_url: String,

    // Disable features we don't need to reduce cost and latency
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_labels: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    punctuate: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    format_text: Option<bool>,
}

/// Response from submitting transcription job
#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    id: String,
    status: String,

    #[serde(default)]
    text: Option<String>,

    #[serde(default)]
    confidence: Option<f64>,

    #[serde(default)]
    language_code: Option<String>,

    #[serde(default)]
    error: Option<String>,
}

/// Response from uploading a file to AssemblyAI
#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

/// Result of a completed transcription
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f64,
    pub language: Option<String>,
}

impl AssemblyAIClient {
    /// Create a new AssemblyAI client
    ///
    /// # Arguments
    /// * `api_key` - AssemblyAI API key
    pub fn new(api_key: String) -> Self {
        // Configure HTTP client with timeouts (following oauth_client.rs pattern)
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10)) // TCP connection timeout
            .timeout(Duration::from_secs(60)) // Total request timeout
            .build()
            .expect("Failed to build HTTP client");

        Self { client, api_key }
    }

    /// Transcribe audio from a URL
    ///
    /// This method:
    /// 1. Submits the audio URL to AssemblyAI
    /// 2. Polls for completion (with exponential backoff)
    /// 3. Returns the transcription result
    ///
    /// # Arguments
    /// * `audio_url` - Publicly accessible URL to the audio file (e.g., pre-signed S3 URL)
    ///
    /// # Returns
    /// * `Ok(TranscriptionResult)` - Transcription completed successfully
    /// * `Err(Error)` - Network error, API error, or transcription failed
    pub async fn transcribe_url(&self, audio_url: &str) -> Result<TranscriptionResult> {
        // Step 1: Submit transcription job
        let transcript_id = self.submit_transcription(audio_url).await?;

        // Step 2: Poll for completion with exponential backoff
        let result = self.poll_for_completion(&transcript_id).await?;

        Ok(result)
    }

    /// Transcribe audio by uploading bytes directly to AssemblyAI
    ///
    /// This is useful for local development where the storage backend (MinIO) is not
    /// publicly accessible. Instead of giving AssemblyAI a URL, we upload the file
    /// directly to their service first.
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio file bytes
    ///
    /// # Returns
    /// * `Ok(TranscriptionResult)` - Transcription completed successfully
    /// * `Err(Error)` - Network error, API error, or transcription failed
    pub async fn transcribe_bytes(&self, audio_data: Vec<u8>) -> Result<TranscriptionResult> {
        // Step 1: Upload file to AssemblyAI and get URL
        let upload_url = self.upload_file(audio_data).await?;

        // Step 2: Submit transcription job with the upload URL
        let transcript_id = self.submit_transcription(&upload_url).await?;

        // Step 3: Poll for completion with exponential backoff
        let result = self.poll_for_completion(&transcript_id).await?;

        Ok(result)
    }

    /// Upload audio file directly to AssemblyAI
    ///
    /// Returns a URL that can be used for transcription
    async fn upload_file(&self, audio_data: Vec<u8>) -> Result<String> {
        let response = self
            .client
            .post(&format!("{}/upload", ASSEMBLYAI_API_BASE))
            .header("authorization", &self.api_key)
            .header("content-type", "application/octet-stream")
            .body(audio_data)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Failed to upload file: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Http(format!(
                "AssemblyAI upload error ({}): {}",
                status, error_text
            )));
        }

        let upload_response: UploadResponse = response
            .json()
            .await
            .map_err(|e| Error::Other(format!("Failed to parse upload response: {}", e)))?;

        Ok(upload_response.upload_url)
    }

    /// Submit audio URL for transcription
    async fn submit_transcription(&self, audio_url: &str) -> Result<String> {
        let request = TranscriptRequest {
            audio_url: audio_url.to_string(),
            speaker_labels: Some(false), // Explicitly disable diarization
            punctuate: Some(true),       // Enable punctuation for readability
            format_text: Some(true),     // Enable text formatting
        };

        let response = self
            .client
            .post(&format!("{}/transcript", ASSEMBLYAI_API_BASE))
            .header("authorization", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Failed to submit transcription: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Http(format!(
                "AssemblyAI API error ({}): {}",
                status, error_text
            )));
        }

        let transcript_response: TranscriptResponse = response
            .json()
            .await
            .map_err(|e| Error::Other(format!("Failed to parse response: {}", e)))?;

        Ok(transcript_response.id)
    }

    /// Poll for transcription completion with exponential backoff
    ///
    /// Maximum wait time: ~10 minutes (120 attempts * 5 seconds)
    async fn poll_for_completion(&self, transcript_id: &str) -> Result<TranscriptionResult> {
        let max_attempts = 120; // 10 minutes max
        let mut attempt = 0;
        let mut backoff_seconds = 3;

        loop {
            if attempt >= max_attempts {
                return Err(Error::Other(format!(
                    "Transcription timeout after {} attempts",
                    max_attempts
                )));
            }

            // Wait before polling (skip on first attempt)
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
            }

            // Query transcription status
            let response = self
                .client
                .get(&format!(
                    "{}/transcript/{}",
                    ASSEMBLYAI_API_BASE, transcript_id
                ))
                .header("authorization", &self.api_key)
                .send()
                .await
                .map_err(|e| Error::Network(format!("Failed to poll transcription: {}", e)))?;

            if !response.status().is_success() {
                return Err(Error::Http(format!(
                    "AssemblyAI API error: {}",
                    response.status()
                )));
            }

            let status: TranscriptResponse = response
                .json()
                .await
                .map_err(|e| Error::Other(format!("Failed to parse status response: {}", e)))?;

            match status.status.as_str() {
                "completed" => {
                    let text = status.text.ok_or_else(|| {
                        Error::Other("Transcription completed but no text returned".into())
                    })?;

                    return Ok(TranscriptionResult {
                        text,
                        confidence: status.confidence.unwrap_or(0.0),
                        language: status.language_code,
                    });
                }
                "error" => {
                    return Err(Error::Http(format!(
                        "Transcription failed: {}",
                        status.error.unwrap_or_else(|| "Unknown error".to_string())
                    )));
                }
                "queued" | "processing" => {
                    // Continue polling
                    attempt += 1;

                    // Exponential backoff: 3s, 5s, 5s, 5s, ...
                    if backoff_seconds < 5 {
                        backoff_seconds += 1;
                    }

                    tracing::debug!(
                        "Transcription {} status: {}, attempt {}/{}",
                        transcript_id,
                        status.status,
                        attempt,
                        max_attempts
                    );
                }
                other => {
                    tracing::warn!("Unknown transcription status: {}", other);
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AssemblyAIClient::new("test-api-key".to_string());
        assert_eq!(client.api_key, "test-api-key");
    }

    // Integration tests would require a real API key
    // Add mock tests here as needed
}
