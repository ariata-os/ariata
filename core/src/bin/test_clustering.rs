//! Test Location Visit Clustering
//!
//! Tests the location visit clustering transform with configurable lookback window.
//! Useful for testing with seed data from previous days.

use ariata::database::Database;
use ariata::jobs::transform_context::{ApiKeys, TransformContext};
use ariata::sources::base::OntologyTransform;
use ariata::storage::{stream_writer::StreamWriter, Storage};
use ariata::transforms::enrich::location::LocationVisitTransform;
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
                .unwrap_or_else(|_| "test_clustering=info,ariata=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("📍 Testing Location Visit Clustering");
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
        // Fall back to local storage
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

    // Get lookback hours from environment or use default (120 hours = 5 days)
    let lookback_hours = env::var("LOOKBACK_HOURS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(120);

    info!("Configuration:");
    info!("  Lookback window: {} hours ({} days)", lookback_hours, lookback_hours / 24);
    info!("");

    // Create transform context with custom metadata
    let metadata = serde_json::json!({
        "lookback_hours": lookback_hours
    });

    let context = TransformContext::with_metadata(
        Arc::new(storage),
        stream_writer,
        api_keys,
        metadata,
    );

    // Get primary source ID (single-tenant assumption)
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

    // Create and run transform
    info!("Running location visit clustering transform...");
    info!("");
    let transform = LocationVisitTransform;

    match transform.transform(&db, &context, source_id).await {
        Ok(result) => {
            info!("");
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("✅ Clustering completed successfully!");
            info!("");
            info!("Results:");
            info!("  Location points processed: {}", result.records_read);
            info!("  Visits created/updated: {}", result.records_written);
            info!("  Failed records: {}", result.records_failed);
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("");

            // Query and display created visits
            if result.records_written > 0 {
                info!("Fetching visit details...");
                match fetch_recent_visits(&db).await {
                    Ok(visits) => {
                        info!("");
                        info!("Recent Visits ({} total):", visits.len());
                        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        for (i, visit) in visits.iter().enumerate().take(10) {
                            info!("");
                            info!("Visit #{}", i + 1);
                            info!("  Location: ({:.6}, {:.6})", visit.latitude, visit.longitude);
                            info!("  Start: {}", visit.start_time.format("%Y-%m-%d %H:%M:%S UTC"));
                            info!("  End: {}", visit.end_time.format("%Y-%m-%d %H:%M:%S UTC"));
                            info!("  Duration: {} minutes", (visit.end_time - visit.start_time).num_minutes());
                            if let Some(metadata) = &visit.metadata {
                                if let Some(point_count) = metadata.get("point_count") {
                                    info!("  Points: {}", point_count);
                                }
                                if let Some(radius) = metadata.get("radius_meters") {
                                    info!("  Radius: {:.1}m", radius.as_f64().unwrap_or(0.0));
                                }
                            }
                        }
                        if visits.len() > 10 {
                            info!("");
                            info!("... and {} more visits", visits.len() - 10);
                        }
                        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    }
                    Err(e) => {
                        error!("Failed to fetch visits: {}", e);
                    }
                }
            }

            std::process::exit(0);
        }
        Err(e) => {
            error!("❌ Clustering failed: {}", e);
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

/// Visit details for display
struct VisitDetails {
    latitude: f64,
    longitude: f64,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
    metadata: Option<serde_json::Value>,
}

/// Fetch recent visits for display
async fn fetch_recent_visits(db: &Database) -> Result<Vec<VisitDetails>, ariata::error::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            latitude,
            longitude,
            start_time,
            end_time,
            metadata
        FROM elt.location_visit
        ORDER BY start_time DESC
        LIMIT 50
        "#
    )
    .fetch_all(db.pool())
    .await?;

    let visits = rows
        .into_iter()
        .map(|row| VisitDetails {
            latitude: row.latitude,
            longitude: row.longitude,
            start_time: row.start_time,
            end_time: row.end_time,
            metadata: row.metadata,
        })
        .collect();

    Ok(visits)
}
