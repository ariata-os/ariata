//! Storage module for S3/MinIO and local file operations

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{Error, Result};

/// Storage trait for different backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn initialize(&self) -> Result<()>;
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()>;
    async fn download(&self, key: &str) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    async fn health_check(&self) -> Result<HealthStatus>;
}

/// Main storage interface
#[derive(Clone)]
pub struct Storage {
    backend: Arc<dyn StorageBackend>,
}

impl Storage {
    /// Create S3/MinIO storage
    pub async fn s3(
        bucket: String,
        endpoint: Option<String>,
        access_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            backend: Arc::new(S3Storage::new(bucket, endpoint, access_key, secret_key).await?),
        })
    }

    /// Create local file storage
    pub fn local(path: String) -> Result<Self> {
        Ok(Self {
            backend: Arc::new(LocalStorage::new(path)?),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        self.backend.initialize().await
    }

    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.backend.upload(key, data).await
    }

    pub async fn put_object(&self, key: &str, data: &[u8]) -> Result<()> {
        self.backend.upload(key, data.to_vec()).await
    }

    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        self.backend.download(key).await
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.backend.delete(key).await
    }

    pub async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        self.backend.list(prefix).await
    }

    pub async fn health_check(&self) -> Result<HealthStatus> {
        self.backend.health_check().await
    }
}

/// S3/MinIO storage backend
pub struct S3Storage {
    bucket: String,
    client: aws_sdk_s3::Client,
}

impl S3Storage {
    pub async fn new(
        bucket: String,
        endpoint: Option<String>,
        access_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Self> {
        // Build AWS configuration
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        // Configure endpoint if provided (for MinIO)
        if let Some(endpoint_url) = endpoint {
            config_loader = config_loader.endpoint_url(endpoint_url);
        }

        // Configure credentials if provided
        if let (Some(access_key), Some(secret_key)) = (access_key, secret_key) {
            let creds = aws_sdk_s3::config::Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "manual",
            );
            config_loader = config_loader.credentials_provider(creds);
        }

        let config = config_loader.load().await;
        let client = aws_sdk_s3::Client::new(&config);

        Ok(Self { bucket, client })
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn initialize(&self) -> Result<()> {
        // Check if bucket exists, create if not
        Ok(())
    }

    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await
            .map_err(|e| Error::S3(e.to_string()))?;

        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Error::S3(e.to_string()))?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| Error::S3(e.to_string()))?
            .into_bytes();

        Ok(bytes.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Error::S3(e.to_string()))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| Error::S3(e.to_string()))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key())
            .map(|k| k.to_string())
            .collect();

        Ok(keys)
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        // Try to list bucket
        match self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
        {
            Ok(_) => Ok(HealthStatus {
                is_healthy: true,
                message: format!("S3 bucket '{}' accessible", self.bucket),
            }),
            Err(e) => Ok(HealthStatus {
                is_healthy: false,
                message: format!("S3 error: {}", e),
            }),
        }
    }
}

/// Local file storage backend
struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    fn new(path: String) -> Result<Self> {
        Ok(Self {
            base_path: PathBuf::from(path),
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn initialize(&self) -> Result<()> {
        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&self.base_path).await?;
        Ok(())
    }

    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let path = self.base_path.join(key);

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.base_path.join(key);
        Ok(tokio::fs::read(path).await?)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.base_path.join(key);
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let prefix_path = self.base_path.join(prefix);
        let mut files = Vec::new();

        let mut dir = tokio::fs::read_dir(prefix_path).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(format!("{}/{}", prefix, name));
                }
            }
        }

        Ok(files)
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        // Check if directory exists and is writable
        match tokio::fs::metadata(&self.base_path).await {
            Ok(metadata) if metadata.is_dir() => Ok(HealthStatus {
                is_healthy: true,
                message: format!("Local storage at {:?} is accessible", self.base_path),
            }),
            _ => Ok(HealthStatus {
                is_healthy: false,
                message: format!("Local storage at {:?} not accessible", self.base_path),
            }),
        }
    }
}

/// Health status for storage
#[derive(Debug)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::local(temp_dir.path().to_str().unwrap().to_string()).unwrap();

        storage.initialize().await.unwrap();

        // Test upload
        let data = b"test data".to_vec();
        storage.upload("test.txt", data.clone()).await.unwrap();

        // Test download
        let downloaded = storage.download("test.txt").await.unwrap();
        assert_eq!(downloaded, data);

        // Test list
        let files = storage.list("").await.unwrap();
        assert!(files.iter().any(|f| f.contains("test.txt")));

        // Test delete
        storage.delete("test.txt").await.unwrap();
    }
}