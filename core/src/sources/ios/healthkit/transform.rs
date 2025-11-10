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

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_heart_rate transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "healthkit_to_heart_rate";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched HealthKit batches from S3"
        );

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

                // Insert into health_heart_rate
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.health_heart_rate (
                        bpm, measurement_context, timestamp,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(heart_rate as i32) // Convert to BPM integer
                .bind(&measurement_context)
                .bind(timestamp)
                .bind(stream_id)
                .bind("stream_ios_healthkit")
                .bind("ios")
                .bind(&metadata)
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform heart rate record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
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

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_hrv transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "healthkit_to_hrv";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched HealthKit batches from S3"
        );

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

                // Insert into health_hrv
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.health_hrv (
                        hrv_ms, measurement_type, timestamp,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(hrv)
                .bind(measurement_type)
                .bind(timestamp)
                .bind(stream_id)
                .bind("stream_ios_healthkit")
                .bind("ios")
                .bind(&metadata)
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform HRV record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
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

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_steps transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "healthkit_to_steps";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched HealthKit batches from S3"
        );

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

                // Insert into health_steps
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.health_steps (
                        step_count, timestamp,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(steps as i32)
                .bind(timestamp)
                .bind(stream_id)
                .bind("stream_ios_healthkit")
                .bind("ios")
                .bind(&metadata)
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform steps record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
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

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_sleep transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "healthkit_to_sleep";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched HealthKit batches from S3"
        );

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

                // Insert into health_sleep
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.health_sleep (
                        sleep_stages, total_duration_minutes, sleep_quality_score,
                        start_time, end_time,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(&sleep_stages)
                .bind(sleep_duration as i32)
                .bind(None::<f64>) // sleep_quality_score not available in basic data
                .bind(timestamp)
                .bind(end_time)
                .bind(stream_id)
                .bind("stream_ios_healthkit")
                .bind("ios")
                .bind(&metadata)
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform sleep record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
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

        tracing::info!(
            source_id = %source_id,
            "Starting HealthKit to health_workout transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "healthkit_to_workout";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "healthkit", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched HealthKit batches from S3"
        );

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

                // Insert into health_workout
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.health_workout (
                        activity_type, intensity,
                        calories_burned, average_heart_rate, max_heart_rate, distance_meters,
                        place_id,
                        start_time, end_time,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(workout_type)
                .bind(&intensity)
                .bind(active_energy.map(|e| e as i32))
                .bind(heart_rate.map(|h| h as i32))
                .bind(max_heart_rate)
                .bind(distance)
                .bind(None::<Uuid>) // place_id not available yet
                .bind(timestamp)
                .bind(end_time)
                .bind(stream_id)
                .bind("stream_ios_healthkit")
                .bind("ios")
                .bind(&metadata)
                .execute(db.pool())
                .await;

                match result {
                    Ok(_) => {
                        records_written += 1;
                        last_processed_id = Some(stream_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to transform workout record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "healthkit",
                    checkpoint_key,
                    max_ts
                ).await?;
            }
        }

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
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
