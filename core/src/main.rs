//! Ariata CLI - Command-line interface for the Ariata personal data platform

use ariata::cli::types::Cli;
use ariata::{AriataBuilder};
use clap::Parser;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    let _ = dotenv::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // Handle Init command early (doesn't need Ariata client)
    if matches!(cli.command, ariata::cli::types::Commands::Init) {
        let config = ariata::setup::run_init().await?;

        // Save configuration
        ariata::setup::save_config(&config)?;

        // Run migrations if requested
        if config.run_migrations {
            println!();
            println!("📊 Running migrations...");
            let db = ariata::database::Database::new(&config.database_url)?;
            db.initialize().await?;
            println!("✅ Migrations complete");
        }

        ariata::setup::display_completion();
        return Ok(());
    }

    // Get database URL from environment
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost/ariata".to_string());

    // Initialize Ariata client with optional S3/MinIO configuration
    let mut builder = AriataBuilder::new().postgres(&database_url);

    // Configure S3/MinIO storage if environment variables are present
    if let Ok(bucket) = env::var("S3_BUCKET") {
        builder = builder.s3_bucket(&bucket);

        if let Ok(endpoint) = env::var("S3_ENDPOINT") {
            builder = builder.s3_endpoint(&endpoint);
        }
        if let Ok(access_key) = env::var("S3_ACCESS_KEY") {
            if let Ok(secret_key) = env::var("S3_SECRET_KEY") {
                builder = builder.s3_credentials(&access_key, &secret_key);
            }
        }
    }

    let ariata = builder.build().await?;

    // Run CLI commands
    ariata::cli::run(cli, ariata).await?;

    Ok(())
}
