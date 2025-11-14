//! Validation utilities for setup configuration

use console::style;
use sqlx::postgres::PgPoolOptions;

use crate::error::{Error, Result};
use crate::storage::Storage;

/// Test PostgreSQL connection
pub async fn test_database_connection(database_url: &str) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|e| Error::Database(format!("Connection failed: {}", e)))?;

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| Error::Database(format!("Query test failed: {}", e)))?;

    pool.close().await;

    Ok(())
}

/// Test S3/MinIO connection by attempting to access the bucket
pub async fn test_s3_connection(
    endpoint: Option<String>,
    bucket: &str,
    access_key: &str,
    secret_key: &str,
) -> Result<()> {
    let storage = Storage::s3(
        bucket.to_string(),
        endpoint,
        Some(access_key.to_string()),
        Some(secret_key.to_string()),
    )
    .await?;

    // Test health check
    let health = storage.health_check().await?;
    if !health.is_healthy {
        return Err(Error::S3(format!(
            "S3 connection unhealthy: {}",
            health.message
        )));
    }

    Ok(())
}

/// Test local storage by creating the directory and writing a test file
pub async fn test_local_storage(path: &str) -> Result<()> {
    let storage = Storage::local(path.to_string())?;

    // Initialize (creates directory)
    storage.initialize().await?;

    // Test write and read
    let test_key = ".ariata_test";
    let test_data = b"test".to_vec();

    storage.upload(test_key, test_data.clone()).await?;
    let read_data = storage.download(test_key).await?;

    if read_data != test_data {
        return Err(Error::Other(
            "Storage test failed: data mismatch".to_string(),
        ));
    }

    // Clean up
    storage.delete(test_key).await?;

    Ok(())
}

/// Display an error message with formatting
pub fn display_error(message: &str) {
    eprintln!("{} {}", style("✗").red().bold(), style(message).red());
}

/// Display a success message with formatting
pub fn display_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), style(message).green());
}

/// Display an info message with formatting
pub fn display_info(message: &str) {
    println!("{} {}", style("⏳").yellow().bold(), message);
}
