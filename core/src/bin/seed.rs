//! Ariata Seed - Database seeding utility
//!
//! Seeds the database with Monday in Rome reference dataset

use ariata::database::Database;
use ariata::jobs::narrative_primitive_pipeline;
use ariata::seeding::seed_monday_in_rome_dataset;
use ariata::storage::{stream_writer::StreamWriter, Storage};
use chrono::{DateTime, Duration, Utc};
use clap::Parser;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "ariata-seed")]
#[command(about = "Ariata Seed - Monday in Rome Reference Dataset")]
struct Args {
    /// Run narrative primitive pipeline after seeding
    #[arg(long, default_value_t = false)]
    run_pipeline: bool,

    /// Simulate hourly cron by running pipeline 12 times with sliding windows (implies --run-pipeline)
    #[arg(long, default_value_t = false)]
    simulate_hourly: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ariata_seed=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🇮🇹 Ariata Seed - Monday in Rome Reference Dataset");

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

    // Initialize storage and stream writer for pipeline seeding
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

    info!("Seeding Monday in Rome dataset...");
    info!("This tests the full pipeline: CSV → Archive → Transform → Ontology tables");

    match seed_monday_in_rome_dataset(&db, &storage, stream_writer).await {
        Ok(count) => {
            info!(
                "✅ Monday in Rome ontology seeding completed: {} records!",
                count
            );
        }
        Err(e) => {
            error!("❌ Monday in Rome seeding failed: {}", e);
            std::process::exit(1);
        }
    }

    // Optional: Run narrative primitive pipeline (generates narrative_primitive records)
    if args.simulate_hourly || args.run_pipeline {
        info!("🔄 Running narrative primitive pipeline...");

        if args.simulate_hourly {
            info!("⏰ Simulating hourly cron execution (12 hours of seed data)");

            // Base time: Nov 10, 2025 07:00 UTC (matches actual seeded data timestamps)
            let base = DateTime::parse_from_rfc3339("2025-11-10T07:00:00Z")
                .unwrap()
                .with_timezone(&Utc);

            let mut total_primitives = 0;
            let mut dedupe_count = 0;

            // Run pipeline 12 times (simulating 12 hours)
            for hour in 0..12 {
                let current_time = base + Duration::hours(hour);
                let window_start = current_time - Duration::hours(6); // 6h lookback
                let window_end = current_time;

                info!(
                    hour = hour,
                    window_start = %window_start,
                    window_end = %window_end,
                    "Running pipeline (simulated hour {})",
                    hour
                );

                match narrative_primitive_pipeline::run_pipeline_for_range(
                    &db,
                    window_start,
                    window_end,
                )
                .await
                {
                    Ok(stats) => {
                        let new_primitives = stats.primitives_created;
                        let expected_dedupe = hour > 0; // Hours 1+ should see duplicates

                        info!(
                            hour = hour,
                            places = stats.places_resolved,
                            people = stats.people_resolved,
                            boundaries = stats.boundaries_detected,
                            primitives = new_primitives,
                            duration_ms = stats.duration_ms,
                            "✅ Hour {} completed",
                            hour
                        );

                        // Track if deduplication is working
                        if new_primitives == 0 && expected_dedupe {
                            dedupe_count += 1;
                        }

                        total_primitives += new_primitives;
                    }
                    Err(e) => {
                        warn!(hour = hour, error = %e, "⚠️  Hour {} failed", hour);
                    }
                }

                // Small delay to separate log entries
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            info!(
                total_primitives = total_primitives,
                dedupe_runs = dedupe_count,
                "✅ Hourly simulation complete. \
                 Expected: primitives created in early hours, then dedupe prevents duplicates."
            );
        } else {
            // Single run for full range (matches actual seeded data: Nov 10, 07:21 - 18:20)
            let start = DateTime::parse_from_rfc3339("2025-11-10T07:00:00Z")
                .unwrap()
                .with_timezone(&Utc);
            let end = DateTime::parse_from_rfc3339("2025-11-10T19:00:00Z")
                .unwrap()
                .with_timezone(&Utc);

            match narrative_primitive_pipeline::run_pipeline_for_range(&db, start, end).await {
                Ok(stats) => {
                    info!(
                        places = stats.places_resolved,
                        people = stats.people_resolved,
                        boundaries = stats.boundaries_detected,
                        primitives = stats.primitives_created,
                        duration_ms = stats.duration_ms,
                        "✅ Pipeline completed"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "⚠️  Pipeline failed");
                }
            }
        }
    }

    std::process::exit(0);
}
