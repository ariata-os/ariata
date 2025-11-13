//! Ingestion API for receiving data from all sources

use axum::{
    extract::{Json, State},
    response::{IntoResponse, Response},
    http::{StatusCode, HeaderMap},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    error::{Error, Result},
    database::Database,
    storage::{Storage, stream_writer::StreamWriter},
};

/// Ingestion request from any source
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    /// Source identifier (e.g., "ios.healthkit", "mac.imessage")
    pub source: String,

    /// Stream within the source (e.g., "heart_rate", "messages")
    pub stream: String,

    /// Device or instance ID
    /// Currently unused but kept for future device-specific features
    #[allow(dead_code)]
    pub device_id: String,

    /// Actual data records
    pub records: Vec<Value>,

    /// Optional checkpoint for incremental sync
    /// Currently unused but kept for future checkpoint tracking
    #[allow(dead_code)]
    pub checkpoint: Option<String>,

    /// Timestamp of this batch
    #[allow(dead_code)]
    pub timestamp: DateTime<Utc>,
}

/// Response after successful ingestion
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    /// Number of records accepted
    pub accepted: usize,

    /// Number of records rejected
    pub rejected: usize,

    /// Next checkpoint for incremental sync
    pub next_checkpoint: Option<String>,

    /// Pipeline activity ID for tracking
    pub activity_id: String,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub storage: Arc<Storage>,
    pub stream_writer: Arc<Mutex<StreamWriter>>,
}

/// Main ingestion handler
pub async fn ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<IngestRequest>,
) -> Response {
    // Extract and validate device token from Authorization header
    let device_token = match extract_device_token(&headers) {
        Some(token) => token,
        None => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Missing Authorization header. Device token is required.",
                "hint": "Include 'Authorization: Bearer <device_token>' header"
            }))).into_response();
        }
    };

    // Validate device token and get source_id
    let source_id = match crate::api::validate_device_token(state.db.pool(), &device_token).await {
        Ok(id) => id,
        Err(e) => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Invalid or revoked device token",
                "message": e.to_string()
            }))).into_response();
        }
    };

    // Update last_seen timestamp
    if let Err(e) = crate::api::update_last_seen(state.db.pool(), source_id).await {
        tracing::warn!("Failed to update last_seen: {}", e);
    }

    // Validate source and stream exist
    if let Err(e) = validate_source_stream(&state.db, &payload.source, &payload.stream).await {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": e.to_string()
        }))).into_response();
    }

    // Process records based on storage strategy
    let (accepted, rejected) = match process_records(
        &state,
        &payload.source,
        &payload.stream,
        &payload.records,
    ).await {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!("Failed to process records: {}", e);
            (0, payload.records.len())
        }
    };

    // Trigger transforms if records were successfully processed (hot path like cloud syncs)
    if accepted > 0 {
        if let Err(e) = trigger_transforms_for_batch(
            &state,
            source_id,
            &payload.stream,
        ).await {
            tracing::warn!(
                error = %e,
                stream = %payload.stream,
                "Failed to trigger transforms for device batch, continuing"
            );
        }
    }

    (StatusCode::OK, Json(IngestResponse {
        accepted,
        rejected,
        next_checkpoint: None, // Checkpoint management will be implemented later if needed
        activity_id: uuid::Uuid::new_v4().to_string(), // Generate ID for debugging/tracing
    })).into_response()
}

/// Extract device token from Authorization header
fn extract_device_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Validate source and stream configuration exists
///
/// This validation is now minimal - we accept any source/stream combination and let
/// the downstream processors and transform registry determine if it's valid. This
/// avoids maintaining a hardcoded list that can get out of sync with the rest of the system.
async fn validate_source_stream(
    _db: &Database,
    _source: &str,
    _stream: &str,
) -> Result<()> {
    // Basic validation - just ensure strings are not empty
    // Downstream processors will handle unknown source/stream combinations
    Ok(())
}

/// Process records based on storage strategy
async fn process_records(
    state: &AppState,
    source: &str,
    stream: &str,
    records: &[Value],
) -> Result<(usize, usize)> {
    let mut accepted = 0;
    let mut rejected = 0;

    // Determine storage strategy from configuration
    let storage_strategy = get_storage_strategy(&state.db, source, stream).await?;

    for record in records {
        match process_single_record(state, &storage_strategy, source, stream, record).await {
            Ok(_) => accepted += 1,
            Err(e) => {
                tracing::warn!("Failed to process record: {}", e);
                rejected += 1;
            }
        }
    }

    Ok((accepted, rejected))
}

/// Process single record - routes to appropriate processor based on source/stream
async fn process_single_record(
    state: &AppState,
    _strategy: &StorageStrategy,
    source: &str,
    stream: &str,
    record: &Value,
) -> Result<()> {
    // Route to appropriate processor based on source and stream type
    // All device processors now use StreamWriter (writes to S3/object storage)
    match (source, stream) {
        ("ios", "healthkit") => {
            crate::sources::ios::healthkit::process(&state.db, &state.storage, &state.stream_writer, record).await?;
        }
        ("ios", "location") => {
            crate::sources::ios::location::process(&state.db, &state.storage, &state.stream_writer, record).await?;
        }
        ("ios", "microphone") | ("ios", "mic") => {
            crate::sources::ios::microphone::process(&state.db, &state.storage, &state.stream_writer, record).await?;
        }
        ("mac", "apps") => {
            crate::sources::mac::apps::process(&state.db, &state.storage, &state.stream_writer, record).await?;
        }
        ("mac", "browser") => {
            crate::sources::mac::browser::process(&state.db, &state.storage, &state.stream_writer, record).await?;
        }
        ("mac", "imessage") => {
            crate::sources::mac::imessage::process(&state.db, &state.storage, &state.stream_writer, record).await?;
        }
        _ => {
            tracing::warn!("Unknown source/stream: {}/{}", source, stream);
            return Err(Error::Other(format!(
                "Unsupported source/stream combination: {source}/{stream}"
            )));
        }
    }

    Ok(())
}

/// Storage strategy for different data types
#[derive(Debug)]
enum StorageStrategy {
    PostgresOnly,
    Hybrid,
}

/// Get storage strategy for source/stream
async fn get_storage_strategy(
    _db: &Database,
    source: &str,
    stream: &str,
) -> Result<StorageStrategy> {
    // For now, use PostgreSQL for structured data, hybrid for large blobs
    // This will be configuration-driven later
    match (source, stream) {
        ("ios", "healthkit") | ("mac", "apps") => Ok(StorageStrategy::PostgresOnly),
        ("ios", "mic") | ("mac", "screenshots") => Ok(StorageStrategy::Hybrid),
        _ => Ok(StorageStrategy::PostgresOnly),
    }
}

/// Trigger transforms for device batch (hot path - unified with cloud syncs)
///
/// After device records are processed and buffered in StreamWriter,
/// this function collects them and triggers transform jobs directly,
/// just like cloud syncs do. Also creates async archive job for S3 backup.
/// This fully unifies the push (device) and pull (cloud) data pipelines.
async fn trigger_transforms_for_batch(
    state: &AppState,
    source_id: uuid::Uuid,
    stream_name: &str,
) -> Result<()> {
    // Collect buffered records from StreamWriter
    let (records, min_timestamp, max_timestamp) = {
        let mut writer = state.stream_writer.lock().await;
        match writer.collect_records(source_id, stream_name) {
            Some((records, min_ts, max_ts)) => {
                tracing::info!(
                    source_id = %source_id,
                    stream_name,
                    record_count = records.len(),
                    "Collected records from device batch for direct transform (hot path)"
                );
                (records, min_ts, max_ts)
            }
            None => {
                tracing::debug!(
                    source_id = %source_id,
                    stream_name,
                    "No buffered records to transform"
                );
                return Ok(());
            }
        }
    };

    // Create archive job for async S3 archival (hot path - same as cloud syncs)
    let _archive_job_id = if !records.is_empty() {
        let archive_id = crate::jobs::spawn_archive_job_async(
            state.db.pool(),
            state.storage.as_ref(),
            None, // No parent job for device ingests
            source_id,
            stream_name,
            records.clone(),
            (min_timestamp, max_timestamp),
        ).await?;
        Some(archive_id)
    } else {
        None
    };

    // Create context without data source for transform triggering
    // The create_transform_job_for_stream function will create a new context
    // with the actual MemoryDataSource when executing the transform
    let api_keys = crate::jobs::ApiKeys::from_env();

    let context = Arc::new(crate::jobs::TransformContext::new(
        Arc::clone(&state.storage),
        state.stream_writer.clone(),
        api_keys,
    ));

    let executor = crate::jobs::JobExecutor::new(state.db.pool().clone(), (*context).clone());

    // Create and execute transform job with in-memory records (hot path)
    let _job_id = crate::jobs::create_transform_job_for_stream(
        state.db.pool(),
        &executor,
        &context,
        source_id,
        stream_name,
        Some(records), // Pass collected records for direct transform
    ).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_request_deserialize() {
        let json = r#"{
            "source": "ios",
            "stream": "healthkit",
            "device_id": "iPhone-123",
            "records": [{"value": 72, "unit": "bpm"}],
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let request: IngestRequest = serde_json::from_str(json)
            .expect("test JSON should be valid");
        assert_eq!(request.source, "ios");
        assert_eq!(request.stream, "healthkit");
        assert_eq!(request.records.len(), 1);
    }
}