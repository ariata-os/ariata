//! HealthKit to health ontology transformations
//!
//! Transforms raw HealthKit data from stream_ios_healthkit into normalized
//! health ontology tables (heart_rate, hrv, steps, sleep, workout).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Batch size for bulk inserts
const BATCH_SIZE: usize = 500;

/// Transform HealthKit heart rate data to health_heart_rate ontology
pub struct HealthKitHeartRateTransform;

#[async_trait]
impl OntologyTransform for HealthKitHeartRateTransform {
    fn source_table(&self) -> &str {
        "stream_ios_healthkit"
    }

    fn target_table(&self) -> &str {
        "health_heart_rate"
    }

    fn domain(&self) -> &str {
        "health"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_heart_rate transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "healthkit_to_heart_rate";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched HealthKit batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(i32, Option<String>, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(heart_rate) = record.get("heart_rate").and_then(|v| v.as_f64()) else {
                    continue; // Skip records without heart_rate
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let raw_data = record.get("raw_data").cloned();

                // Determine measurement context from raw_data or time of day
                let measurement_context = raw_data
                    .as_ref()
                    .and_then(|d| d.get("context"))
                    .and_then(|c| c.as_str())
                    .map(String::from);

                // Build metadata
                let metadata = serde_json::json!({
                    "healthkit_raw": raw_data,
                });

                // Add to pending batch
                pending_records.push((
                    heart_rate as i32,
                    measurement_context,
                    timestamp,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_heart_rate_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_heart_rate_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "HealthKit to health_heart_rate transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Transform HealthKit HRV data to health_hrv ontology
pub struct HealthKitHRVTransform;

#[async_trait]
impl OntologyTransform for HealthKitHRVTransform {
    fn source_table(&self) -> &str {
        "stream_ios_healthkit"
    }

    fn target_table(&self) -> &str {
        "health_hrv"
    }

    fn domain(&self) -> &str {
        "health"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_hrv transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "healthkit_to_hrv";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched HealthKit batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(f64, String, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(hrv) = record.get("hrv").and_then(|v| v.as_f64()) else {
                    continue; // Skip records without hrv
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let raw_data = record.get("raw_data").cloned();

                // Determine HRV measurement type (default to RMSSD for Apple Watch)
                let measurement_type = raw_data
                    .as_ref()
                    .and_then(|d| d.get("hrv_type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("rmssd");

                let metadata = serde_json::json!({
                    "healthkit_raw": raw_data,
                });

                // Add to pending batch
                pending_records.push((
                    hrv,
                    measurement_type.to_string(),
                    timestamp,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_hrv_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_hrv_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "HealthKit to health_hrv transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Transform HealthKit steps data to health_steps ontology
pub struct HealthKitStepsTransform;

#[async_trait]
impl OntologyTransform for HealthKitStepsTransform {
    fn source_table(&self) -> &str {
        "stream_ios_healthkit"
    }

    fn target_table(&self) -> &str {
        "health_steps"
    }

    fn domain(&self) -> &str {
        "health"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_steps transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "healthkit_to_steps";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched HealthKit batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(i32, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(steps) = record.get("steps").and_then(|v| v.as_i64()) else {
                    continue; // Skip records without steps
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let raw_data = record.get("raw_data").cloned();

                let metadata = serde_json::json!({
                    "healthkit_raw": raw_data,
                });

                // Add to pending batch
                pending_records.push((
                    steps as i32,
                    timestamp,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_steps_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_steps_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "HealthKit to health_steps transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Transform HealthKit sleep data to health_sleep ontology
pub struct HealthKitSleepTransform;

#[async_trait]
impl OntologyTransform for HealthKitSleepTransform {
    fn source_table(&self) -> &str {
        "stream_ios_healthkit"
    }

    fn target_table(&self) -> &str {
        "health_sleep"
    }

    fn domain(&self) -> &str {
        "health"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_sleep transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "healthkit_to_sleep";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched HealthKit batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(Option<serde_json::Value>, i32, Option<f64>, DateTime<Utc>, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(sleep_duration) = record.get("sleep_duration").and_then(|v| v.as_i64()) else {
                    continue; // Skip records without sleep_duration
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let sleep_stage = record.get("sleep_stage")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let raw_data = record.get("raw_data").cloned();

                // Build sleep_stages JSON from raw_data if available
                let sleep_stages = raw_data
                    .as_ref()
                    .and_then(|d| d.get("stages"))
                    .cloned()
                    .or_else(|| {
                        sleep_stage.as_ref().map(|stage| {
                            serde_json::json!([{
                                "stage": stage,
                                "duration_minutes": sleep_duration
                            }])
                        })
                    });

                // Calculate end_time from timestamp + duration
                let end_time = timestamp + chrono::Duration::minutes(sleep_duration);

                let metadata = serde_json::json!({
                    "healthkit_raw": raw_data,
                });

                // Add to pending batch
                pending_records.push((
                    sleep_stages,
                    sleep_duration as i32,
                    None, // sleep_quality_score not available in basic data
                    timestamp,
                    end_time,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_sleep_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_sleep_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "HealthKit to health_sleep transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Transform HealthKit workout data to health_workout ontology
pub struct HealthKitWorkoutTransform;

#[async_trait]
impl OntologyTransform for HealthKitWorkoutTransform {
    fn source_table(&self) -> &str {
        "stream_ios_healthkit"
    }

    fn target_table(&self) -> &str {
        "health_workout"
    }

    fn domain(&self) -> &str {
        "health"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_workout transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "healthkit_to_workout";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| crate::Error::Other("No data source available for transform".to_string()))?;
        let batches = data_source
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched HealthKit batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(String, Option<String>, Option<i32>, Option<i32>, Option<i32>, Option<f64>, Option<Uuid>, DateTime<Utc>, DateTime<Utc>, Uuid, serde_json::Value)> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let Some(workout_type) = record.get("workout_type").and_then(|v| v.as_str()) else {
                    continue; // Skip records without workout_type
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let workout_duration = record.get("workout_duration")
                    .and_then(|v| v.as_i64())
                    .map(|d| d as i32);

                let active_energy = record.get("active_energy").and_then(|v| v.as_f64());
                let distance = record.get("distance").and_then(|v| v.as_f64());
                let heart_rate = record.get("heart_rate").and_then(|v| v.as_f64());
                let raw_data = record.get("raw_data").cloned();

                // Calculate end_time from timestamp + duration
                let duration_minutes = workout_duration.unwrap_or(0);
                let end_time = timestamp + chrono::Duration::minutes(duration_minutes as i64);

                // Extract additional workout details from raw_data
                let max_heart_rate = raw_data
                    .as_ref()
                    .and_then(|d| d.get("max_heart_rate"))
                    .and_then(|h| h.as_f64())
                    .map(|h| h as i32);

                let intensity = raw_data
                    .as_ref()
                    .and_then(|d| d.get("intensity"))
                    .and_then(|i| i.as_str())
                    .map(String::from);

                let metadata = serde_json::json!({
                    "healthkit_raw": raw_data,
                });

                // Add to pending batch
                pending_records.push((
                    workout_type.to_string(),
                    intensity,
                    active_energy.map(|e| e as i32),
                    heart_rate.map(|h| h as i32),
                    max_heart_rate,
                    distance,
                    None, // place_id not available yet
                    timestamp,
                    end_time,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_workout_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_workout_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "HealthKit to health_workout transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Execute batch insert for heart rate records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_heart_rate_batch_insert(
    db: &Database,
    records: &[(i32, Option<String>, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.health_heart_rate",
        &["bpm", "measurement_context", "timestamp", "source_stream_id", "source_table", "source_provider", "metadata"],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (bpm, measurement_context, timestamp, stream_id, metadata) in records {
        query = query
            .bind(bpm)
            .bind(measurement_context)
            .bind(timestamp)
            .bind(stream_id)
            .bind("stream_ios_healthkit")
            .bind("ios")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

/// Execute batch insert for HRV records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_hrv_batch_insert(
    db: &Database,
    records: &[(f64, String, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.health_hrv",
        &["hrv_ms", "measurement_type", "timestamp", "source_stream_id", "source_table", "source_provider", "metadata"],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (hrv_ms, measurement_type, timestamp, stream_id, metadata) in records {
        query = query
            .bind(hrv_ms)
            .bind(measurement_type)
            .bind(timestamp)
            .bind(stream_id)
            .bind("stream_ios_healthkit")
            .bind("ios")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

/// Execute batch insert for steps records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_steps_batch_insert(
    db: &Database,
    records: &[(i32, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.health_steps",
        &["step_count", "timestamp", "source_stream_id", "source_table", "source_provider", "metadata"],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (step_count, timestamp, stream_id, metadata) in records {
        query = query
            .bind(step_count)
            .bind(timestamp)
            .bind(stream_id)
            .bind("stream_ios_healthkit")
            .bind("ios")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

/// Execute batch insert for sleep records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_sleep_batch_insert(
    db: &Database,
    records: &[(Option<serde_json::Value>, i32, Option<f64>, DateTime<Utc>, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.health_sleep",
        &["sleep_stages", "total_duration_minutes", "sleep_quality_score", "start_time", "end_time", "source_stream_id", "source_table", "source_provider", "metadata"],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (sleep_stages, total_duration_minutes, sleep_quality_score, start_time, end_time, stream_id, metadata) in records {
        query = query
            .bind(sleep_stages)
            .bind(total_duration_minutes)
            .bind(sleep_quality_score)
            .bind(start_time)
            .bind(end_time)
            .bind(stream_id)
            .bind("stream_ios_healthkit")
            .bind("ios")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

/// Execute batch insert for workout records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_workout_batch_insert(
    db: &Database,
    records: &[(String, Option<String>, Option<i32>, Option<i32>, Option<i32>, Option<f64>, Option<Uuid>, DateTime<Utc>, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "elt.health_workout",
        &["activity_type", "intensity", "calories_burned", "average_heart_rate", "max_heart_rate", "distance_meters", "place_id", "start_time", "end_time", "source_stream_id", "source_table", "source_provider", "metadata"],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (activity_type, intensity, calories_burned, average_heart_rate, max_heart_rate, distance_meters, place_id, start_time, end_time, stream_id, metadata) in records {
        query = query
            .bind(activity_type)
            .bind(intensity)
            .bind(calories_burned)
            .bind(average_heart_rate)
            .bind(max_heart_rate)
            .bind(distance_meters)
            .bind(place_id)
            .bind(start_time)
            .bind(end_time)
            .bind(stream_id)
            .bind("stream_ios_healthkit")
            .bind("ios")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heart_rate_transform_metadata() {
        let transform = HealthKitHeartRateTransform;
        assert_eq!(transform.source_table(), "stream_ios_healthkit");
        assert_eq!(transform.target_table(), "health_heart_rate");
        assert_eq!(transform.domain(), "health");
    }

    #[test]
    fn test_hrv_transform_metadata() {
        let transform = HealthKitHRVTransform;
        assert_eq!(transform.source_table(), "stream_ios_healthkit");
        assert_eq!(transform.target_table(), "health_hrv");
        assert_eq!(transform.domain(), "health");
    }

    #[test]
    fn test_steps_transform_metadata() {
        let transform = HealthKitStepsTransform;
        assert_eq!(transform.source_table(), "stream_ios_healthkit");
        assert_eq!(transform.target_table(), "health_steps");
        assert_eq!(transform.domain(), "health");
    }

    #[test]
    fn test_sleep_transform_metadata() {
        let transform = HealthKitSleepTransform;
        assert_eq!(transform.source_table(), "stream_ios_healthkit");
        assert_eq!(transform.target_table(), "health_sleep");
        assert_eq!(transform.domain(), "health");
    }

    #[test]
    fn test_workout_transform_metadata() {
        let transform = HealthKitWorkoutTransform;
        assert_eq!(transform.source_table(), "stream_ios_healthkit");
        assert_eq!(transform.target_table(), "health_workout");
        assert_eq!(transform.domain(), "health");
    }
}
