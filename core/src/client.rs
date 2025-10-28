//! Ariata client - Main interface for the Ariata data pipeline

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::database::Database;
use crate::storage::Storage;
use crate::error::{Error, Result};

/// Main Ariata client for managing personal data
pub struct Ariata {
    pub database: Arc<Database>,
    pub storage: Arc<Storage>,
}

impl Ariata {
    /// Create a new Ariata client builder
    pub fn builder() -> AriataBuilder {
        AriataBuilder::default()
    }

    /// Initialize the client and verify connections
    pub async fn initialize(&self) -> Result<()> {
        self.database.initialize().await?;
        self.storage.initialize().await?;
        Ok(())
    }

    /// Get the status of all components
    pub async fn status(&self) -> Result<Status> {
        let db_status = self.database.health_check().await?;
        let storage_status = self.storage.health_check().await?;

        Ok(Status {
            is_healthy: db_status.is_healthy && storage_status.is_healthy,
            database_status: format!("{db_status:?}"),
            storage_status: format!("{storage_status:?}"),
            active_sources: 0, // TODO: Implement
        })
    }

    /// Execute a SQL query
    pub async fn query(&self, sql: &str) -> Result<Vec<HashMap<String, Value>>> {
        self.database.query(sql).await
    }

    /// Ingest data from a source
    pub async fn ingest(&self, source: &str, data: Value) -> Result<IngestResult> {
        // Store raw data
        let key = format!("raw/{}/{}.json", source, chrono::Utc::now().timestamp());
        let bytes = serde_json::to_vec(&data)?;
        self.storage.upload(&key, bytes).await?;

        // Process and store in database
        // TODO: Implement processing pipeline

        Ok(IngestResult {
            records_ingested: 1,
            source: source.to_string(),
        })
    }

    // Source management operations are now in api.rs
    // Use ariata::list_sources(), ariata::sync_stream(), etc.

    /// Run the HTTP ingestion server
    pub async fn run_server(&self, host: &str, port: u16) -> Result<()> {
        use crate::server;
        server::run(self.clone(), host, port).await
    }
}

impl Clone for Ariata {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            storage: self.storage.clone(),
        }
    }
}

/// Builder for creating Ariata clients
#[derive(Default)]
pub struct AriataBuilder {
    postgres_url: Option<String>,
    s3_bucket: Option<String>,
    s3_endpoint: Option<String>,
    s3_access_key: Option<String>,
    s3_secret_key: Option<String>,
    storage_path: Option<String>,
}

impl AriataBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set PostgreSQL connection string
    pub fn postgres(mut self, url: &str) -> Self {
        self.postgres_url = Some(url.to_string());
        self
    }

    /// Set S3 bucket name
    pub fn s3_bucket(mut self, bucket: &str) -> Self {
        self.s3_bucket = Some(bucket.to_string());
        self
    }

    /// Set S3 endpoint (for MinIO)
    pub fn s3_endpoint(mut self, endpoint: &str) -> Self {
        self.s3_endpoint = Some(endpoint.to_string());
        self
    }

    /// Set S3 credentials
    pub fn s3_credentials(mut self, access_key: &str, secret_key: &str) -> Self {
        self.s3_access_key = Some(access_key.to_string());
        self.s3_secret_key = Some(secret_key.to_string());
        self
    }

    /// Set S3 access key
    pub fn s3_access_key(mut self, access_key: &str) -> Self {
        self.s3_access_key = Some(access_key.to_string());
        self
    }

    /// Set S3 secret key
    pub fn s3_secret_key(mut self, secret_key: &str) -> Self {
        self.s3_secret_key = Some(secret_key.to_string());
        self
    }

    /// Set local storage path
    pub fn storage_path(mut self, path: &str) -> Self {
        self.storage_path = Some(path.to_string());
        self
    }

    /// Build the Ariata client
    pub async fn build(self) -> Result<Ariata> {
        let postgres_url = self.postgres_url
            .or_else(|| std::env::var("DATABASE_URL").ok())
            .ok_or_else(|| Error::Configuration("PostgreSQL URL required".to_string()))?;

        let database = Database::new(&postgres_url)?;

        let storage = if let Some(bucket) = self.s3_bucket {
            Storage::s3(
                bucket,
                self.s3_endpoint,
                self.s3_access_key,
                self.s3_secret_key,
            ).await?
        } else {
            let path = self.storage_path.unwrap_or_else(|| "./data".to_string());
            Storage::local(path)?
        };

        Ok(Ariata {
            database: Arc::new(database),
            storage: Arc::new(storage),
        })
    }
}

/// Status of the Ariata system
#[derive(Debug, Serialize)]
pub struct Status {
    pub is_healthy: bool,
    pub database_status: String,
    pub storage_status: String,
    pub active_sources: usize,
}

/// Result of an ingestion operation
#[derive(Debug, Serialize)]
pub struct IngestResult {
    pub records_ingested: usize,
    pub source: String,
}

// Source and sync types are now in api.rs
// Use ariata::Source, ariata::SyncLog, etc.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let builder = AriataBuilder::new()
            .postgres("postgresql://localhost/test")
            .s3_bucket("test-bucket")
            .s3_endpoint("localhost:9000");

        assert!(builder.postgres_url.is_some());
        assert!(builder.s3_bucket.is_some());
        assert!(builder.s3_endpoint.is_some());
    }
}