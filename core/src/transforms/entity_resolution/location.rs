//! Entity Resolution: Location Visit → Place (POI)
//!
//! Resolves location visits to canonical place entities using Overpass API POI search.
//! Only creates place entities for actual points of interest (tourism, amenity, historic, etc.).
//!
//! ## Transform Flow
//!
//! ```text
//! location_visit (lat/lon)
//!   → Overpass API (POI search within 100m radius)
//!   → Filter: tourism, amenity, historic, shop, leisure
//!   → entities_place (create with ALL POI candidates)
//!   → location_visit.place_id (link)
//!   → Semantic enrichment layer selects correct POI from candidates
//! ```
//!
//! ## POI Categories
//!
//! Searches OpenStreetMap for these tag types:
//! - `tourism`: Museums, attractions, viewpoints, hotels
//! - `amenity`: Restaurants, cafes, bars, banks
//! - `historic`: Archaeological sites, monuments, ruins
//! - `shop`: Stores, markets, malls
//! - `leisure`: Parks, stadiums, gardens
//!
//! ## Rate Limiting
//!
//! Overpass public API allows ~10,000 requests/day.
//! This transform conservatively uses 1 req/sec throttling.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::database::Database;
use crate::error::{Error, Result};
use crate::sources::base::{OntologyTransform, TransformResult};

/// Overpass API rate limit: 1 request per second (conservative)
const OVERPASS_RATE_LIMIT_MS: u64 = 1000;

/// POI search radius (meters) - accounts for GPS accuracy ±5-20m + clustering variance
const POI_SEARCH_RADIUS_METERS: f64 = 100.0;

/// Maximum distance (meters) to consider two places as the same in database
const PLACE_DEDUPLICATION_RADIUS_METERS: f64 = 50.0;

/// Location visit for place resolution
#[derive(Debug, Clone)]
struct LocationVisit {
    id: Uuid,
    centroid_lat: f64,
    centroid_lon: f64,
    _start_time: DateTime<Utc>,
    _end_time: DateTime<Utc>,
}

/// POI candidate from Overpass search
#[derive(Debug, Clone, Serialize)]
struct PoiCandidate {
    name: String,
    poi_type: String,
    distance_m: f64,
    osm_id: i64,
    lat: f64,
    lon: f64,
}

/// Resolved place information from Nominatim
#[derive(Debug, Clone)]
struct PlaceInfo {
    name: Option<String>,
    street: Option<String>,
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
    postal_code: Option<String>,
    lat: f64,
    lon: f64,
    candidates: Vec<PoiCandidate>,
}

/// Transform to resolve location visits to places
pub struct LocationPlaceResolutionTransform;

#[async_trait]
impl OntologyTransform for LocationPlaceResolutionTransform {
    fn source_table(&self) -> &str {
        "location_visit"
    }

    fn target_table(&self) -> &str {
        "entities_place"
    }

    fn domain(&self) -> &str {
        "location"
    }

    #[tracing::instrument(skip(self, db, _context), fields(
        source_table = %self.source_table(),
        target_table = %self.target_table()
    ))]
    async fn transform(
        &self,
        db: &Database,
        _context: &crate::jobs::transform_context::TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting location place resolution"
        );

        // 1. Fetch visits without place_id
        let visits = fetch_unresolved_visits(db).await?;

        if visits.is_empty() {
            tracing::info!("No unresolved location visits found");
            return Ok(TransformResult {
                records_read: 0,
                records_written: 0,
                records_failed: 0,
                last_processed_id: None,
                chained_transforms: vec![],
            });
        }

        tracing::info!(
            visit_count = visits.len(),
            "Found unresolved location visits"
        );

        // 2. Resolve each visit to a place
        let mut records_written = 0;
        let mut records_failed = 0;

        for (i, visit) in visits.iter().enumerate() {
            // Rate limit: 1 req/sec
            if i > 0 {
                tokio::time::sleep(StdDuration::from_millis(OVERPASS_RATE_LIMIT_MS)).await;
            }

            tracing::debug!(
                visit_id = %visit.id,
                lat = visit.centroid_lat,
                lon = visit.centroid_lon,
                "Resolving place for visit"
            );

            match resolve_place_for_visit(db, visit).await {
                Ok(_) => {
                    records_written += 1;
                    tracing::debug!(
                        visit_id = %visit.id,
                        "Successfully resolved place"
                    );
                }
                Err(e) => {
                    records_failed += 1;
                    tracing::warn!(
                        visit_id = %visit.id,
                        error = %e,
                        "Failed to resolve place"
                    );
                }
            }
        }

        let total_duration = transform_start.elapsed();
        tracing::info!(
            source_id = %source_id,
            records_read = visits.len(),
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            "Location place resolution completed"
        );

        Ok(TransformResult {
            records_read: visits.len(),
            records_written,
            records_failed,
            last_processed_id: None,
            chained_transforms: vec![],
        })
    }
}

/// Fetch location visits that don't have a place_id
async fn fetch_unresolved_visits(db: &Database) -> Result<Vec<LocationVisit>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            latitude as centroid_lat,
            longitude as centroid_lon,
            start_time,
            end_time
        FROM data.location_visit
        WHERE place_id IS NULL
        ORDER BY start_time DESC
        LIMIT 100
        "#
    )
    .fetch_all(db.pool())
    .await?;

    let visits = rows
        .into_iter()
        .map(|row| LocationVisit {
            id: row.id,
            centroid_lat: row.centroid_lat,
            centroid_lon: row.centroid_lon,
            _start_time: row.start_time,
            _end_time: row.end_time,
        })
        .collect();

    Ok(visits)
}

/// Resolve a place for a visit using Overpass POI search
async fn resolve_place_for_visit(
    db: &Database,
    visit: &LocationVisit,
) -> Result<()> {
    // 1. Check if a place already exists within 50m radius
    if let Some(place_id) = find_nearby_place(db, visit.centroid_lat, visit.centroid_lon).await? {
        // Link existing place to visit
        link_visit_to_place(db, visit.id, place_id).await?;
        tracing::debug!(
            visit_id = %visit.id,
            place_id = %place_id,
            "Linked to existing nearby place"
        );
        return Ok(());
    }

    // 2. Query Overpass for nearby POIs
    let place_info = match query_overpass_poi(visit.centroid_lat, visit.centroid_lon).await? {
        Some(info) => info,
        None => {
            // No POI found - skip this visit
            tracing::debug!(
                visit_id = %visit.id,
                lat = visit.centroid_lat,
                lon = visit.centroid_lon,
                "No POI found within radius, skipping visit"
            );
            return Err(Error::NotFound("No POI found".to_string()));
        }
    };

    // 3. Create new place entity
    let place_id = create_place_entity(db, &place_info).await?;

    // 4. Link visit to place
    link_visit_to_place(db, visit.id, place_id).await?;

    tracing::debug!(
        visit_id = %visit.id,
        place_id = %place_id,
        place_name = ?place_info.name,
        "Created and linked new POI place"
    );

    Ok(())
}

/// Find an existing place within radius
async fn find_nearby_place(db: &Database, lat: f64, lon: f64) -> Result<Option<Uuid>> {
    let row = sqlx::query!(
        r#"
        SELECT id
        FROM data.entities_place
        WHERE ST_DWithin(
            geo_center::geography,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography,
            $3
        )
        ORDER BY ST_Distance(
            geo_center::geography,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography
        )
        LIMIT 1
        "#,
        lon,
        lat,
        PLACE_DEDUPLICATION_RADIUS_METERS
    )
    .fetch_optional(db.pool())
    .await?;

    Ok(row.map(|r| r.id))
}

/// Overpass API response
#[derive(Debug, Deserialize)]
struct OverpassResponse {
    elements: Vec<OverpassElement>,
}

/// Overpass POI element
#[derive(Debug, Deserialize)]
struct OverpassElement {
    #[serde(rename = "type")]
    _element_type: String,
    id: i64,
    #[serde(default)]
    lat: f64,
    #[serde(default)]
    lon: f64,
    #[serde(default)]
    center: Option<OverpassCenter>,
    tags: HashMap<String, String>,
}

/// Center coordinate for ways/relations
#[derive(Debug, Deserialize)]
struct OverpassCenter {
    lat: f64,
    lon: f64,
}

/// Query Overpass API for POIs near coordinates
async fn query_overpass_poi(lat: f64, lon: f64) -> Result<Option<PlaceInfo>> {
    // Query for POIs within 100m radius (using nwr for nodes, ways, relations)
    let radius = POI_SEARCH_RADIUS_METERS as u32;
    let query = format!(
        r#"[out:json][timeout:25];
(
  nwr["tourism"](around:{},{},{});
  nwr["amenity"](around:{},{},{});
  nwr["historic"](around:{},{},{});
  nwr["shop"](around:{},{},{});
  nwr["leisure"](around:{},{},{});
);
out center;"#,
        radius, lat, lon,
        radius, lat, lon,
        radius, lat, lon,
        radius, lat, lon,
        radius, lat, lon
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://overpass-api.de/api/interpreter")
        .body(query)
        .send()
        .await
        .map_err(|e| Error::Other(format!("Overpass HTTP error: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::Other(format!(
            "Overpass API returned status: {}",
            response.status()
        )));
    }

    let overpass_response: OverpassResponse = response
        .json()
        .await
        .map_err(|e| Error::Other(format!("Failed to parse Overpass response: {}", e)))?;

    // Build list of all POI candidates with names
    let mut candidates: Vec<PoiCandidate> = overpass_response
        .elements
        .into_iter()
        .filter(|e| e.tags.contains_key("name"))
        .filter_map(|e| {
            // Extract coordinates (nodes have direct lat/lon, ways/relations have center)
            let (poi_lat, poi_lon) = if e.lat != 0.0 && e.lon != 0.0 {
                (e.lat, e.lon)
            } else if let Some(center) = &e.center {
                (center.lat, center.lon)
            } else {
                // Skip elements without coordinates
                return None;
            };

            let distance = calculate_distance(lat, lon, poi_lat, poi_lon);
            let poi_type = e
                .tags
                .get("tourism")
                .map(|v| format!("tourism={}", v))
                .or_else(|| e.tags.get("amenity").map(|v| format!("amenity={}", v)))
                .or_else(|| e.tags.get("historic").map(|v| format!("historic={}", v)))
                .or_else(|| e.tags.get("shop").map(|v| format!("shop={}", v)))
                .or_else(|| e.tags.get("leisure").map(|v| format!("leisure={}", v)))
                .unwrap_or_else(|| "unknown".to_string());

            Some(PoiCandidate {
                name: e.tags.get("name").cloned().unwrap_or_default(),
                poi_type,
                distance_m: distance,
                osm_id: e.id,
                lat: poi_lat,
                lon: poi_lon,
            })
        })
        .collect();

    // Sort by distance (closest first)
    candidates.sort_by(|a, b| {
        a.distance_m
            .partial_cmp(&b.distance_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if candidates.is_empty() {
        // No POI found
        return Ok(None);
    }

    // Use closest POI for place info, but store all candidates
    let closest = &candidates[0];

    let place_info = PlaceInfo {
        name: Some(closest.name.clone()),
        street: None,
        city: None,
        state: None,
        country: None,
        postal_code: None,
        lat: closest.lat,
        lon: closest.lon,
        candidates: candidates,
    };

    Ok(Some(place_info))
}

/// Calculate Haversine distance between two lat/lon points in meters
fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6371000.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

/// Create a new place entity in the database
async fn create_place_entity(db: &Database, place_info: &PlaceInfo) -> Result<Uuid> {
    let place_id = Uuid::new_v4();

    // Build canonical name from available components
    let canonical_name = place_info
        .name
        .clone()
        .or_else(|| {
            place_info
                .street
                .as_ref()
                .map(|s| format!("{}, {}", s, place_info.city.as_deref().unwrap_or("Unknown")))
        })
        .unwrap_or_else(|| "Unknown Location".to_string());

    // Serialize all POI candidates
    let osm_candidates = serde_json::to_value(&place_info.candidates)
        .unwrap_or_else(|_| serde_json::json!([]));

    // Create metadata with all POI candidates
    // Note: Candidates are sorted by distance (closest first)
    // Semantic enrichment layer should select based on context, not just distance
    let metadata = serde_json::json!({
        "name": place_info.name,
        "street": place_info.street,
        "city": place_info.city,
        "state": place_info.state,
        "country": place_info.country,
        "postal_code": place_info.postal_code,
        "osm_candidates": osm_candidates,
        "resolution_status": "pending_enrichment",
        "resolved_at": chrono::Utc::now().to_rfc3339(),
        "resolution_provider": "overpass"
    });

    let point_wkt = format!("POINT({} {})", place_info.lon, place_info.lat);

    sqlx::query!(
        r#"
        INSERT INTO data.entities_place (
            id,
            canonical_name,
            category,
            geo_center,
            bounding_box,
            metadata,
            created_at,
            updated_at
        ) VALUES (
            $1, $2, NULL, ST_SetSRID(ST_GeomFromText($3), 4326), NULL, $4, NOW(), NOW()
        )
        "#,
        place_id,
        canonical_name,
        point_wkt,
        metadata
    )
    .execute(db.pool())
    .await?;

    Ok(place_id)
}

/// Link a visit to a place
async fn link_visit_to_place(db: &Database, visit_id: Uuid, place_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE data.location_visit
        SET place_id = $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
        place_id,
        visit_id
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_deduplication_radius() {
        // 50 meters is a reasonable radius for considering two visits
        // at the same place
        assert_eq!(PLACE_DEDUPLICATION_RADIUS_METERS, 50.0);
    }

    #[test]
    fn test_overpass_rate_limit() {
        // 1 request per second (conservative)
        assert_eq!(OVERPASS_RATE_LIMIT_MS, 1000);
    }
}
