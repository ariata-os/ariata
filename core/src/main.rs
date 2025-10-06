//! Ariata CLI - Command-line interface for the Ariata data pipeline

use anyhow::Result;
use ariata::AriataBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse environment variables
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let minio_endpoint = std::env::var("MINIO_ENDPOINT")
        .expect("MINIO_ENDPOINT must be set");
    let minio_user = std::env::var("MINIO_ROOT_USER")
        .unwrap_or_else(|_| "minioadmin".to_string());
    let minio_password = std::env::var("MINIO_ROOT_PASSWORD")
        .unwrap_or_else(|_| "minioadmin".to_string());

    // Build Ariata client using the correct API
    let client = AriataBuilder::new()
        .postgres(&database_url)
        .s3_bucket("ariata")
        .s3_endpoint(&minio_endpoint)
        .s3_credentials(&minio_user, &minio_password)
        .build()
        .await?;

    tracing::info!("Ariata client initialized");

    // Initialize and verify connections
    client.initialize().await?;
    tracing::info!("Database and storage initialized");

    // Start HTTP server
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    tracing::info!("Starting HTTP server on {}:{}", host, port);
    ariata::server::run(client, &host, port).await?;

    Ok(())
}