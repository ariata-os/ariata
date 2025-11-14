//! Encryption utilities for stream data in object storage
//!
//! This module provides SSE-C (Server-Side Encryption with Customer-provided Keys)
//! support for S3/MinIO. Keys are derived deterministically from a master secret
//! and stream metadata, ensuring each source/stream/date combination has a unique key.

use chrono::NaiveDate;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::error::{Error, Result};

type HmacSha256 = Hmac<Sha256>;

/// Derives a 32-byte encryption key for a specific stream object
///
/// The key is derived using HMAC-SHA256 with the following input:
/// `{source_id}:{stream_name}:{date}`
///
/// This ensures:
/// - Each source has unique keys (multi-tenant isolation)
/// - Each stream type has unique keys (reduces blast radius)
/// - Each date has unique keys (temporal isolation)
/// - Keys are deterministic (can re-derive for reading)
///
/// # Arguments
/// * `master_key` - Master encryption key from environment (STREAM_ENCRYPTION_MASTER_KEY)
/// * `source_id` - UUID of the source (user/device)
/// * `stream_name` - Name of the stream (e.g., "healthkit", "location")
/// * `date` - Date of the stream data
///
/// # Returns
/// A 32-byte key suitable for AES-256 encryption
pub fn derive_stream_key(
    master_key: &[u8],
    source_id: Uuid,
    stream_name: &str,
    date: NaiveDate,
) -> Result<[u8; 32]> {
    if master_key.len() != 32 {
        return Err(Error::Other(format!(
            "Master key must be 32 bytes, got {}",
            master_key.len()
        )));
    }

    // Create input: source_id:stream_name:date
    let input = format!("{}:{}:{}", source_id, stream_name, date);

    // Compute HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(master_key)
        .map_err(|e| Error::Other(format!("Failed to create HMAC: {}", e)))?;
    mac.update(input.as_bytes());

    let result = mac.finalize();
    let key_bytes = result.into_bytes();

    // Convert to [u8; 32]
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes[..32]);

    Ok(key)
}

/// Encodes a 32-byte key to base64 for AWS SSE-C headers
pub fn encode_key_base64(key: &[u8; 32]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(key)
}

/// Parses a hex-encoded master key from environment variable
///
/// Expected format: 64 hex characters (32 bytes)
/// Generate with: `openssl rand -hex 32`
pub fn parse_master_key_hex(hex_str: &str) -> Result<[u8; 32]> {
    let hex_str = hex_str.trim();

    if hex_str.len() != 64 {
        return Err(Error::Other(format!(
            "Master key hex must be 64 characters (32 bytes), got {} characters. Generate with: openssl rand -hex 32",
            hex_str.len()
        )));
    }

    let bytes =
        hex::decode(hex_str).map_err(|e| Error::Other(format!("Invalid hex string: {}", e)))?;

    if bytes.len() != 32 {
        return Err(Error::Other(format!(
            "Decoded master key must be 32 bytes, got {}",
            bytes.len()
        )));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_derive_stream_key() {
        let master_key = [0u8; 32];
        let source_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let stream_name = "healthkit";
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let key = derive_stream_key(&master_key, source_id, stream_name, date).unwrap();

        // Key should be deterministic
        let key2 = derive_stream_key(&master_key, source_id, stream_name, date).unwrap();
        assert_eq!(key, key2);

        // Different dates should produce different keys
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();
        let key3 = derive_stream_key(&master_key, source_id, stream_name, date2).unwrap();
        assert_ne!(key, key3);

        // Different streams should produce different keys
        let key4 = derive_stream_key(&master_key, source_id, "location", date).unwrap();
        assert_ne!(key, key4);
    }

    #[test]
    fn test_parse_master_key_hex() {
        // Valid 64-character hex string
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let key = parse_master_key_hex(hex).unwrap();
        assert_eq!(key.len(), 32);

        // Invalid length
        let result = parse_master_key_hex("abc");
        assert!(result.is_err());

        // Invalid hex characters
        let result = parse_master_key_hex(
            "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_key_base64() {
        let key = [0u8; 32];
        let encoded = encode_key_base64(&key);
        assert!(!encoded.is_empty());

        // Should be base64 encoded
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let decoded = STANDARD.decode(&encoded).unwrap();
        assert_eq!(decoded.len(), 32);
    }
}
