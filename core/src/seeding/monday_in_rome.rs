//! Monday in Rome reference dataset seeding
//!
//! Seeds the database with a real-world reference dataset from a full day in Rome, Italy.
//! This module:
//! - Loads reduced CSVs from core/seeds/monday_in_rome/
//! - Converts them to JSON records
//! - Triggers the full data pipeline: Archive job (S3) + Transform job (ontology tables)
//!
//! This tests the complete Ariata pipeline end-to-end with realistic data volumes.

use crate::{
    database::Database,
    error::{Error, Result},
    jobs::{
        self, spawn_archive_job_async, transform_trigger::create_transform_job_for_stream, ApiKeys,
        JobExecutor, TransformContext,
    },
    storage::{stream_writer::StreamWriter, Storage},
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

/// Device metadata from Monday in Rome recording
const DEVICE_ID: &str = "a1162b36-4606-4b50-a875-8be0f7274ff0";
const DEVICE_NAME: &str = "iPhone 17 Pro Max";
const RECORDING_TIMEZONE: &str = "Europe/Rome";

/// Get or create the test source for Monday in Rome dataset
async fn get_or_create_test_source(db: &Database) -> Result<Uuid> {
    // Check if test source already exists
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM elt.sources WHERE name = 'monday-in-rome' LIMIT 1",
    )
    .fetch_optional(db.pool())
    .await?;

    if let Some(id) = existing {
        info!("Using existing monday-in-rome source: {}", id);
        return Ok(id);
    }

    // Create test source
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO elt.sources (name, provider, auth_type, is_active)
         VALUES ('monday-in-rome', 'ios', 'device', true)
         RETURNING id",
    )
    .fetch_one(db.pool())
    .await?;

    info!("Created monday-in-rome source: {}", id);
    Ok(id)
}

/// Load a CSV file and convert to JSON records
fn load_csv_to_records(csv_path: &PathBuf, stream_name: &str) -> Result<Vec<Value>> {
    let file_content = std::fs::read_to_string(csv_path)
        .map_err(|e| Error::Other(format!("Failed to read {}: {}", csv_path.display(), e)))?;

    let mut rdr = csv::Reader::from_reader(file_content.as_bytes());
    let headers = rdr
        .headers()
        .map_err(|e| Error::Other(format!("Failed to read CSV headers: {}", e)))?
        .clone();

    let mut records = Vec::new();

    for result in rdr.records() {
        let record =
            result.map_err(|e| Error::Other(format!("Failed to read CSV record: {}", e)))?;

        // Convert CSV row to JSON
        let mut json_obj = serde_json::Map::new();

        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                // Parse numeric fields, keep strings as-is
                let value = if field.is_empty() {
                    Value::Null
                } else if let Ok(num) = field.parse::<f64>() {
                    json!(num)
                } else if field == "true" || field == "True" {
                    json!(true)
                } else if field == "false" || field == "False" {
                    json!(false)
                } else {
                    json!(field)
                };

                json_obj.insert(header.to_string(), value);
            }
        }

        // Add metadata fields required by pipeline
        json_obj.insert("device_id".to_string(), json!(DEVICE_ID));
        json_obj.insert("device_name".to_string(), json!(DEVICE_NAME));
        json_obj.insert("timezone".to_string(), json!(RECORDING_TIMEZONE));

        // Convert timestamp from nanoseconds if needed
        if let Some(time_nanos) = json_obj.get("time").and_then(|v| v.as_f64()) {
            // Convert nanoseconds since epoch to RFC3339
            let timestamp = DateTime::<Utc>::from_timestamp(
                (time_nanos / 1_000_000_000.0) as i64,
                (time_nanos % 1_000_000_000.0) as u32,
            )
            .ok_or_else(|| Error::Other("Invalid timestamp".into()))?;

            json_obj.insert("timestamp".to_string(), json!(timestamp.to_rfc3339()));
        }

        records.push(Value::Object(json_obj));
    }

    info!(
        stream = stream_name,
        record_count = records.len(),
        "Loaded CSV file: {}",
        csv_path.display()
    );

    Ok(records)
}

/// Seed a single stream through the full pipeline
async fn seed_stream_pipeline(
    db: &Database,
    storage: &Storage,
    stream_writer: Arc<Mutex<StreamWriter>>,
    source_id: Uuid,
    stream_name: &str,
    records: Vec<Value>,
) -> Result<()> {
    if records.is_empty() {
        warn!(stream = stream_name, "No records to seed");
        return Ok(());
    }

    info!(
        stream = stream_name,
        record_count = records.len(),
        "Seeding stream through full pipeline"
    );

    // Extract timestamp range for archive job metadata
    let timestamps: Vec<DateTime<Utc>> = records
        .iter()
        .filter_map(|r| {
            r.get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
        })
        .collect();

    let min_ts = timestamps.iter().min().cloned();
    let max_ts = timestamps.iter().max().cloned();

    // Spawn archive job for async S3/MinIO upload (fire-and-forget)
    let archive_id = spawn_archive_job_async(
        db.pool(),
        storage,
        None, // No parent job for seeding
        source_id,
        stream_name,
        records.clone(),
        (min_ts, max_ts),
    )
    .await?;

    info!(
        stream = stream_name,
        archive_job_id = %archive_id,
        "Archive job spawned for S3 upload"
    );

    // Create transform context and executor
    let api_keys = ApiKeys::from_env();
    let context = Arc::new(TransformContext::new(
        Arc::new(storage.clone()),
        stream_writer.clone(),
        api_keys,
    ));
    let executor = JobExecutor::new(db.pool().clone(), (*context).clone());

    // Trigger transform with memory records (hot path)
    let job_id = create_transform_job_for_stream(
        db.pool(),
        &executor,
        &context,
        source_id,
        stream_name,
        Some(records),
    )
    .await?;

    info!(
        stream = stream_name,
        job_id = %job_id,
        "Transform job created, waiting for completion..."
    );

    // Wait for transform to complete (60 second timeout, poll every 500ms)
    match jobs::wait_for_job_completion(db.pool(), job_id, 60, 500).await {
        Ok(completed_job) => {
            info!(
                stream = stream_name,
                job_id = %job_id,
                records_processed = completed_job.records_processed,
                "Transform job completed successfully"
            );
        }
        Err(e) => {
            warn!(
                stream = stream_name,
                job_id = %job_id,
                error = %e,
                "Transform job failed or timed out"
            );
            // Don't fail the entire seeding - just log the warning
        }
    }

    Ok(())
}

/// Seed microphone transcriptions directly to speech_transcription table
///
/// Loads microphone.csv and directly inserts into speech_transcription ontology table,
/// bypassing the transform layer to avoid calling AssemblyAI API during seeding.
async fn seed_microphone_transcriptions(
    db: &Database,
    source_id: Uuid,
    csv_path: &PathBuf,
) -> Result<usize> {
    info!("Loading microphone CSV: {}", csv_path.display());

    let file_content = std::fs::read_to_string(csv_path)
        .map_err(|e| Error::Other(format!("Failed to read microphone CSV: {}", e)))?;

    let mut rdr = csv::Reader::from_reader(file_content.as_bytes());
    let mut count = 0;

    // Create a seed stream ID for microphone (similar to other streams)
    let seed_stream_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO elt.streams (source_id, stream_name, table_name, created_at, updated_at)
         VALUES ($1, 'microphone', 'stream_ios_microphone', NOW(), NOW())
         ON CONFLICT (source_id, stream_name) DO UPDATE SET updated_at = NOW()
         RETURNING id",
    )
    .bind(source_id)
    .fetch_one(db.pool())
    .await?;

    for result in rdr.deserialize() {
        let record: serde_json::Map<String, serde_json::Value> =
            result.map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;

        // Extract fields from CSV
        let time_ns = record
            .get("time")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| Error::Other("Missing time field".into()))?;

        let timestamp = DateTime::<Utc>::from_timestamp(
            (time_ns / 1_000_000_000.0) as i64,
            (time_ns % 1_000_000_000.0) as u32,
        )
        .ok_or_else(|| Error::Other("Invalid timestamp".into()))?;

        let duration_seconds = record
            .get("duration_seconds")
            .and_then(|v| v.as_f64())
            .map(|d| d as i32);

        let transcript_text = record
            .get("transcript_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Missing transcript_text".into()))?;

        let language = record.get("language").and_then(|v| v.as_str());

        let confidence_score = record
            .get("confidence_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let speaker_count = record
            .get("speaker_count")
            .and_then(|v| v.as_i64())
            .map(|c| c as i32);

        let speaker_labels_json = record
            .get("speaker_labels_json")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

        let metadata_json = record
            .get("metadata_json")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .unwrap_or(json!({}));

        // Insert into speech_transcription table
        sqlx::query(
            r#"
            INSERT INTO elt.speech_transcription (
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind("test-day/Microphone.mp4") // audio_file_path
        .bind(duration_seconds)
        .bind(transcript_text)
        .bind(language)
        .bind(confidence_score)
        .bind(speaker_count)
        .bind(speaker_labels_json)
        .bind(timestamp)
        .bind(seed_stream_id)
        .bind("stream_seed_data") // source_table
        .bind("seed") // source_provider
        .bind(metadata_json)
        .execute(db.pool())
        .await
        .map_err(|e| Error::Database(format!("Failed to insert speech_transcription: {}", e)))?;

        count += 1;
    }

    info!("Inserted {} microphone transcription records", count);
    Ok(count)
}

/// Seed sleep data directly to health_sleep table
///
/// Loads sleep.csv and directly inserts into health_sleep ontology table.
/// CSV already contains the final ontology fields, so no transformation needed.
async fn seed_sleep_data(db: &Database, csv_path: &PathBuf) -> Result<usize> {
    info!("Loading sleep CSV: {}", csv_path.display());

    let file_content = std::fs::read_to_string(csv_path)
        .map_err(|e| Error::Other(format!("Failed to read sleep CSV: {}", e)))?;

    let mut rdr = csv::Reader::from_reader(file_content.as_bytes());
    let mut count = 0;

    for result in rdr.deserialize() {
        let record: serde_json::Map<String, serde_json::Value> =
            result.map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;

        // Extract fields from CSV
        let start_time = record
            .get("start_time")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .ok_or_else(|| Error::Other("Missing or invalid start_time".into()))?
            .with_timezone(&Utc);

        let end_time = record
            .get("end_time")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .ok_or_else(|| Error::Other("Missing or invalid end_time".into()))?
            .with_timezone(&Utc);

        let total_duration_minutes = record
            .get("total_duration_minutes")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| Error::Other("Missing total_duration_minutes".into()))?
            as i32;

        let sleep_quality_score = record.get("sleep_quality_score").and_then(|v| v.as_f64());

        let sleep_stages_str = record
            .get("sleep_stages")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Missing sleep_stages".into()))?;

        let sleep_stages: serde_json::Value = serde_json::from_str(sleep_stages_str)
            .map_err(|e| Error::Other(format!("Invalid sleep_stages JSON: {}", e)))?;

        let source_stream_id = record
            .get("source_stream_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| Error::Other("Missing or invalid source_stream_id".into()))?;

        let source_table = record
            .get("source_table")
            .and_then(|v| v.as_str())
            .unwrap_or("stream_seed_health_sleep");

        let source_provider = record
            .get("source_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("seed");

        // Insert into health_sleep table
        sqlx::query(
            r#"
            INSERT INTO elt.health_sleep (
                start_time,
                end_time,
                total_duration_minutes,
                sleep_quality_score,
                sleep_stages,
                source_stream_id,
                source_table,
                source_provider,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (source_stream_id) DO NOTHING
            "#,
        )
        .bind(start_time)
        .bind(end_time)
        .bind(total_duration_minutes)
        .bind(sleep_quality_score)
        .bind(sleep_stages)
        .bind(source_stream_id)
        .bind(source_table)
        .bind(source_provider)
        .bind(json!({})) // metadata
        .execute(db.pool())
        .await
        .map_err(|e| Error::Database(format!("Failed to insert health_sleep: {}", e)))?;

        count += 1;
    }

    info!("Inserted {} sleep records", count);
    Ok(count)
}

/// Seed iMessage data directly to social_message table
///
/// Loads imessages.csv and directly inserts into social_message ontology table.
async fn seed_imessage_data(db: &Database, _source_id: Uuid, csv_path: &PathBuf) -> Result<usize> {
    info!("Loading iMessage CSV: {}", csv_path.display());

    let file_content = std::fs::read_to_string(csv_path)
        .map_err(|e| Error::Other(format!("Failed to read iMessage CSV: {}", e)))?;

    let mut rdr = csv::Reader::from_reader(file_content.as_bytes());
    let mut count = 0;

    for result in rdr.deserialize() {
        let record: serde_json::Map<String, serde_json::Value> =
            result.map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;

        // Extract fields from CSV (matches existing imessages.csv structure)
        // CSV fields: body, timestamp, channel, direction, from_name, to_names, thread_id, is_group_message
        let message_text = record
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Missing body field".into()))?;

        let timestamp = record
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .ok_or_else(|| Error::Other("Missing or invalid timestamp".into()))?
            .with_timezone(&Utc);

        let platform = record
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or("imessage");

        let direction = record
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("sent");

        let sender_name = record
            .get("from_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        // to_names is like "{\"Me\"}" or "{\"Jimmy James\"}", extract first name
        let receiver_name = record
            .get("to_names")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                let cleaned = s.trim_matches(|c| c == '{' || c == '}' || c == '"');
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned.split(',').next().unwrap_or(cleaned))
                }
            });

        let conversation_id = record.get("thread_id").and_then(|v| v.as_str());

        let is_group_message = record
            .get("is_group_message")
            .and_then(|v| {
                if let Some(b) = v.as_bool() {
                    Some(b)
                } else if let Some(s) = v.as_str() {
                    Some(s.to_lowercase() == "true")
                } else {
                    None
                }
            })
            .unwrap_or(false);

        // Try to get or create person entities for sender/receiver
        let sender_person_id = if let Some(name) = sender_name {
            get_or_create_person(db, name).await.ok()
        } else {
            None
        };

        let receiver_person_id = if let Some(name) = receiver_name {
            get_or_create_person(db, name).await.ok()
        } else {
            None
        };

        // Generate a unique message_id
        let message_id = record
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("msg_seed_{}", Uuid::new_v4()));

        // Generate a unique source_stream_id for each message (to avoid unique constraint violation)
        let source_stream_id = Uuid::new_v4();

        // Insert into social_message table (matching actual schema)
        // Required fields: message_id, channel, timestamp, direction
        sqlx::query(
            r#"
            INSERT INTO elt.social_message (
                message_id,
                thread_id,
                channel,
                body,
                timestamp,
                from_name,
                to_names,
                from_person_id,
                to_person_ids,
                direction,
                is_group_message,
                source_stream_id,
                source_table,
                source_provider,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, ARRAY[$7]::TEXT[], $8, ARRAY[$9]::UUID[], $10, $11, $12, $13, $14, $15)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(message_id)
        .bind(conversation_id)
        .bind(platform)
        .bind(message_text)
        .bind(timestamp)
        .bind(sender_name)
        .bind(receiver_name)
        .bind(sender_person_id)
        .bind(receiver_person_id)
        .bind(direction)
        .bind(is_group_message)
        .bind(source_stream_id)
        .bind("stream_seed_imessage")
        .bind("seed")
        .bind(json!({}))
        .execute(db.pool())
        .await
        .map_err(|e| Error::Database(format!("Failed to insert social_message: {}", e)))?;

        count += 1;
    }

    info!("Inserted {} iMessage records", count);
    Ok(count)
}

/// Helper function to get or create a person entity by name
async fn get_or_create_person(db: &Database, name: &str) -> Result<Uuid> {
    // Check if person already exists
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM elt.entities_person WHERE canonical_name = $1 LIMIT 1",
    )
    .bind(name)
    .fetch_optional(db.pool())
    .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    // Create new person
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO elt.entities_person (canonical_name, metadata)
         VALUES ($1, '{}')
         RETURNING id",
    )
    .bind(name)
    .fetch_one(db.pool())
    .await?;

    Ok(id)
}

/// Seed email data directly to social_email table
///
/// Loads emails.csv and directly inserts into social_email ontology table.
async fn seed_email_data(db: &Database, _source_id: Uuid, csv_path: &PathBuf) -> Result<usize> {
    info!("Loading email CSV: {}", csv_path.display());

    let file_content = std::fs::read_to_string(csv_path)
        .map_err(|e| Error::Other(format!("Failed to read email CSV: {}", e)))?;

    let mut rdr = csv::Reader::from_reader(file_content.as_bytes());
    let mut count = 0;

    for result in rdr.deserialize() {
        let record: serde_json::Map<String, serde_json::Value> =
            result.map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;

        // Extract required fields
        let message_id = record
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Missing message_id".into()))?;

        let timestamp = record
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .ok_or_else(|| Error::Other("Missing or invalid timestamp".into()))?
            .with_timezone(&Utc);

        let direction = record
            .get("direction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Missing direction".into()))?;

        // Extract optional fields
        let thread_id = record.get("thread_id").and_then(|v| v.as_str());
        let subject = record.get("subject").and_then(|v| v.as_str());
        let snippet = record.get("snippet").and_then(|v| v.as_str());
        let body_plain = record.get("body_plain").and_then(|v| v.as_str());
        let body_html = record.get("body_html").and_then(|v| v.as_str());
        let from_address = record.get("from_address").and_then(|v| v.as_str());
        let from_name = record.get("from_name").and_then(|v| v.as_str());

        // Parse array fields (PostgreSQL array format: {value1,value2})
        let parse_pg_array = |field_name: &str| -> Vec<String> {
            record
                .get(field_name)
                .and_then(|v| v.as_str())
                .map(|s| {
                    let cleaned = s.trim_matches(|c| c == '{' || c == '}');
                    if cleaned.is_empty() {
                        vec![]
                    } else {
                        cleaned.split(',').map(|s| s.to_string()).collect()
                    }
                })
                .unwrap_or_default()
        };

        let to_addresses = parse_pg_array("to_addresses");
        let to_names = parse_pg_array("to_names");
        let cc_addresses = parse_pg_array("cc_addresses");
        let cc_names = parse_pg_array("cc_names");
        let bcc_addresses = parse_pg_array("bcc_addresses");
        let labels = parse_pg_array("labels");

        // Parse boolean fields
        let parse_bool = |field_name: &str| -> bool {
            record
                .get(field_name)
                .and_then(|v| {
                    if let Some(b) = v.as_bool() {
                        Some(b)
                    } else if let Some(s) = v.as_str() {
                        Some(s.to_lowercase() == "true")
                    } else {
                        None
                    }
                })
                .unwrap_or(false)
        };

        let is_read = parse_bool("is_read");
        let is_starred = parse_bool("is_starred");
        let has_attachments = parse_bool("has_attachments");

        // Parse integer fields
        let attachment_count = record
            .get("attachment_count")
            .and_then(|v| v.as_i64())
            .map(|i| i as i32)
            .unwrap_or(0);

        let thread_position = record
            .get("thread_position")
            .and_then(|v| v.as_i64())
            .map(|i| i as i32);

        let thread_message_count = record
            .get("thread_message_count")
            .and_then(|v| v.as_i64())
            .map(|i| i as i32);

        // Parse source metadata
        let source_stream_id = record
            .get("source_stream_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| Error::Other("Missing or invalid source_stream_id".into()))?;

        let source_table = record
            .get("source_table")
            .and_then(|v| v.as_str())
            .unwrap_or("stream_seed_gmail");

        let source_provider = record
            .get("source_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("seed");

        // Get or create person entity for sender
        let from_person_id = if let Some(name) = from_name {
            get_or_create_person(db, name).await.ok()
        } else {
            None
        };

        // Get or create person entities for recipients
        let mut to_person_ids: Vec<Uuid> = Vec::new();
        for name in &to_names {
            if let Ok(person_id) = get_or_create_person(db, name).await {
                to_person_ids.push(person_id);
            }
        }

        let mut cc_person_ids: Vec<Uuid> = Vec::new();
        for name in &cc_names {
            if let Ok(person_id) = get_or_create_person(db, name).await {
                cc_person_ids.push(person_id);
            }
        }

        // Insert into social_email table
        sqlx::query(
            r#"
            INSERT INTO elt.social_email (
                message_id,
                thread_id,
                subject,
                snippet,
                body_plain,
                body_html,
                timestamp,
                from_address,
                from_name,
                to_addresses,
                to_names,
                cc_addresses,
                cc_names,
                bcc_addresses,
                from_person_id,
                to_person_ids,
                cc_person_ids,
                direction,
                labels,
                is_read,
                is_starred,
                has_attachments,
                attachment_count,
                thread_position,
                thread_message_count,
                source_stream_id,
                source_table,
                source_provider
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28)
            ON CONFLICT (source_table, message_id) DO NOTHING
            "#,
        )
        .bind(message_id)
        .bind(thread_id)
        .bind(subject)
        .bind(snippet)
        .bind(body_plain)
        .bind(body_html)
        .bind(timestamp)
        .bind(from_address)
        .bind(from_name)
        .bind(&to_addresses)
        .bind(&to_names)
        .bind(&cc_addresses)
        .bind(&cc_names)
        .bind(&bcc_addresses)
        .bind(from_person_id)
        .bind(&to_person_ids)
        .bind(&cc_person_ids)
        .bind(direction)
        .bind(&labels)
        .bind(is_read)
        .bind(is_starred)
        .bind(has_attachments)
        .bind(attachment_count)
        .bind(thread_position)
        .bind(thread_message_count)
        .bind(source_stream_id)
        .bind(source_table)
        .bind(source_provider)
        .execute(db.pool())
        .await
        .map_err(|e| Error::Database(format!("Failed to insert social_email: {}", e)))?;

        count += 1;
    }

    info!("Inserted {} email records", count);
    Ok(count)
}

/// Seed axiology data (values, telos, goals, virtues, vices, habits, temperaments, preferences)
///
/// Loads all axiology CSV files and seeds them into respective tables.
async fn seed_axiology_data(db: &Database, base_path: &PathBuf) -> Result<usize> {
    info!("🎯 Seeding axiology data (values, goals, virtues, habits, etc.)...");
    let mut total_count = 0;

    // Seed VALUES
    let values_path = base_path.join("axiology_values.csv");
    if values_path.exists() {
        info!("Seeding axiology values...");
        let file_content = std::fs::read_to_string(&values_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO elt.axiology_value (title, description, is_active)
                 VALUES ($1, $2, true)
                 ON CONFLICT DO NOTHING",
            )
            .bind(title)
            .bind(description)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded values");
    }

    // Seed TELOS
    let telos_path = base_path.join("axiology_telos.csv");
    if telos_path.exists() {
        info!("Seeding axiology telos...");
        let file_content = std::fs::read_to_string(&telos_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO elt.axiology_telos (title, description, is_active)
                 VALUES ($1, $2, true)
                 ON CONFLICT DO NOTHING",
            )
            .bind(title)
            .bind(description)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded telos");
    }

    // Seed GOALS
    let goals_path = base_path.join("axiology_goals.csv");
    if goals_path.exists() {
        info!("Seeding axiology goals...");
        let file_content = std::fs::read_to_string(&goals_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());
            let goal_type = record.get("goal_type").and_then(|v| v.as_str());
            let status = record
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("active");
            let progress_percent = record
                .get("progress_percent")
                .and_then(|v| v.as_i64())
                .map(|i| i as i32);

            let start_date = record
                .get("start_date")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let target_date = record
                .get("target_date")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            sqlx::query(
                "INSERT INTO elt.axiology_goal (title, description, goal_type, status, progress_percent, start_date, target_date, is_active)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, true)
                 ON CONFLICT DO NOTHING"
            )
            .bind(title)
            .bind(description)
            .bind(goal_type)
            .bind(status)
            .bind(progress_percent)
            .bind(start_date)
            .bind(target_date)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded goals");
    }

    // Seed VIRTUES
    let virtues_path = base_path.join("axiology_virtues.csv");
    if virtues_path.exists() {
        info!("Seeding axiology virtues...");
        let file_content = std::fs::read_to_string(&virtues_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO elt.axiology_virtue (title, description, is_active)
                 VALUES ($1, $2, true)
                 ON CONFLICT DO NOTHING",
            )
            .bind(title)
            .bind(description)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded virtues");
    }

    // Seed VICES
    let vices_path = base_path.join("axiology_vices.csv");
    if vices_path.exists() {
        info!("Seeding axiology vices...");
        let file_content = std::fs::read_to_string(&vices_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO elt.axiology_vice (title, description, is_active)
                 VALUES ($1, $2, true)
                 ON CONFLICT DO NOTHING",
            )
            .bind(title)
            .bind(description)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded vices");
    }

    // Seed HABITS
    let habits_path = base_path.join("axiology_habits.csv");
    if habits_path.exists() {
        info!("Seeding axiology habits...");
        let file_content = std::fs::read_to_string(&habits_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());
            let frequency = record.get("frequency").and_then(|v| v.as_str());
            let time_of_day = record.get("time_of_day").and_then(|v| v.as_str());
            let streak_count = record
                .get("streak_count")
                .and_then(|v| v.as_i64())
                .map(|i| i as i32)
                .unwrap_or(0);

            let last_completed_date = record
                .get("last_completed_date")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

            sqlx::query(
                "INSERT INTO elt.axiology_habit (title, description, frequency, time_of_day, streak_count, last_completed_date, is_active)
                 VALUES ($1, $2, $3, $4, $5, $6, true)
                 ON CONFLICT DO NOTHING"
            )
            .bind(title)
            .bind(description)
            .bind(frequency)
            .bind(time_of_day)
            .bind(streak_count)
            .bind(last_completed_date)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded habits");
    }

    // Seed TEMPERAMENTS
    let temperaments_path = base_path.join("axiology_temperaments.csv");
    if temperaments_path.exists() {
        info!("Seeding axiology temperaments...");
        let file_content = std::fs::read_to_string(&temperaments_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());
            let temperament_type = record.get("temperament_type").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO elt.axiology_temperament (title, description, temperament_type, is_active)
                 VALUES ($1, $2, $3, true)
                 ON CONFLICT DO NOTHING"
            )
            .bind(title)
            .bind(description)
            .bind(temperament_type)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded temperaments");
    }

    // Seed PREFERENCES
    let preferences_path = base_path.join("axiology_preferences.csv");
    if preferences_path.exists() {
        info!("Seeding axiology preferences...");
        let file_content = std::fs::read_to_string(&preferences_path)?;
        let mut rdr = csv::Reader::from_reader(file_content.as_bytes());

        for result in rdr.deserialize() {
            let record: serde_json::Map<String, serde_json::Value> = result
                .map_err(|e| Error::Other(format!("Failed to deserialize CSV record: {}", e)))?;
            let title = record
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other("Missing title".into()))?;
            let description = record.get("description").and_then(|v| v.as_str());
            let preference_domain = record.get("preference_domain").and_then(|v| v.as_str());
            let valence = record.get("valence").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO elt.axiology_preference (title, description, preference_domain, valence, is_active)
                 VALUES ($1, $2, $3, $4, true)
                 ON CONFLICT DO NOTHING"
            )
            .bind(title)
            .bind(description)
            .bind(preference_domain)
            .bind(valence)
            .execute(db.pool())
            .await?;

            total_count += 1;
        }
        info!("✅ Seeded preferences");
    }

    info!(
        "✅ Axiology seeding completed: {} total records",
        total_count
    );
    Ok(total_count)
}

/// Main seeding function - seeds all Monday in Rome streams
pub async fn seed_monday_in_rome(
    db: &Database,
    storage: &Storage,
    stream_writer: Arc<Mutex<StreamWriter>>,
) -> Result<usize> {
    info!("🇮🇹 Seeding Monday in Rome dataset...");

    // Get or create source
    let source_id = get_or_create_test_source(db).await?;

    // Define CSV paths (relative to core/ directory)
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seeds/monday_in_rome");

    // Stream mappings: (csv_filename, stream_name)
    let streams = vec![
        ("pedometer.csv", "pedometer"),
        ("location.csv", "location"),
        ("network.csv", "network"),
        ("accelerometer.csv", "accelerometer"),
        // Note: microphone.csv is handled separately below (direct ontology seeding)
    ];

    let mut total_records = 0;

    for (csv_file, stream_name) in streams {
        let csv_path = base_path.join(csv_file);

        // Check if file exists
        if !csv_path.exists() {
            warn!(
                stream = stream_name,
                path = %csv_path.display(),
                "CSV file not found, skipping"
            );
            continue;
        }

        // Load CSV records
        let records = match load_csv_to_records(&csv_path, stream_name) {
            Ok(records) => records,
            Err(e) => {
                warn!(
                    stream = stream_name,
                    error = %e,
                    "Failed to load CSV, skipping"
                );
                continue;
            }
        };

        total_records += records.len();

        // Seed through full pipeline
        if let Err(e) = seed_stream_pipeline(
            db,
            storage,
            stream_writer.clone(),
            source_id,
            stream_name,
            records,
        )
        .await
        {
            warn!(
                stream = stream_name,
                error = %e,
                "Failed to seed stream, continuing with next"
            );
        }
    }

    // Special handling for microphone transcriptions: seed directly to speech_transcription table
    // (bypasses transform since we don't want to call AssemblyAI API during seeding)
    let microphone_csv_path = base_path.join("microphone.csv");
    if microphone_csv_path.exists() {
        info!("📝 Seeding microphone transcriptions directly to speech_transcription table...");

        match seed_microphone_transcriptions(db, source_id, &microphone_csv_path).await {
            Ok(count) => {
                info!("✅ Seeded {} microphone transcription records", count);
                total_records += count;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to seed microphone transcriptions, continuing"
                );
            }
        }
    } else {
        info!("ℹ️  microphone.csv not found, skipping speech transcription seeding");
    }

    // Special handling for sleep data: seed directly to health_sleep table
    // (CSV already contains final ontology fields)
    let sleep_csv_path = base_path.join("sleep.csv");
    if sleep_csv_path.exists() {
        info!("😴 Seeding sleep data directly to health_sleep table...");

        match seed_sleep_data(db, &sleep_csv_path).await {
            Ok(count) => {
                info!("✅ Seeded {} sleep records", count);
                total_records += count;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to seed sleep data, continuing"
                );
            }
        }
    } else {
        info!("ℹ️  sleep.csv not found, skipping sleep data seeding");
    }

    // Special handling for iMessage data: seed directly to social_message table
    let imessages_csv_path = base_path.join("imessages.csv");
    if imessages_csv_path.exists() {
        info!("💬 Seeding iMessage data directly to social_message table...");

        match seed_imessage_data(db, source_id, &imessages_csv_path).await {
            Ok(count) => {
                info!("✅ Seeded {} iMessage records", count);
                total_records += count;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to seed iMessage data, continuing"
                );
            }
        }
    } else {
        info!("ℹ️  imessages.csv not found, skipping iMessage data seeding");
    }

    // Special handling for email data: seed directly to social_email table
    let emails_csv_path = base_path.join("emails.csv");
    if emails_csv_path.exists() {
        info!("📧 Seeding email data directly to social_email table...");

        match seed_email_data(db, source_id, &emails_csv_path).await {
            Ok(count) => {
                info!("✅ Seeded {} email records", count);
                total_records += count;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to seed email data, continuing"
                );
            }
        }
    } else {
        info!("ℹ️  emails.csv not found, skipping email data seeding");
    }

    // Seed axiology data (values, goals, virtues, habits, preferences, etc.)
    match seed_axiology_data(db, &base_path).await {
        Ok(count) => {
            info!("✅ Seeded {} axiology records", count);
            total_records += count;
        }
        Err(e) => {
            warn!(
                error = %e,
                "Failed to seed axiology data, continuing"
            );
        }
    }

    info!(
        "✅ Monday in Rome seeding completed: {} total records across all streams",
        total_records
    );

    Ok(total_records)
}
