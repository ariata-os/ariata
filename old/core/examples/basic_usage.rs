//! Basic usage example for Ariata

use anyhow::Result;
use ariata::AriataBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create Ariata client with local storage
    let client = AriataBuilder::new()
        .postgres("postgresql://ariata_user:ariata_password@localhost/ariata")
        .storage_path("./data")
        .build()
        .await?;

    // Initialize the client
    client.initialize().await?;

    // Check status
    let status = client.status().await?;
    println!("Ariata Status: {:?}", status);

    // List available sources
    let sources = client.list_available_sources().await?;
    println!("Available sources: {:?}", sources);

    // Add a source
    let source = client.add_source("my-iphone", "ios").await?;
    println!("Added source: {:?}", source);

    // Ingest some data
    let test_data = serde_json::json!({
        "device_id": "iphone-001",
        "timestamp": "2024-01-01T12:00:00Z",
        "health_data": {
            "steps": 5000,
            "heart_rate": 72,
            "calories": 2000
        }
    });

    let result = client.ingest("my-iphone", test_data).await?;
    println!("Ingested {} records", result.records_ingested);

    // Query data
    let results = client
        .query("SELECT * FROM sources WHERE name = 'my-iphone'")
        .await?;

    for row in results {
        println!("Row: {:?}", row);
    }

    Ok(())
}
