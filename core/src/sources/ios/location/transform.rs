//! iOS Location to location_point ontology transformation
//!
//! Transforms raw iOS location data from stream_ios_location into the normalized
//! location_point ontology table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
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

    #[tracing::instrument(skip(self, db), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        tracing::info!(
            source_id = %source_id,
            "Starting iOS Location to location_point transformation"
        );

        // Query stream_ios_location for records not yet transformed
        // Use left join to find records that don't exist in location_point
        let rows = sqlx::query(
            r#"
            SELECT
                l.id, l.timestamp,
                l.latitude, l.longitude, l.altitude,
                l.speed, l.course,
                l.horizontal_accuracy, l.vertical_accuracy,
                l.activity_type, l.activity_confidence,
                l.floor_level,
                l.raw_data
            FROM elt.stream_ios_location l
            LEFT JOIN elt.location_point p ON (p.source_stream_id = l.id)
            WHERE l.source_id = $1
              AND p.id IS NULL
            ORDER BY l.timestamp ASC
            LIMIT 1000
            "#,
        )
        .bind(source_id)
        .fetch_all(db.pool())
        .await?;

        tracing::debug!(
            records_to_transform = rows.len(),
            "Fetched untransformed iOS location records"
        );

        for row in rows {
            records_read += 1;

            // Extract fields from row
            let stream_id: Uuid = row.try_get("id")?;
            let timestamp: DateTime<Utc> = row.try_get("timestamp")?;
            let latitude: f64 = row.try_get("latitude")?;
            let longitude: f64 = row.try_get("longitude")?;
            let altitude: Option<f64> = row.try_get("altitude")?;
            let speed: Option<f64> = row.try_get("speed")?;
            let course: Option<f64> = row.try_get("course")?;
            let horizontal_accuracy: Option<f64> = row.try_get("horizontal_accuracy")?;
            let _vertical_accuracy: Option<f64> = row.try_get("vertical_accuracy")?;
            let activity_type: Option<String> = row.try_get("activity_type")?;
            let activity_confidence: Option<String> = row.try_get("activity_confidence")?;
            let floor_level: Option<i32> = row.try_get("floor_level")?;
            let raw_data: Option<serde_json::Value> = row.try_get("raw_data")?;

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
