//! Ariata Seed - Database seeding utility
//!
//! Seeds the database with Monday in Rome reference dataset

use ariata::database::Database;
use ariata::seeding::{seed_monday_in_rome_dataset, narratives::seed_rome_monday_narrative};
use ariata::storage::{Storage, stream_writer::StreamWriter};
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
            info!("✅ Monday in Rome ontology seeding completed: {} records!", count);
        }
        Err(e) => {
            error!("❌ Monday in Rome seeding failed: {}", e);
            std::process::exit(1);
        }
    }

    // Seed the narrative layer
    info!("Seeding Rome Monday day narrative...");
    match seed_rome_monday_narrative(&db).await {
        Ok(count) => {
            info!("✅ Rome Monday narrative seeding completed: {} narrative chunks!", count);
            std::process::exit(0);
        }
        Err(e) => {
            error!("❌ Rome Monday narrative seeding failed: {}", e);
            std::process::exit(1);
        }
    }
}
