//! Place Resolution via Location Clustering
//!
//! Transforms raw location_point primitives into semantic location_visit records
//! using density-adaptive HDBSCAN-like clustering, then links to entities_place.
//!
//! ## Process
//!
//! 1. Fetch location_point records in time window
//! 2. Auto-detect sampling rate (points/minute)
//! 3. Run spatial-temporal clustering
//! 4. Write location_visit records
//! 5. Link visits to place entities (create if new)

use chrono::{DateTime, Utc};
use geo::HaversineDistance;
use geo::Point as GeoPoint;
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use super::TimeWindow;

/// Spatial clustering parameters
const SPATIAL_EPSILON_METERS: f64 = 75.0;  // Max distance within a cluster (better GPS drift tolerance)
const MIN_VISIT_DURATION_MINUTES: i64 = 5; // Minimum visit duration
const TEMPORAL_GAP_MINUTES: i64 = 5;        // Max time gap within visit (robust for iOS backgrounding)
const MAX_HORIZONTAL_ACCURACY: f64 = 100.0; // Filter low-quality points
const MAX_SPEED_MPS: f64 = 50.0;             // Filter high-speed transit

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

/// Resolve places for the given time window
///
/// Returns the number of visits created/updated.
pub async fn resolve_places(db: &Database, window: TimeWindow) -> Result<usize> {
    tracing::info!(
        start = %window.start,
        end = %window.end,
        "Resolving places via location clustering"
    );

    // 1. Fetch location points in window
    let points = fetch_location_points(db, window).await?;

    if points.is_empty() {
        tracing::debug!("No location points to cluster");
        return Ok(0);
    }

    tracing::debug!(
        point_count = points.len(),
        "Fetched location points"
    );

    // 2. Auto-detect sampling rate
    let sampling_rate = detect_sampling_rate(&points);
    tracing::debug!(
        points_per_minute = sampling_rate,
        "Detected sampling rate"
    );

    // 3. Run density-adaptive clustering
    let visits = cluster_location_points(&points, sampling_rate)?;

    tracing::debug!(
        visit_count = visits.len(),
        "Completed clustering"
    );

    // 4. Write visits idempotently and link to place entities
    let mut records_written = 0;
    for visit in &visits {
        match write_visit_and_link_place(db, visit).await {
            Ok(_) => records_written += 1,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to write visit"
                );
            }
        }
    }

    tracing::info!(
        visits_written = records_written,
        "Place resolution completed"
    );

    Ok(records_written)
}

/// Fetch location points from database in time window
async fn fetch_location_points(db: &Database, window: TimeWindow) -> Result<Vec<LocationPoint>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            latitude,
            longitude,
            timestamp,
            accuracy_meters as horizontal_accuracy,
            speed_meters_per_second as speed
        FROM data.location_point
        WHERE timestamp >= $1
          AND timestamp < $2
          AND (accuracy_meters IS NULL OR accuracy_meters < $3)
          AND (speed_meters_per_second IS NULL
               OR speed_meters_per_second < 0
               OR speed_meters_per_second < $4)
        ORDER BY timestamp ASC
        "#,
        window.start,
        window.end,
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

    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    gaps[gaps.len() / 2] // Median points per minute
}

/// Cluster location points using density-adaptive spatial-temporal clustering
fn cluster_location_points(points: &[LocationPoint], points_per_minute: f64) -> Result<Vec<Visit>> {
    // Calculate density-adaptive parameters
    let min_cluster_size = (MIN_VISIT_DURATION_MINUTES as f64 * points_per_minute).round() as usize;
    let min_cluster_size = min_cluster_size.max(3); // At least 3 points

    tracing::debug!(
        min_cluster_size,
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
fn generate_visit_id(centroid_lat: f64, centroid_lon: f64, start_time: DateTime<Utc>) -> Uuid {
    // Round coordinates to ~10 meter precision (4 decimal places)
    let lat_rounded = (centroid_lat * 10000.0).round() / 10000.0;
    let lon_rounded = (centroid_lon * 10000.0).round() / 10000.0;

    // Round start time to nearest minute
    let timestamp_secs = start_time.timestamp();
    let rounded_secs = (timestamp_secs / 60) * 60;
    let start_rounded = DateTime::from_timestamp(rounded_secs, 0)
        .unwrap_or(start_time);

    // Create deterministic UUID v5
    let hash_input = format!(
        "{}:{}:{}",
        lat_rounded,
        lon_rounded,
        start_rounded.to_rfc3339()
    );

    Uuid::new_v5(&Uuid::NAMESPACE_OID, hash_input.as_bytes())
}

/// Write visit idempotently to database and link to place entity
async fn write_visit_and_link_place(db: &Database, visit: &Visit) -> Result<()> {
    let visit_id = generate_visit_id(visit.centroid_lat, visit.centroid_lon, visit.start_time);

    // Find or create place entity for this location
    let place_id = resolve_or_create_place(db, visit.centroid_lat, visit.centroid_lon).await?;

    let point_wkt = format!("POINT({} {})", visit.centroid_lon, visit.centroid_lat);

    let metadata = serde_json::json!({
        "point_count": visit.points.len(),
        "radius_meters": calculate_visit_radius(visit),
    });

    sqlx::query!(
        r#"
        INSERT INTO data.location_visit (
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
            $1, $2, ST_GeogFromText($3), $4, $5, $6, $7, $8, $9, $10, $11
        )
        ON CONFLICT (id) DO UPDATE SET
            place_id = EXCLUDED.place_id,
            end_time = EXCLUDED.end_time,
            centroid_coordinates = EXCLUDED.centroid_coordinates,
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            metadata = EXCLUDED.metadata,
            updated_at = NOW()
        WHERE data.location_visit.end_time < EXCLUDED.end_time
           OR data.location_visit.place_id IS NULL
        "#,
        visit_id,
        place_id,
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

/// Find or create a place entity for the given coordinates
///
/// This function checks if a place entity exists within 75 meters of the given coordinates.
/// If found, returns its ID. If not, creates a new place entity with reverse geocoded name.
async fn resolve_or_create_place(db: &Database, lat: f64, lon: f64) -> Result<Uuid> {
    // Check for existing place within 75 meters (match SPATIAL_EPSILON_METERS)
    let existing = sqlx::query!(
        r#"
        SELECT id, canonical_name
        FROM data.entities_place
        WHERE ST_DWithin(
            geo_center::geography,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography,
            75.0
        )
        ORDER BY ST_Distance(
            geo_center::geography,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography
        )
        LIMIT 1
        "#,
        lon,
        lat
    )
    .fetch_optional(db.pool())
    .await?;

    if let Some(place) = existing {
        tracing::debug!(
            place_id = %place.id,
            place_name = %place.canonical_name,
            "Found existing place entity"
        );
        return Ok(place.id);
    }

    // Create new place entity with reverse geocoded name
    let canonical_name = reverse_geocode_stub(lat, lon);

    let place_id = sqlx::query!(
        r#"
        INSERT INTO data.entities_place (
            canonical_name,
            geo_center,
            cluster_radius_meters,
            metadata
        ) VALUES (
            $1,
            ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography,
            75.0,
            '{}'::jsonb
        )
        RETURNING id
        "#,
        canonical_name,
        lon,
        lat
    )
    .fetch_one(db.pool())
    .await?
    .id;

    tracing::info!(
        place_id = %place_id,
        place_name = %canonical_name,
        lat = %lat,
        lon = %lon,
        "Created new place entity"
    );

    Ok(place_id)
}

/// Reverse geocode coordinates to human-readable name (stub implementation)
///
/// This is a placeholder implementation that generates a simple label based on coordinates.
/// In production, this should be replaced with a proper reverse geocoding service like:
/// - Nominatim (free, open source)
/// - Google Places API (requires API key)
/// - Mapbox Geocoding API
///
/// TODO: Implement proper reverse geocoding with rate limiting
fn reverse_geocode_stub(lat: f64, lon: f64) -> String {
    format!("Location {:.4}, {:.4}", lat, lon)
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
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
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
        let lat = 37.7749;
        let lon = -122.4194;
        let time = Utc::now();

        let id1 = generate_visit_id(lat, lon, time);
        let id2 = generate_visit_id(lat, lon, time);

        assert_eq!(id1, id2);
    }
}
