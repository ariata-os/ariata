//! StreamWriter - In-memory buffer for direct transform architecture
//!
//! Simplified writer that ONLY buffers records in memory.
//! S3 archival is handled separately by async archive jobs.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::error::Result;

/// Buffer for a single stream
///
/// Note: source_id and stream_name are encoded in the HashMap key,
/// so we don't need to store them redundantly here.
struct StreamBuffer {
    records: Vec<Value>,
    min_timestamp: Option<DateTime<Utc>>,
    max_timestamp: Option<DateTime<Utc>>,
}

impl StreamBuffer {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            min_timestamp: None,
            max_timestamp: None,
        }
    }

    fn add_record(&mut self, record: Value, timestamp: Option<DateTime<Utc>>) {
        // Update timestamp range
        if let Some(ts) = timestamp {
            self.min_timestamp = Some(match self.min_timestamp {
                Some(min) if ts < min => ts,
                Some(min) => min,
                None => ts,
            });
            self.max_timestamp = Some(match self.max_timestamp {
                Some(max) if ts > max => ts,
                Some(max) => max,
                None => ts,
            });
        }

        self.records.push(record);
    }
}

/// In-memory stream writer for direct transform architecture
pub struct StreamWriter {
    buffers: HashMap<String, StreamBuffer>,
}

impl StreamWriter {
    /// Create a new stream writer
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Write a record to in-memory buffer
    ///
    /// Records accumulate in memory until extracted via `collect_records()`.
    pub fn write_record(
        &mut self,
        source_id: Uuid,
        stream_name: &str,
        record: Value,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let buffer_key = format!("{}:{}", source_id, stream_name);

        let buffer = self.buffers
            .entry(buffer_key)
            .or_insert_with(StreamBuffer::new);

        buffer.add_record(record, timestamp);
        Ok(())
    }

    /// Collect all buffered records for a stream and clear the buffer
    ///
    /// Returns: (records, min_timestamp, max_timestamp)
    pub fn collect_records(
        &mut self,
        source_id: Uuid,
        stream_name: &str,
    ) -> Option<(Vec<Value>, Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
        let buffer_key = format!("{}:{}", source_id, stream_name);

        self.buffers.remove(&buffer_key).and_then(|buffer| {
            if buffer.records.is_empty() {
                None
            } else {
                Some((buffer.records, buffer.min_timestamp, buffer.max_timestamp))
            }
        })
    }

    /// Get record count for a stream (for monitoring)
    pub fn buffer_count(&self, source_id: Uuid, stream_name: &str) -> usize {
        let buffer_key = format!("{}:{}", source_id, stream_name);
        self.buffers.get(&buffer_key).map_or(0, |b| b.records.len())
    }
}

impl Default for StreamWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_buffer_and_collect() {
        let mut writer = StreamWriter::new();
        let source_id = Uuid::new_v4();
        let stream_name = "test_stream";

        // Write records
        writer.write_record(
            source_id,
            stream_name,
            json!({"value": 1}),
            None,
        ).unwrap();

        writer.write_record(
            source_id,
            stream_name,
            json!({"value": 2}),
            None,
        ).unwrap();

        assert_eq!(writer.buffer_count(source_id, stream_name), 2);

        // Collect records
        let result = writer.collect_records(source_id, stream_name);
        assert!(result.is_some());

        let (records, _, _) = result.unwrap();
        assert_eq!(records.len(), 2);

        // Buffer should be empty after collection
        assert_eq!(writer.buffer_count(source_id, stream_name), 0);
    }

    #[test]
    fn test_timestamp_tracking() {
        let mut writer = StreamWriter::new();
        let source_id = Uuid::new_v4();
        let stream_name = "test_stream";

        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::hours(1);

        writer.write_record(source_id, stream_name, json!({"value": 1}), Some(ts2)).unwrap();
        writer.write_record(source_id, stream_name, json!({"value": 2}), Some(ts1)).unwrap();

        let result = writer.collect_records(source_id, stream_name).unwrap();
        assert_eq!(result.1, Some(ts1)); // min
        assert_eq!(result.2, Some(ts2)); // max
    }
}
