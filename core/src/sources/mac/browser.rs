//! macOS browser history data processor

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

/// Mac Browser stream implementing PushStream trait
///
/// Receives browser history data pushed from macOS devices via /ingest endpoint.
pub struct MacBrowserStream {
    db: PgPool,
    stream_writer: Arc<Mutex<StreamWriter>>,
}

impl MacBrowserStream {
    /// Create a new MacBrowserStream
    pub fn new(db: PgPool, stream_writer: Arc<Mutex<StreamWriter>>) -> Self {
        Self { db, stream_writer }
    }
}

#[async_trait]
impl PushStream for MacBrowserStream {
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
                writer.write_record(source_id, "browser", record.clone(), Some(timestamp_dt))?;
            }

            result.records_written += 1;
        }

        tracing::info!(
            "Processed {} browser records from device {}",
            result.records_written,
            payload.device_id
        );

        Ok(result)
    }

    fn table_name(&self) -> &str {
        "stream_mac_browser"
    }

    fn stream_name(&self) -> &str {
        "browser"
    }

    fn source_name(&self) -> &str {
        "mac"
    }
}
