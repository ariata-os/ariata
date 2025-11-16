//! Test Place Resolution
//!
//! Tests the location place resolution transform with the Rome visit data.

use ariata::database::Database;
use ariata::jobs::transform_context::{ApiKeys, TransformContext};
use ariata::sources::base::OntologyTransform;
use ariata::storage::{stream_writer::StreamWriter, Storage};
use ariata::transforms::entity_resolution::LocationPlaceResolutionTransform;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "test_place_resolution=info,ariata=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🗺️  Testing Location Place Resolution");
    info!("");

    // Get database URL from environment
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Connect to database
    info!("Connecting to database...");
    let db = match Database::new(&database_url) {
        Ok(db) => {
            info!("✅ Database connection established");
            db
        }
        Err(e) => {
            error!("❌ Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize storage
    let storage = if let Ok(bucket) = env::var("S3_BUCKET") {
        let endpoint = env::var("S3_ENDPOINT").ok();
        let access_key = env::var("S3_ACCESS_KEY").ok();
        let secret_key = env::var("S3_SECRET_KEY").ok();

        match Storage::s3(bucket, endpoint, access_key, secret_key).await {
            Ok(storage) => {
                info!("✅ S3/MinIO storage initialized");
                storage
            }
            Err(e) => {
                error!("❌ Failed to initialize S3 storage: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let path = env::var("STORAGE_PATH").unwrap_or_else(|_| "./data".to_string());
        match Storage::local(path) {
            Ok(storage) => {
                info!("✅ Local storage initialized");
                storage
            }
            Err(e) => {
                error!("❌ Failed to initialize local storage: {}", e);
                std::process::exit(1);
            }
        }
    };

    let stream_writer = Arc::new(Mutex::new(StreamWriter::new()));
    let api_keys = ApiKeys::from_env();

    // Create transform context
    let context = TransformContext::new(Arc::new(storage), stream_writer, api_keys);

    // Get primary source ID
    info!("Fetching primary source...");
    let source_id = match fetch_primary_source_id(&db).await {
        Ok(id) => {
            info!("✅ Source ID: {}", id);
            id
        }
        Err(e) => {
            error!("❌ Failed to fetch source ID: {}", e);
            std::process::exit(1);
        }
    };
    info!("");

    // Show unresolved visits before
    info!("Checking unresolved visits...");
    match count_unresolved_visits(&db).await {
        Ok(count) => {
            info!("Found {} unresolved visits", count);
        }
        Err(e) => {
            error!("Failed to count visits: {}", e);
        }
    }
    info!("");

    // Run place resolution transform
    info!("Running place resolution transform...");
    info!("⚠️  Note: Nominatim rate limit is 1 req/sec, this will take a few seconds");
    info!("");

    let transform = LocationPlaceResolutionTransform;

    match transform.transform(&db, &context, source_id).await {
        Ok(result) => {
            info!("");
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("✅ Place resolution completed successfully!");
            info!("");
            info!("Results:");
            info!("  Visits processed: {}", result.records_read);
            info!("  Places resolved: {}", result.records_written);
            info!("  Failed: {}", result.records_failed);
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("");

            // Query and display resolved places
            if result.records_written > 0 {
                info!("Fetching resolved place details...");
                match fetch_resolved_places(&db).await {
                    Ok(places) => {
                        info!("");
                        info!("Resolved Places ({} total):", places.len());
                        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        for (i, place) in places.iter().enumerate() {
                            info!("");
                            info!("Place #{}", i + 1);
                            info!("  Name: {}", place.canonical_name);
                            info!("  Location: ({:.6}, {:.6})", place.lat, place.lon);

                            if let Some(metadata) = &place.metadata {
                                if let Some(street) = metadata.get("street") {
                                    info!("  Street: {}", street.as_str().unwrap_or("N/A"));
                                }
                                if let Some(city) = metadata.get("city") {
                                    info!("  City: {}", city.as_str().unwrap_or("N/A"));
                                }
                                if let Some(country) = metadata.get("country") {
                                    info!("  Country: {}", country.as_str().unwrap_or("N/A"));
                                }
                                if let Some(postal_code) = metadata.get("postal_code") {
                                    info!("  Postal Code: {}", postal_code.as_str().unwrap_or("N/A"));
                                }
                            }

                            // Show linked visits
                            if let Ok(visit_count) = count_visits_for_place(&db, place.id).await {
                                info!("  Linked visits: {}", visit_count);
                            }
                        }
                        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    }
                    Err(e) => {
                        error!("Failed to fetch places: {}", e);
                    }
                }
            }

            std::process::exit(0);
        }
        Err(e) => {
            error!("❌ Place resolution failed: {}", e);
            std::process::exit(1);
        }
    }
}

/// Fetch the primary source ID (for single-tenant)
async fn fetch_primary_source_id(db: &Database) -> Result<uuid::Uuid, ariata::error::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id
        FROM elt.sources
        ORDER BY created_at ASC
        LIMIT 1
        "#
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| ariata::error::Error::NotFound("No sources found".to_string()))?;

    Ok(row.id)
}

/// Count unresolved visits
async fn count_unresolved_visits(db: &Database) -> Result<i64, ariata::error::Error> {
    let row = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM elt.location_visit
        WHERE place_id IS NULL
        "#
    )
    .fetch_one(db.pool())
    .await?;

    Ok(row.count.unwrap_or(0))
}

/// Place details for display
struct PlaceDetails {
    id: uuid::Uuid,
    canonical_name: String,
    lat: f64,
    lon: f64,
    metadata: Option<serde_json::Value>,
}

/// Fetch resolved places
async fn fetch_resolved_places(db: &Database) -> Result<Vec<PlaceDetails>, ariata::error::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            p.id,
            p.canonical_name,
            ST_Y(p.geo_center::geometry) as lat,
            ST_X(p.geo_center::geometry) as lon,
            p.metadata
        FROM elt.entities_place p
        WHERE EXISTS (
            SELECT 1 FROM elt.location_visit v
            WHERE v.place_id = p.id
        )
        ORDER BY p.created_at DESC
        LIMIT 20
        "#
    )
    .fetch_all(db.pool())
    .await?;

    let places = rows
        .into_iter()
        .map(|row| PlaceDetails {
            id: row.id,
            canonical_name: row.canonical_name,
            lat: row.lat.unwrap_or(0.0),
            lon: row.lon.unwrap_or(0.0),
            metadata: row.metadata,
        })
        .collect();

    Ok(places)
}

/// Count visits for a place
async fn count_visits_for_place(
    db: &Database,
    place_id: uuid::Uuid,
) -> Result<i64, ariata::error::Error> {
    let row = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM elt.location_visit
        WHERE place_id = $1
        "#,
        place_id
    )
    .fetch_one(db.pool())
    .await?;

    Ok(row.count.unwrap_or(0))
}
