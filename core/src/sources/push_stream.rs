use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Result;

/// Trait for sources where the client initiates synchronization
/// by pushing data to the backend (e.g., Mac app, iOS app).
///
/// Push streams are characterized by:
/// - Client controls when sync happens (whenever it has new data)
/// - Data lives on user's device, not accessible via API
/// - Uses device tokens for authentication
/// - Backend is passive receiver via /ingest endpoint
/// - Client may be offline/unavailable
///
/// # Examples
///
/// ```ignore
/// impl PushStream for MacAppsStream {
///     async fn receive_push(&self, source_id: Uuid, payload: IngestPayload) -> Result<PushResult> {
///         // 1. Validate payload structure
///         // 2. Transform records if needed
///         // 3. Write records to StreamWriter using source_id
///         // 4. Return stats
///     }
/// }
/// ```
#[async_trait]
pub trait PushStream: Send + Sync {
    /// Client initiates: receive and process data pushed from device
    ///
    /// This method should:
    /// 1. Validate payload structure and device_id
    /// 2. Transform records if needed
    /// 3. Write records to StreamWriter
    /// 4. Return statistics about what was received
    ///
    /// # Arguments
    /// * `source_id` - The source connection ID (created by handler, single source of truth)
    /// * `payload` - The data pushed from the device
    async fn receive_push(&self, source_id: Uuid, payload: IngestPayload) -> Result<PushResult>;

    /// Table name in data schema (e.g., "stream_mac_apps")
    fn table_name(&self) -> &str;

    /// Stream identifier (e.g., "apps", "imessage", "location")
    fn stream_name(&self) -> &str;

    /// Provider identifier (e.g., "mac", "ios")
    fn source_name(&self) -> &str;

    /// Whether this stream requires device token authentication
    ///
    /// Default is true for all device streams
    fn requires_device_auth(&self) -> bool {
        true
    }

    /// Optional: Validate payload structure before processing
    ///
    /// Default implementation checks for device_id and non-empty records.
    /// Override to add stream-specific validation.
    fn validate_payload(&self, payload: &IngestPayload) -> Result<()> {
        if payload.device_id.is_empty() {
            return Err(crate::Error::MissingDeviceId);
        }
        if payload.records.is_empty() {
            return Err(crate::Error::EmptyPayload);
        }
        Ok(())
    }
}

/// Payload sent by device apps via POST /ingest
///
/// # Example
///
/// ```json
/// {
///   "source": "mac",
///   "stream": "apps",
///   "device_id": "MAC-123e4567-e89b-12d3-a456-426614174000",
///   "records": [
///     {
///       "app_name": "Claude",
///       "bundle_id": "com.anthropic.claude",
///       "event_type": "activate",
///       "timestamp": "2024-01-15T10:30:00Z"
///     }
///   ],
///   "timestamp": "2024-01-15T10:30:05Z"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestPayload {
    /// Provider name (e.g., "mac", "ios")
    pub source: String,

    /// Stream name (e.g., "apps", "imessage", "location")
    pub stream: String,

    /// Unique device identifier
    ///
    /// Format: {PLATFORM}-{UUID}
    /// Examples: "MAC-123e4567...", "IOS-987f6543..."
    pub device_id: String,

    /// Array of records to ingest
    ///
    /// Schema varies by stream type. Each stream validates its own records.
    pub records: Vec<serde_json::Value>,

    /// When the batch was created on device
    ///
    /// This is the client's timestamp, may differ from received_at
    pub timestamp: DateTime<Utc>,
}

/// Result of a push operation
#[derive(Debug, Clone)]
pub struct PushResult {
    /// Number of records received from client
    pub records_received: usize,

    /// Number of records written to database
    pub records_written: usize,

    /// When the push was received by backend
    pub received_at: DateTime<Utc>,
}

impl PushResult {
    /// Create a new PushResult with the current timestamp
    pub fn new(records_received: usize) -> Self {
        Self {
            records_received,
            records_written: 0,
            received_at: Utc::now(),
        }
    }

    /// Create a PushResult with both received and written counts
    pub fn with_counts(received: usize, written: usize) -> Self {
        Self {
            records_received: received,
            records_written: written,
            received_at: Utc::now(),
        }
    }
}
