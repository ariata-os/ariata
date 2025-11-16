//! iOS Location Visit Transformation
//!
//! Transforms raw location_point primitives into semantic location_visit records
//! using density-adaptive HDBSCAN clustering.
//!
//! ## Configuration
//!
//! The lookback window can be configured via `TransformContext.metadata`:
//!
//! ```json
//! {
//!   "lookback_hours": 168  // Process last 7 days instead of default 12 hours
//! }
//! ```
//!
//! **Use cases:**
//! - `12` hours (default) - Normal hourly cron operation
//! - `168` hours (7 days) - Testing with seed data
//! - `720` hours (30 days) - Initial deployment backfill

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use geo::HaversineDistance;
use geo::Point as GeoPoint;
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Spatial clustering parameters
const SPATIAL_EPSILON_METERS: f64 = 50.0;  // Max distance within a cluster
const MIN_VISIT_DURATION_MINUTES: i64 = 5; // Minimum visit duration
const TEMPORAL_GAP_MINUTES: i64 = 3;        // Max time gap within visit
const MAX_HORIZONTAL_ACCURACY: f64 = 100.0; // Filter low-quality points
const MAX_SPEED_MPS: f64 = 50.0;             // Filter high-speed transit
const LOOKBACK_HOURS: i64 = 12;              // Default rolling window size (can be overridden)

/// Location point for clustering
#[derive(Debug, Clone)]
struct LocationPoint {
    id: Uuid,
    latitude: f64,
    longitude: f64,
    timestamp: DateTime<Utc>,
    horizontal_accuracy: Option<f64>,
    _speed: Option<f64>,
}

/// Clustered visit
#[derive(Debug, Clone)]
struct Visit {
    points: Vec<LocationPoint>,
    centroid_lat: f64,
    centroid_lon: f64,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

/// Transform location_point to location_visit using spatial clustering
///
/// # Architecture Note
///
/// This is a primitive-to-primitive transform (location_point → location_visit).
/// It processes ALL location_point records from the last 12 hours.
///
/// Source filtering is NOT performed because:
/// - Ontology primitives are source-agnostic normalized facts
/// - Stream tables (stream_ios_location) exist in S3, not Postgres
/// - Single-tenant MVP assumption (one source per deployment)
/// - Clustering semantics require all available points
///
/// The source_id parameter is used for logging/monitoring only.
pub struct LocationVisitTransform;

#[async_trait]
impl OntologyTransform for LocationVisitTransform {
    fn source_table(&self) -> &str {
        "location_point"
    }

    fn target_table(&self) -> &str {
        "location_visit"
    }

    fn domain(&self) -> &str {
        "location"
    }

    #[tracing::instrument(skip(self, db, context), fields(
        source_table = %self.source_table(),
        target_table = %self.target_table()
    ))]
    async fn transform(
        &self,
        db: &Database,
        context: &crate::jobs::transform_context::TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let transform_start = std::time::Instant::now();

        // Read lookback from transform metadata (if provided)
        let lookback_hours = context.metadata
            .get("lookback_hours")
            .and_then(|v| v.as_i64())
            .unwrap_or(LOOKBACK_HOURS);

        tracing::info!(
            source_id = %source_id,
            lookback_hours,
            "Starting location visit clustering"
        );

        // 1. Fetch location points from last N hours (all sources)
        let points = fetch_location_points(db, lookback_hours).await?;

        if points.is_empty() {
            tracing::info!("No location points to cluster");
            return Ok(TransformResult {
                records_read: 0,
                records_written: 0,
                records_failed: 0,
                last_processed_id: None,
                chained_transforms: vec![],
            });
        }

        tracing::info!(
            point_count = points.len(),
            "Fetched location points"
        );

        // 2. Auto-detect sampling rate
        let sampling_rate = detect_sampling_rate(&points);
        tracing::info!(
            points_per_minute = sampling_rate,
            "Detected sampling rate"
        );

        // 3. Run density-adaptive clustering
        let cluster_start = std::time::Instant::now();
        let visits = cluster_location_points(&points, sampling_rate)?;
        let cluster_duration = cluster_start.elapsed();

        tracing::info!(
            visit_count = visits.len(),
            cluster_duration_ms = cluster_duration.as_millis(),
            "Completed clustering"
        );

        // 4. Write visits idempotently
        let mut records_written = 0;
        for visit in &visits {
            match write_visit_idempotent(db, source_id, visit).await {
                Ok(_) => records_written += 1,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to write visit"
                    );
                }
            }
        }

        let total_duration = transform_start.elapsed();
        tracing::info!(
            source_id = %source_id,
            records_read = points.len(),
            records_written,
            total_duration_ms = total_duration.as_millis(),
            "Location visit transformation completed"
        );

        Ok(TransformResult {
            records_read: points.len(),
            records_written,
            records_failed: 0,
            last_processed_id: None,
            chained_transforms: vec![],
        })
    }
}

/// Fetch location points from database (configurable lookback, all sources)
///
/// # Arguments
/// * `lookback_hours` - How far back to look for points (e.g., 12, 168, 720)
///
/// Note: Queries all location_point records regardless of source.
/// This is correct for single-tenant MVP and ontology design.
async fn fetch_location_points(db: &Database, lookback_hours: i64) -> Result<Vec<LocationPoint>> {
    let cutoff = Utc::now() - Duration::hours(lookback_hours);

    tracing::debug!(
        lookback_hours,
        cutoff = %cutoff,
        "Fetching location points for clustering"
    );

    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            latitude,
            longitude,
            timestamp,
            accuracy_meters as horizontal_accuracy,
            speed_meters_per_second as speed
        FROM elt.location_point
        WHERE timestamp >= $1
          AND (accuracy_meters IS NULL OR accuracy_meters < $2)
          AND (speed_meters_per_second IS NULL
               OR speed_meters_per_second < 0
               OR speed_meters_per_second < $3)
        ORDER BY timestamp ASC
        "#,
        cutoff,
        MAX_HORIZONTAL_ACCURACY,
        MAX_SPEED_MPS
    )
    .fetch_all(db.pool())
    .await?;

    let points = rows
        .into_iter()
        .map(|row| LocationPoint {
            id: row.id,
            latitude: row.latitude,
            longitude: row.longitude,
            timestamp: row.timestamp,
            horizontal_accuracy: row.horizontal_accuracy,
            _speed: row.speed,
        })
        .collect();

    Ok(points)
}

/// Auto-detect sampling rate from point density
fn detect_sampling_rate(points: &[LocationPoint]) -> f64 {
    if points.len() < 10 {
        return 1.0; // Default to 1 point/min
    }

    // Calculate median time gap between consecutive points
    let mut gaps: Vec<f64> = points
        .windows(2)
        .filter_map(|w| {
            let gap_seconds = (w[1].timestamp - w[0].timestamp).num_seconds();
            if gap_seconds > 0 && gap_seconds < 600 {
                // Sanity check: 0-10 minutes
                Some(60.0 / gap_seconds as f64)
            } else {
                None
            }
        })
        .collect();

    if gaps.is_empty() {
        return 1.0;
    }

    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    gaps[gaps.len() / 2] // Median points per minute
}

/// Cluster location points using density-adaptive HDBSCAN
fn cluster_location_points(points: &[LocationPoint], points_per_minute: f64) -> Result<Vec<Visit>> {
    // Calculate density-adaptive parameters
    let min_cluster_size = (MIN_VISIT_DURATION_MINUTES as f64 * points_per_minute).round() as usize;
    let min_cluster_size = min_cluster_size.max(3); // At least 3 points

    let temporal_gap_points = (TEMPORAL_GAP_MINUTES as f64 * points_per_minute).round() as usize;

    tracing::debug!(
        min_cluster_size,
        temporal_gap_points,
        "Calculated adaptive parameters"
    );

    // Simple density-based clustering (DBSCAN-like)
    let mut visits = Vec::new();
    let mut visited = vec![false; points.len()];

    for i in 0..points.len() {
        if visited[i] {
            continue;
        }

        // Start a new potential cluster
        let mut cluster_points = vec![points[i].clone()];
        visited[i] = true;

        // Expand cluster
        let mut j = i + 1;
        while j < points.len() {
            if visited[j] {
                j += 1;
                continue;
            }

            let last_point = cluster_points.last().unwrap();
            let current_point = &points[j];

            // Check spatial distance
            let distance = haversine_distance(
                last_point.latitude,
                last_point.longitude,
                current_point.latitude,
                current_point.longitude,
            );

            // Check temporal gap
            let time_gap = (current_point.timestamp - last_point.timestamp).num_minutes();

            if distance <= SPATIAL_EPSILON_METERS && time_gap <= TEMPORAL_GAP_MINUTES {
                // Point belongs to cluster
                cluster_points.push(current_point.clone());
                visited[j] = true;
                j += 1;
            } else if time_gap > TEMPORAL_GAP_MINUTES {
                // Temporal gap too large - end cluster
                break;
            } else {
                // Spatial distance too large but temporal OK - skip point
                j += 1;
            }
        }

        // Check if cluster meets minimum size
        if cluster_points.len() >= min_cluster_size {
            let visit = create_visit_from_cluster(cluster_points)?;
            let duration_minutes = (visit.end_time - visit.start_time).num_minutes();

            // Filter by minimum duration
            if duration_minutes >= MIN_VISIT_DURATION_MINUTES {
                visits.push(visit);
            }
        }
    }

    Ok(visits)
}

/// Create a visit from a cluster of points
fn create_visit_from_cluster(points: Vec<LocationPoint>) -> Result<Visit> {
    // Calculate weighted centroid (weight by accuracy)
    let mut total_weight = 0.0;
    let mut weighted_lat = 0.0;
    let mut weighted_lon = 0.0;

    for point in &points {
        let weight = 1.0 / point.horizontal_accuracy.unwrap_or(50.0);
        weighted_lat += point.latitude * weight;
        weighted_lon += point.longitude * weight;
        total_weight += weight;
    }

    let centroid_lat = weighted_lat / total_weight;
    let centroid_lon = weighted_lon / total_weight;

    let start_time = points.first().unwrap().timestamp;
    let end_time = points.last().unwrap().timestamp;

    Ok(Visit {
        points,
        centroid_lat,
        centroid_lon,
        start_time,
        end_time,
    })
}

/// Calculate Haversine distance between two points (in meters)
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let p1 = GeoPoint::new(lon1, lat1);
    let p2 = GeoPoint::new(lon2, lat2);
    p1.haversine_distance(&p2)
}

/// Generate deterministic visit ID
fn generate_visit_id(source_id: Uuid, centroid_lat: f64, centroid_lon: f64, start_time: DateTime<Utc>) -> Uuid {
    // Round coordinates to ~10 meter precision (4 decimal places)
    let lat_rounded = (centroid_lat * 10000.0).round() / 10000.0;
    let lon_rounded = (centroid_lon * 10000.0).round() / 10000.0;

    // Round start time to nearest minute by converting to timestamp and back
    let timestamp_secs = start_time.timestamp();
    let rounded_secs = (timestamp_secs / 60) * 60; // Round down to nearest minute
    let start_rounded = DateTime::from_timestamp(rounded_secs, 0).unwrap();

    // Create deterministic UUID v5
    let hash_input = format!(
        "{}:{}:{}:{}",
        source_id,
        lat_rounded,
        lon_rounded,
        start_rounded.to_rfc3339()
    );

    Uuid::new_v5(&Uuid::NAMESPACE_OID, hash_input.as_bytes())
}

/// Write visit idempotently to database
async fn write_visit_idempotent(db: &Database, source_id: Uuid, visit: &Visit) -> Result<()> {
    let visit_id = generate_visit_id(source_id, visit.centroid_lat, visit.centroid_lon, visit.start_time);

    let point_wkt = format!("POINT({} {})", visit.centroid_lon, visit.centroid_lat);

    let metadata = serde_json::json!({
        "point_count": visit.points.len(),
        "radius_meters": calculate_visit_radius(visit),
    });

    sqlx::query!(
        r#"
        INSERT INTO elt.location_visit (
            id,
            place_id,
            centroid_coordinates,
            latitude,
            longitude,
            start_time,
            end_time,
            source_stream_id,
            source_table,
            source_provider,
            metadata
        ) VALUES (
            $1, NULL, ST_GeogFromText($2), $3, $4, $5, $6, $7, $8, $9, $10
        )
        ON CONFLICT (id) DO UPDATE SET
            end_time = EXCLUDED.end_time,
            centroid_coordinates = EXCLUDED.centroid_coordinates,
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            metadata = EXCLUDED.metadata,
            updated_at = NOW()
        WHERE elt.location_visit.end_time < EXCLUDED.end_time
        "#,
        visit_id,
        format!("SRID=4326;{}", point_wkt),
        visit.centroid_lat,
        visit.centroid_lon,
        visit.start_time,
        visit.end_time,
        visit.points.first().unwrap().id, // Use first point ID as source
        "location_point",
        "ios",
        metadata
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

/// Calculate radius of visit (max distance from centroid)
fn calculate_visit_radius(visit: &Visit) -> f64 {
    visit
        .points
        .iter()
        .map(|p| {
            haversine_distance(
                visit.centroid_lat,
                visit.centroid_lon,
                p.latitude,
                p.longitude,
            )
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // San Francisco to Los Angeles (approx 559 km)
        let dist = haversine_distance(37.7749, -122.4194, 34.0522, -118.2437);
        assert!((dist - 559_000.0).abs() < 10_000.0); // Within 10km
    }

    #[test]
    fn test_detect_sampling_rate() {
        let points = vec![
            LocationPoint {
                id: Uuid::new_v4(),
                latitude: 37.7749,
                longitude: -122.4194,
                timestamp: Utc::now(),
                horizontal_accuracy: Some(10.0),
                _speed: Some(0.0),
            },
            LocationPoint {
                id: Uuid::new_v4(),
                latitude: 37.7749,
                longitude: -122.4194,
                timestamp: Utc::now() + Duration::seconds(10),
                horizontal_accuracy: Some(10.0),
                _speed: Some(0.0),
            },
        ];

        let rate = detect_sampling_rate(&points);
        assert!((rate - 6.0).abs() < 1.0); // Should be ~6 points/min
    }

    #[test]
    fn test_generate_visit_id_deterministic() {
        let source_id = Uuid::new_v4();
        let lat = 37.7749;
        let lon = -122.4194;
        let time = Utc::now();

        let id1 = generate_visit_id(source_id, lat, lon, time);
        let id2 = generate_visit_id(source_id, lat, lon, time);

        assert_eq!(id1, id2);
    }
}
