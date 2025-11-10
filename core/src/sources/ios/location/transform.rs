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
    async fn transform(&self, db: &Database, context: &crate::jobs::transform_context::TransformContext, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting iOS Location to location_point transformation"
        );

        // Read stream data from S3 using checkpoint
        let checkpoint_key = "location_to_location_point";
        let batches = context.stream_reader
            .read_with_checkpoint(source_id, "location", checkpoint_key)
            .await?;

        tracing::debug!(
            batch_count = batches.len(),
            "Fetched iOS location batches from S3"
        );

        for batch in batches {
            for record in &batch.records {
                records_read += 1;

                // Extract required fields from JSONL record
                let Some(latitude) = record.get("latitude").and_then(|v| v.as_f64()) else {
                    continue; // Skip records without latitude
                };
                let Some(longitude) = record.get("longitude").and_then(|v| v.as_f64()) else {
                    continue; // Skip records without longitude
                };

                let timestamp = record.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let altitude = record.get("altitude").and_then(|v| v.as_f64());
                let speed = record.get("speed").and_then(|v| v.as_f64());
                let course = record.get("course").and_then(|v| v.as_f64());
                let horizontal_accuracy = record.get("horizontal_accuracy").and_then(|v| v.as_f64());
                let activity_type = record.get("activity_type").and_then(|v| v.as_str()).map(String::from);
                let activity_confidence = record.get("activity_confidence").and_then(|v| v.as_str()).map(String::from);
                let floor_level = record.get("floor_level").and_then(|v| v.as_i64()).map(|v| v as i32);
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

                // Insert into location_point
                let result = sqlx::query(
                    r#"
                    INSERT INTO elt.location_point (
                        coordinates, latitude, longitude, altitude_meters,
                        accuracy_meters, speed_meters_per_second, course_degrees,
                        timestamp,
                        source_stream_id, source_table, source_provider,
                        metadata
                    ) VALUES (
                        ST_GeogFromText($1), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
                    )
                    ON CONFLICT (source_stream_id) DO NOTHING
                    "#,
                )
                .bind(&point_wkt)
                .bind(latitude)
                .bind(longitude)
                .bind(altitude)
                .bind(horizontal_accuracy)
                .bind(speed)
                .bind(course)
                .bind(timestamp)
                .bind(stream_id)
                .bind("stream_ios_location")
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
                            latitude = %latitude,
                            longitude = %longitude,
                            error = %e,
                            "Failed to transform location record"
                        );
                        records_failed += 1;
                    }
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                context.stream_reader.update_checkpoint(
                    source_id,
                    "location",
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
