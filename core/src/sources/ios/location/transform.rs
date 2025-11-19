//! iOS Location to location_point ontology transformation
//!
//! Transforms raw iOS location data from stream_ios_location into the normalized
//! location_point ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Batch size for database inserts
const BATCH_SIZE: usize = 500;

/// Transform iOS location data to location_point ontology
pub struct IosLocationTransform;

#[async_trait]
impl OntologyTransform for IosLocationTransform {
    fn source_table(&self) -> &str {
        "stream_ios_location"
    }

    fn target_table(&self) -> &str {
        "location_point"
    }

    fn domain(&self) -> &str {
        "location"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(
        &self,
        db: &Database,
        context: &crate::jobs::transform_context::TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting iOS Location to location_point transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "location_to_location_point";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| {
            crate::Error::Other("No data source available for transform".to_string())
        })?;
        let batches = data_source
            .read_with_checkpoint(source_id, "location", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched iOS location batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            DateTime<Utc>,
            Uuid,
            serde_json::Value,
        )> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;

        let processing_start = std::time::Instant::now();

        for batch in batches {
            tracing::debug!(batch_record_count = batch.records.len(), "Processing batch");

            for record in &batch.records {
                records_read += 1;

                // Extract required fields from JSONL record
                let Some(latitude) = record.get("latitude").and_then(|v| v.as_f64()) else {
                    continue; // Skip records without latitude
                };
                let Some(longitude) = record.get("longitude").and_then(|v| v.as_f64()) else {
                    continue; // Skip records without longitude
                };

                let timestamp = record
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let altitude = record.get("altitude").and_then(|v| v.as_f64());
                let speed = record.get("speed").and_then(|v| v.as_f64());
                let course = record.get("course").and_then(|v| v.as_f64());
                let horizontal_accuracy =
                    record.get("horizontal_accuracy").and_then(|v| v.as_f64());
                let activity_type = record
                    .get("activity_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let activity_confidence = record
                    .get("activity_confidence")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let floor_level = record
                    .get("floor_level")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                let raw_data = record.get("raw_data").cloned();

                // Build metadata with iOS-specific fields
                let metadata = serde_json::json!({
                    "activity_type": activity_type,
                    "activity_confidence": activity_confidence,
                    "floor_level": floor_level,
                    "ios_raw": raw_data,
                });

                // Create PostGIS POINT geography from lat/lon
                // Format: POINT(longitude latitude)
                let point_wkt = format!("POINT({} {})", longitude, latitude);

                // Add to pending batch
                pending_records.push((
                    point_wkt,
                    latitude,
                    longitude,
                    altitude,
                    horizontal_accuracy,
                    speed,
                    course,
                    timestamp,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_location_batch_insert(db, &pending_records).await;
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
                data_source
                    .update_checkpoint(source_id, "location", checkpoint_key, max_ts)
                    .await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_location_batch_insert(db, &pending_records).await;
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
            "iOS Location to location_point transformation completed"
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

/// Execute batch insert for location records
///
/// Builds and executes a multi-row INSERT statement for efficient bulk insertion.
async fn execute_location_batch_insert(
    db: &Database,
    records: &[(
        String,
        f64,
        f64,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        DateTime<Utc>,
        Uuid,
        serde_json::Value,
    )],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    // Build custom batch insert query with PostGIS function
    // We can't use the generic builder because coordinates needs ST_GeogFromText()
    let num_cols = 12;
    let mut query_str = String::from("INSERT INTO data.location_point (");
    query_str.push_str("coordinates, latitude, longitude, altitude_meters, ");
    query_str.push_str("accuracy_meters, speed_meters_per_second, course_degrees, ");
    query_str.push_str("timestamp, source_stream_id, source_table, source_provider, metadata");
    query_str.push_str(") VALUES ");

    // Build VALUES clauses with ST_GeogFromText for coordinates
    let mut value_clauses = Vec::with_capacity(records.len());
    for row_idx in 0..records.len() {
        let base_param = row_idx * num_cols + 1;
        let clause = format!(
            "(ST_GeogFromText(${base_param}), ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
            base_param + 1,
            base_param + 2,
            base_param + 3,
            base_param + 4,
            base_param + 5,
            base_param + 6,
            base_param + 7,
            base_param + 8,
            base_param + 9,
            base_param + 10,
            base_param + 11,
        );
        value_clauses.push(clause);
    }

    query_str.push_str(&value_clauses.join(", "));
    query_str.push_str(" ON CONFLICT (source_stream_id) DO NOTHING");

    // Build query with proper parameter binding
    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (
        point_wkt,
        latitude,
        longitude,
        altitude,
        accuracy,
        speed,
        course,
        timestamp,
        stream_id,
        metadata,
    ) in records
    {
        query = query
            .bind(format!("SRID=4326;{}", point_wkt))
            .bind(latitude)
            .bind(longitude)
            .bind(altitude)
            .bind(accuracy)
            .bind(speed)
            .bind(course)
            .bind(timestamp)
            .bind(stream_id)
            .bind("stream_ios_location")
            .bind("ios")
            .bind(metadata);
    }

    // Execute batch insert
    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_metadata() {
        let transform = IosLocationTransform;
        assert_eq!(transform.source_table(), "stream_ios_location");
        assert_eq!(transform.target_table(), "location_point");
        assert_eq!(transform.domain(), "location");
    }
}
