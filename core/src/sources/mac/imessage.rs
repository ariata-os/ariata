//! macOS iMessage/SMS data processor

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
    storage::stream_writer::StreamWriter,
};

/// Mac iMessage stream implementing PushStream trait
///
/// Receives iMessage/SMS data pushed from macOS devices via /ingest endpoint.
pub struct MacIMessageStream {
    _db: PgPool,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl MacIMessageStream {
    /// Create a new MacIMessageStream
    pub fn new(db: PgPool, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self { _db: db, stream_writer }
    }
}

#[async_trait]
impl PushStream for MacIMessageStream {
    async fn receive_push(&self, source_id: Uuid, payload: IngestPayload) -> Result<PushResult> {
        // Validate payload
        self.validate_payload(&payload)?;

        let mut result = PushResult::new(payload.records.len());

        // source_id is passed from handler - single source of truth, no duplicate DB query

        // Process each record
        for record in &payload.records {
            // Extract timestamp
            let timestamp = record
                .get("timestamp")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing timestamp in record".into()))?;

            let timestamp_dt = chrono::DateTime::parse_from_rfc3339(timestamp)
                .map_err(|e| Error::Other(format!("Invalid timestamp format: {e}")))?
                .with_timezone(&Utc);

            validate_timestamp_reasonable(timestamp_dt)?;

            // Write to object storage via StreamWriter
            {
                let mut writer = self.stream_writer.lock().await;
                writer.write_record(source_id, "imessage", record.clone(), Some(timestamp_dt))?;
            }

            result.records_written += 1;
        }

        tracing::info!(
            "Processed {} iMessage records from device {}",
            result.records_written,
            payload.device_id
        );

        Ok(result)
    }

    fn table_name(&self) -> &str {
        "stream_mac_imessage"
    }

    fn stream_name(&self) -> &str {
        "imessage"
    }

    fn source_name(&self) -> &str {
        "mac"
    }
}
