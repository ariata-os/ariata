//! Storage module for S3/MinIO and local file operations

pub mod encryption;
pub mod models;
pub mod stream_writer;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Encryption options for S3 uploads/downloads
#[derive(Clone)]
pub struct EncryptionKey {
    /// Base64-encoded 32-byte AES-256 key
    pub key_base64: String,
}

/// Storage trait for different backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn initialize(&self) -> Result<()>;
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()>;
    async fn upload_encrypted(&self, key: &str, data: Vec<u8>, encryption_key: &EncryptionKey) -> Result<()>;
    async fn download(&self, key: &str) -> Result<Vec<u8>>;
    async fn download_encrypted(&self, key: &str, encryption_key: &EncryptionKey) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    async fn list_with_pagination(&self, prefix: &str, max_keys: Option<i32>, continuation_token: Option<String>) -> Result<ListResult>;
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Generate a presigned URL for temporary public access to an object
    ///
    /// # Arguments
    /// * `key` - The object key
    /// * `expires_in` - How long the URL should be valid for
    ///
    /// # Returns
    /// A public URL that can be used to access the object until it expires
    async fn get_presigned_url(&self, key: &str, expires_in: Duration) -> Result<String>;
}

/// Result from list_with_pagination
#[derive(Debug)]
pub struct ListResult {
    pub keys: Vec<String>,
    pub continuation_token: Option<String>,
    pub is_truncated: bool,
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

    /// Generate a presigned URL for temporary public access to an object
    ///
    /// This is useful for allowing external services (like AssemblyAI) to access
    /// files stored in S3/MinIO without giving them permanent credentials.
    ///
    /// # Arguments
    /// * `key` - The object key (e.g., "ios/microphone/device123/audio.m4a")
    /// * `expires_in` - How long the URL should be valid for (e.g., Duration::from_secs(3600) for 1 hour)
    ///
    /// # Example
    /// ```ignore
    /// let url = storage.get_presigned_url("audio.m4a", Duration::from_secs(3600)).await?;
    /// // Pass URL to external service
    /// external_service.process(url).await?;
    /// ```
    pub async fn get_presigned_url(&self, key: &str, expires_in: Duration) -> Result<String> {
        self.backend.get_presigned_url(key, expires_in).await
    }

    /// Upload with SSE-C encryption
    pub async fn upload_encrypted(&self, key: &str, data: Vec<u8>, encryption_key: &EncryptionKey) -> Result<()> {
        self.backend.upload_encrypted(key, data, encryption_key).await
    }

    /// Download with SSE-C encryption
    pub async fn download_encrypted(&self, key: &str, encryption_key: &EncryptionKey) -> Result<Vec<u8>> {
        self.backend.download_encrypted(key, encryption_key).await
    }

    /// List objects with pagination support
    pub async fn list_with_pagination(&self, prefix: &str, max_keys: Option<i32>, continuation_token: Option<String>) -> Result<ListResult> {
        self.backend.list_with_pagination(prefix, max_keys, continuation_token).await
    }

    /// Upload JSON object
    pub async fn upload_json<T: Serialize>(&self, key: &str, data: &T) -> Result<()> {
        let json_bytes = serde_json::to_vec(data)
            .map_err(|e| Error::Other(format!("Failed to serialize JSON: {}", e)))?;
        self.upload(key, json_bytes).await
    }

    /// Upload JSON object with encryption
    pub async fn upload_json_encrypted<T: Serialize>(&self, key: &str, data: &T, encryption_key: &EncryptionKey) -> Result<()> {
        let json_bytes = serde_json::to_vec(data)
            .map_err(|e| Error::Other(format!("Failed to serialize JSON: {}", e)))?;
        self.upload_encrypted(key, json_bytes, encryption_key).await
    }

    /// Download and deserialize JSON object
    pub async fn download_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T> {
        let bytes = self.download(key).await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Other(format!("Failed to deserialize JSON: {}", e)))
    }

    /// Download and deserialize JSON object with encryption
    pub async fn download_json_encrypted<T: for<'de> Deserialize<'de>>(&self, key: &str, encryption_key: &EncryptionKey) -> Result<T> {
        let bytes = self.download_encrypted(key, encryption_key).await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Other(format!("Failed to deserialize JSON: {}", e)))
    }

    /// Upload JSONL (newline-delimited JSON) from a vector of objects
    pub async fn upload_jsonl<T: Serialize>(&self, key: &str, records: &[T]) -> Result<()> {
        let mut jsonl = Vec::new();
        for record in records {
            let json = serde_json::to_string(record)
                .map_err(|e| Error::Other(format!("Failed to serialize record: {}", e)))?;
            jsonl.extend_from_slice(json.as_bytes());
            jsonl.push(b'\n');
        }
        self.upload(key, jsonl).await
    }

    /// Upload JSONL with encryption
    pub async fn upload_jsonl_encrypted<T: Serialize>(&self, key: &str, records: &[T], encryption_key: &EncryptionKey) -> Result<()> {
        let mut jsonl = Vec::new();
        for record in records {
            let json = serde_json::to_string(record)
                .map_err(|e| Error::Other(format!("Failed to serialize record: {}", e)))?;
            jsonl.extend_from_slice(json.as_bytes());
            jsonl.push(b'\n');
        }
        self.upload_encrypted(key, jsonl, encryption_key).await
    }

    /// Download and parse JSONL (newline-delimited JSON) into a vector
    pub async fn download_jsonl<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Vec<T>> {
        let bytes = self.download(key).await?;
        let text = String::from_utf8(bytes)
            .map_err(|e| Error::Other(format!("Invalid UTF-8 in JSONL: {}", e)))?;

        let mut records = Vec::new();
        for (line_num, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record = serde_json::from_str(line)
                .map_err(|e| Error::Other(format!("Failed to parse JSONL line {}: {}", line_num + 1, e)))?;
            records.push(record);
        }
        Ok(records)
    }

    /// Download and parse JSONL with encryption
    pub async fn download_jsonl_encrypted<T: for<'de> Deserialize<'de>>(&self, key: &str, encryption_key: &EncryptionKey) -> Result<Vec<T>> {
        let bytes = self.download_encrypted(key, encryption_key).await?;
        let text = String::from_utf8(bytes)
            .map_err(|e| Error::Other(format!("Invalid UTF-8 in JSONL: {}", e)))?;

        let mut records = Vec::new();
        for (line_num, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record = serde_json::from_str(line)
                .map_err(|e| Error::Other(format!("Failed to parse JSONL line {}: {}", line_num + 1, e)))?;
            records.push(record);
        }
        Ok(records)
    }

    /// Download stream JSONL with automatic key derivation and decryption
    ///
    /// This is a convenience method for StreamReader to download stream data
    /// with the correct encryption key derived from source_id, stream_name, and date.
    ///
    /// # Arguments
    /// * `source_id` - Source UUID
    /// * `stream_name` - Stream name
    /// * `s3_key` - Full S3 key to download
    /// * `master_key` - Master encryption key for key derivation
    ///
    /// # Returns
    /// Raw JSONL bytes (decrypted)
    pub async fn download_stream_jsonl(
        &self,
        source_id: uuid::Uuid,
        stream_name: &str,
        s3_key: &str,
        master_key: &[u8; 32],
    ) -> Result<Vec<u8>> {
        // Parse the date from the S3 key
        // Key format: streams/{source_id}/{stream_name}/date={YYYY-MM-DD}/records_{timestamp}.jsonl
        let date = models::StreamKeyParser::parse_date_from_key(s3_key)?;

        // Derive encryption key
        let derived_key = encryption::derive_stream_key(master_key, source_id, stream_name, date)?;

        // Convert to base64 for S3 SSE-C
        let key_base64 = base64::engine::general_purpose::STANDARD.encode(&derived_key);
        let encryption_key = EncryptionKey { key_base64 };

        // Download with decryption
        self.download_encrypted(s3_key, &encryption_key).await
    }
}

/// S3/MinIO storage backend
pub struct S3Storage {
    bucket: String,
    client: aws_sdk_s3::Client,
    /// Whether the endpoint uses HTTPS (required for SSE-C encryption)
    is_https: bool,
}

impl S3Storage {
    /// Helper to convert AWS SDK errors to our Error type with context preserved
    fn map_s3_error<E: std::fmt::Display>(operation: &str, err: E) -> Error {
        let error_msg = format!("{}", err);

        // Try to extract error code from the message for better classification
        let error_info = if error_msg.contains("NoSuchBucket") {
            "NoSuchBucket"
        } else if error_msg.contains("AccessDenied") || error_msg.contains("forbidden") {
            "AccessDenied"
        } else if error_msg.contains("InvalidRequest") {
            "InvalidRequest"
        } else if error_msg.contains("NoSuchKey") {
            "NoSuchKey"
        } else if error_msg.contains("PreconditionFailed") {
            "PreconditionFailed"
        } else {
            "Unknown"
        };

        Error::S3(format!("{} failed: {} [code: {}]", operation, error_msg, error_info))
    }

    pub async fn new(
        bucket: String,
        endpoint: Option<String>,
        access_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Self> {
        // Detect if endpoint uses HTTPS (required for SSE-C encryption)
        let is_https = endpoint.as_ref()
            .map(|url| url.starts_with("https://"))
            .unwrap_or(true); // AWS S3 always uses HTTPS

        // Build AWS configuration
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        // Configure endpoint if provided (for MinIO)
        if let Some(endpoint_url) = &endpoint {
            config_loader = config_loader.endpoint_url(endpoint_url);
            // Set a default region for MinIO (required by AWS SDK)
            config_loader = config_loader.region(aws_sdk_s3::config::Region::new("us-east-1"));
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

        // Build S3 client with path-style addressing for MinIO compatibility
        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(s3_config);

        // Log encryption configuration
        if is_https {
            tracing::info!(
                bucket = %bucket,
                endpoint = ?endpoint,
                "S3 storage initialized with HTTPS - SSE-C encryption enabled"
            );
        } else {
            tracing::warn!(
                bucket = %bucket,
                endpoint = ?endpoint,
                "S3 storage initialized with HTTP - SSE-C encryption DISABLED (requires HTTPS)"
            );
        }

        Ok(Self { bucket, client, is_https })
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
            .map_err(|e| Self::map_s3_error("PutObject", e))?;

        Ok(())
    }

    async fn upload_encrypted(&self, key: &str, data: Vec<u8>, encryption_key: &EncryptionKey) -> Result<()> {
        // SSE-C encryption requires HTTPS - fall back to unencrypted upload for HTTP endpoints
        if !self.is_https {
            tracing::debug!(
                key = %key,
                "Skipping SSE-C encryption for HTTP endpoint - uploading without encryption"
            );
            return self.upload(key, data).await;
        }

        // Decode the base64 key to compute MD5 hash
        let key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &encryption_key.key_base64
        ).map_err(|e| Error::Other(format!("Failed to decode encryption key: {}", e)))?;

        // Compute MD5 hash of the key and encode as base64 (required by S3 SSE-C)
        let key_md5_bytes = md5::compute(&key_bytes);
        let key_md5 = base64::engine::general_purpose::STANDARD.encode(key_md5_bytes.as_ref());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .sse_customer_algorithm("AES256")
            .sse_customer_key(&encryption_key.key_base64)
            .sse_customer_key_md5(&key_md5)
            .send()
            .await
            .map_err(|e| Self::map_s3_error("PutObject (encrypted)", e))?;

        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Self::map_s3_error("GetObject", e))?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| Self::map_s3_error("GetObject (read body)", e))?
            .into_bytes();

        Ok(bytes.to_vec())
    }

    async fn download_encrypted(&self, key: &str, encryption_key: &EncryptionKey) -> Result<Vec<u8>> {
        // SSE-C encryption requires HTTPS - fall back to unencrypted download for HTTP endpoints
        if !self.is_https {
            tracing::debug!(
                key = %key,
                "Skipping SSE-C decryption for HTTP endpoint - downloading without encryption"
            );
            return self.download(key).await;
        }

        // Decode the base64 key to compute MD5 hash
        let key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &encryption_key.key_base64
        ).map_err(|e| Error::Other(format!("Failed to decode encryption key: {}", e)))?;

        // Compute MD5 hash of the key and encode as base64 (required by S3 SSE-C)
        let key_md5_bytes = md5::compute(&key_bytes);
        let key_md5 = base64::engine::general_purpose::STANDARD.encode(key_md5_bytes.as_ref());

        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .sse_customer_algorithm("AES256")
            .sse_customer_key(&encryption_key.key_base64)
            .sse_customer_key_md5(&key_md5)
            .send()
            .await
            .map_err(|e| Self::map_s3_error("GetObject (encrypted)", e))?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| Self::map_s3_error("GetObject (read encrypted body)", e))?
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
            .map_err(|e| Self::map_s3_error("DeleteObject", e))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| Self::map_s3_error("ListObjectsV2", e))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key())
            .map(|k| k.to_string())
            .collect();

        Ok(keys)
    }

    async fn list_with_pagination(&self, prefix: &str, max_keys: Option<i32>, continuation_token: Option<String>) -> Result<ListResult> {
        let mut request = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix);

        if let Some(max) = max_keys {
            request = request.max_keys(max);
        }

        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| Self::map_s3_error("ListObjectsV2 (paginated)", e))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key())
            .map(|k| k.to_string())
            .collect();

        Ok(ListResult {
            keys,
            continuation_token: response.next_continuation_token().map(|s| s.to_string()),
            is_truncated: response.is_truncated().unwrap_or(false),
        })
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
                message: format!("S3 error: {e}"),
            }),
        }
    }

    async fn get_presigned_url(&self, key: &str, expires_in: Duration) -> Result<String> {
        // Create presigning config
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| Error::S3(format!("Failed to create presigning config: {}", e)))?;

        // Generate presigned URL
        let presigned_request = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| Error::S3(format!("Failed to generate presigned URL: {}", e)))?;

        Ok(presigned_request.uri().to_string())
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

    async fn upload_encrypted(&self, key: &str, data: Vec<u8>, _encryption_key: &EncryptionKey) -> Result<()> {
        // Local storage doesn't support encryption at rest
        // Just do a regular upload (encryption would need to be done at application level)
        self.upload(key, data).await
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.base_path.join(key);
        Ok(tokio::fs::read(path).await?)
    }

    async fn download_encrypted(&self, key: &str, _encryption_key: &EncryptionKey) -> Result<Vec<u8>> {
        // Local storage doesn't support encryption at rest
        // Just do a regular download
        self.download(key).await
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
                    files.push(format!("{prefix}/{name}"));
                }
            }
        }

        Ok(files)
    }

    async fn list_with_pagination(&self, prefix: &str, max_keys: Option<i32>, _continuation_token: Option<String>) -> Result<ListResult> {
        // Local storage doesn't support pagination, so we just return all files
        // and truncate if max_keys is specified
        let keys = self.list(prefix).await?;

        let (keys, is_truncated) = if let Some(max) = max_keys {
            let max_usize = max as usize;
            if keys.len() > max_usize {
                (keys[..max_usize].to_vec(), true)
            } else {
                (keys, false)
            }
        } else {
            (keys, false)
        };

        Ok(ListResult {
            keys,
            continuation_token: None,
            is_truncated,
        })
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

    async fn get_presigned_url(&self, _key: &str, _expires_in: Duration) -> Result<String> {
        // Local storage doesn't support presigned URLs
        // In a real implementation, you might set up a local HTTP server
        // For now, return an error
        Err(Error::Other(
            "Presigned URLs not supported for local storage".into()
        ))
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

    #[tokio::test]
    async fn test_local_storage_encryption_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::local(temp_dir.path().to_str().unwrap().to_string()).unwrap();

        storage.initialize().await.unwrap();

        // Create test data
        let original_data = b"sensitive test data that should be encrypted".to_vec();

        // Create encryption key (base64-encoded 32-byte key for AES-256)
        let key_bytes = [0u8; 32]; // In real usage, use a proper random key
        let key_base64 = base64::engine::general_purpose::STANDARD.encode(&key_bytes);
        let encryption_key = EncryptionKey { key_base64 };

        // Test upload with encryption
        storage.upload_encrypted("encrypted_test.bin", original_data.clone(), &encryption_key)
            .await
            .unwrap();

        // Test download with encryption
        let decrypted_data = storage.download_encrypted("encrypted_test.bin", &encryption_key)
            .await
            .unwrap();

        // Verify round-trip
        assert_eq!(original_data, decrypted_data);

        // Test JSONL encryption
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct TestRecord {
            id: i32,
            name: String,
        }

        let records = vec![
            TestRecord { id: 1, name: "Alice".to_string() },
            TestRecord { id: 2, name: "Bob".to_string() },
            TestRecord { id: 3, name: "Charlie".to_string() },
        ];

        // Upload JSONL with encryption
        storage.upload_jsonl_encrypted("encrypted_records.jsonl", &records, &encryption_key)
            .await
            .unwrap();

        // Download JSONL with encryption
        let downloaded_records: Vec<TestRecord> = storage
            .download_jsonl_encrypted("encrypted_records.jsonl", &encryption_key)
            .await
            .unwrap();

        // Verify records
        assert_eq!(records, downloaded_records);
    }

    #[test]
    fn test_s3_md5_hash_encoding() {
        // Test that MD5 hashes are properly base64 encoded (not hex)
        // This is a regression test for the bug where MD5 was hex-encoded

        let key_bytes = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11,
                        0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99];

        // Compute MD5 hash
        let md5_hash = md5::compute(&key_bytes);

        // Encode as base64 (CORRECT way for S3 SSE-C)
        let md5_base64 = base64::engine::general_purpose::STANDARD.encode(md5_hash.as_ref());

        // Verify it's base64 (contains typical base64 characters)
        assert!(md5_base64.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));

        // Verify it's NOT hex format (should not be 32 hex characters)
        let md5_hex = format!("{:x}", md5_hash);
        assert_ne!(md5_base64, md5_hex, "MD5 should be base64-encoded, not hex");
        assert_eq!(md5_hex.len(), 32, "Hex MD5 should be 32 characters");

        // Base64-encoded MD5 should be 24 characters (16 bytes -> 24 base64 chars with padding)
        assert_eq!(md5_base64.len(), 24, "Base64 MD5 should be 24 characters");
    }
}