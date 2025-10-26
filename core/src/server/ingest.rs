//! Ingestion API for receiving data from all sources

use axum::{
    extract::{Json, State, Query},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::{
    error::{Error, Result},
    database::Database,
    storage::Storage,
};

/// Ingestion request from any source
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    /// Source identifier (e.g., "ios.healthkit", "mac.imessage")
    pub source: String,

    /// Stream within the source (e.g., "heart_rate", "messages")
    pub stream: String,

    /// Device or instance ID
    pub device_id: String,

    /// Actual data records
    pub records: Vec<Value>,

    /// Optional checkpoint for incremental sync
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

/// Authentication query parameters
#[derive(Debug, Deserialize)]
pub struct AuthQuery {
    pub device_token: Option<String>,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub storage: Arc<Storage>,
}

/// Main ingestion handler
pub async fn ingest(
    State(state): State<AppState>,
    Query(auth_query): Query<AuthQuery>,
    Json(payload): Json<IngestRequest>,
) -> Response {
    // Validate authentication
    if let Some(token) = auth_query.device_token {
        if let Err(e) = validate_device_token(&state.db, &token, &payload.device_id).await {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response();
        }
    } else {
        // For now, allow without token for testing
        tracing::warn!("Ingestion without device token - allowing for development");
    }

    // Validate source and stream exist
    if let Err(e) = validate_source_stream(&state.db, &payload.source, &payload.stream).await {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": e.to_string()
        }))).into_response();
    }

    // Create pipeline activity record
    let activity_id = match create_pipeline_activity(
        &state.db,
        &payload.source,
        &payload.stream,
        &payload.device_id,
        payload.records.len(),
    ).await {
        Ok(id) => id,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response();
        }
    };

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

    // Update checkpoint if provided
    let next_checkpoint = if let Some(checkpoint) = payload.checkpoint {
        if let Err(e) = update_checkpoint(
            &state.db,
            &payload.source,
            &payload.stream,
            &payload.device_id,
            &checkpoint,
        ).await {
            tracing::warn!("Failed to update checkpoint: {}", e);
            None
        } else {
            Some(generate_next_checkpoint(&checkpoint))
        }
    } else {
        None
    };

    // Update pipeline activity status
    if let Err(e) = update_pipeline_activity(&state.db, &activity_id, "completed", accepted).await {
        tracing::warn!("Failed to update pipeline activity: {}", e);
    }

    (StatusCode::OK, Json(IngestResponse {
        accepted,
        rejected,
        next_checkpoint,
        activity_id,
    })).into_response()
}

/// Validate device token
async fn validate_device_token(
    _db: &Database,
    _token: &str,
    _device_id: &str,
) -> Result<()> {
    // SECURITY: Token validation not implemented!
    // Device sources currently have no authentication.
    //
    // For production deployment, implement one of:
    // 1. JWT-based device tokens (signed by auth server)
    // 2. API keys stored in database with rate limiting
    // 3. mTLS client certificates for device authentication
    //
    // Current behavior: Accept all requests (suitable for single-user localhost only)
    Ok(())
}

/// Validate source and stream configuration exists
async fn validate_source_stream(
    _db: &Database,
    source: &str,
    stream: &str,
) -> Result<()> {
    // List of valid source/stream combinations
    let valid_combinations = [
        // iOS streams
        ("ios", "healthkit"),
        ("ios", "location"),
        ("ios", "microphone"),
        ("ios", "mic"),  // alias for microphone

        // Mac streams
        ("mac", "apps"),
        ("mac", "browser"),
        ("mac", "imessage"),
        ("mac", "screentime"),

        // Cloud OAuth sources
        ("google", "calendar"),
        ("google", "gmail"),
        ("strava", "activities"),
        ("notion", "pages"),
    ];

    let is_valid = valid_combinations
        .iter()
        .any(|(s, st)| *s == source && *st == stream);

    if !is_valid {
        return Err(Error::Other(format!("Invalid source/stream: {source}/{stream}")));
    }

    Ok(())
}

/// Create pipeline activity record
async fn create_pipeline_activity(
    _db: &Database,
    source: &str,
    stream: &str,
    device_id: &str,
    record_count: usize,
) -> Result<String> {
    let activity_id = uuid::Uuid::new_v4().to_string();

    // NOTE: Pipeline activity tracking not persisted to database yet.
    // This would enable features like:
    // - Ingestion history and audit trail
    // - Progress tracking for large uploads
    // - Retry logic for failed batches
    //
    // Current behavior: Log activity ID for debugging, return immediately
    tracing::info!(
        "Created pipeline activity {} for {}/{} from device {} with {} records",
        activity_id, source, stream, device_id, record_count
    );

    Ok(activity_id)
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
    match (source, stream) {
        ("ios", "healthkit") => {
            crate::sources::ios::healthkit::process(&state.db, &state.storage, record).await?;
        }
        ("ios", "location") => {
            crate::sources::ios::location::process(&state.db, &state.storage, record).await?;
        }
        ("ios", "microphone") | ("ios", "mic") => {
            crate::sources::ios::microphone::process(&state.db, &state.storage, record).await?;
        }
        ("mac", "apps") => {
            crate::sources::mac::apps::process(&state.db, &state.storage, record).await?;
        }
        ("mac", "browser") => {
            crate::sources::mac::browser::process(&state.db, &state.storage, record).await?;
        }
        ("mac", "imessage") => {
            crate::sources::mac::imessage::process(&state.db, &state.storage, record).await?;
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

/// Update sync checkpoint
///
/// NOTE: Checkpoint persistence not implemented yet.
/// For device sources using push model, checkpoints are less critical since
/// devices maintain their own state. For future pull-based device syncs,
/// implement checkpoint storage in the `sources` table.
async fn update_checkpoint(
    _db: &Database,
    source: &str,
    stream: &str,
    device_id: &str,
    checkpoint: &str,
) -> Result<()> {
    tracing::debug!(
        "Checkpoint update (not persisted): {}/{} device {} -> {}",
        source, stream, device_id, checkpoint
    );
    Ok(())
}

/// Generate next checkpoint
fn generate_next_checkpoint(current: &str) -> String {
    // Simple increment for now, will be more sophisticated based on source type
    format!("{current}_next")
}

/// Update pipeline activity status
///
/// NOTE: Activity status tracking not persisted (see `create_pipeline_activity`).
async fn update_pipeline_activity(
    _db: &Database,
    activity_id: &str,
    status: &str,
    records_processed: usize,
) -> Result<()> {
    tracing::debug!(
        "Activity update (not persisted): {} -> {} ({} records)",
        activity_id, status, records_processed
    );
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

        let request: IngestRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.source, "ios");
        assert_eq!(request.stream, "healthkit");
        assert_eq!(request.records.len(), 1);
    }
}