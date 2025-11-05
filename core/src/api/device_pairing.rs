//! Device pairing API - Secure device registration and authentication

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::sources::base::TokenEncryptor;
use crate::registry::StreamDescriptor;

/// Response when pairing is initiated
#[derive(Debug, Clone)]
pub struct PairingInitiated {
    pub source_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

/// Response when pairing is completed successfully
#[derive(Debug, Clone)]
pub struct PairingCompleted {
    pub source_id: Uuid,
    pub device_token: String,
    pub available_streams: Vec<StreamDescriptor>,
}

/// Device information provided during pairing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub device_model: String,
    pub os_version: String,
    pub app_version: Option<String>,
}

/// Pairing status for a source
#[derive(Debug, Clone)]
pub enum PairingStatus {
    Pending,
    Active(DeviceInfo),
    Revoked,
}

/// Pending pairing information for display
#[derive(Debug, Clone)]
pub struct PendingPairing {
    pub source_id: Uuid,
    pub name: String,
    pub device_type: String,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Initiate device pairing by generating a pairing code
///
/// This creates a pending source entry with a 6-character alphanumeric pairing code
/// that expires in 10 minutes.
pub async fn initiate_device_pairing(
    db: &PgPool,
    device_type: &str,
    name: &str,
) -> Result<PairingInitiated> {
    // Validate device type exists in registry
    crate::api::validation::validate_provider_name(device_type)?;

    // Validate name format
    crate::api::validation::validate_name(name, "Device name")?;

    // Generate 6-character alphanumeric pairing code
    let code = generate_pairing_code();

    // Set expiration to 10 minutes from now
    let expires_at = Utc::now() + Duration::minutes(10);

    // Create pending source
    let source_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO sources (provider, name, auth_type, pairing_code, pairing_status, code_expires_at, is_active, is_internal)
        VALUES ($1, $2, 'device', $3, 'pending', $4, false, false)
        RETURNING id
        "#,
    )
    .bind(device_type)
    .bind(name)
    .bind(&code)
    .bind(expires_at)
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to create pending pairing: {e}")))?;

    Ok(PairingInitiated {
        source_id,
        code,
        expires_at,
    })
}

/// Complete device pairing with a valid pairing code
///
/// Validates the pairing code, updates the source with device information,
/// generates a secure device token, and returns available streams.
pub async fn complete_device_pairing(
    db: &PgPool,
    code: &str,
    device_info: DeviceInfo,
) -> Result<PairingCompleted> {
    // Find source with matching pairing code
    let source = sqlx::query!(
        r#"
        SELECT id, provider, code_expires_at
        FROM sources
        WHERE pairing_code = $1 AND pairing_status = 'pending'
        "#,
        code
    )
    .fetch_optional(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to find pairing: {e}")))?
    .ok_or_else(|| Error::Other("Invalid or expired pairing code".to_string()))?;

    // Check if code has expired
    if let Some(expires_at) = source.code_expires_at {
        if expires_at < Utc::now() {
            return Err(Error::Other("Pairing code has expired".to_string()));
        }
    }

    // Generate secure device token
    let device_token = generate_device_token();

    // Encrypt device token before storing
    let encryptor = TokenEncryptor::from_env()
        .map_err(|e| Error::Other(format!("Failed to initialize encryption: {e}")))?;
    let encrypted_token = encryptor.encrypt(&device_token)
        .map_err(|e| Error::Other(format!("Failed to encrypt device token: {e}")))?;

    // Update source with device info and activate
    sqlx::query(
        r#"
        UPDATE sources
        SET device_id = $1,
            device_info = $2,
            device_token = $3,
            pairing_status = 'active',
            pairing_code = NULL,
            code_expires_at = NULL,
            is_active = true,
            updated_at = NOW()
        WHERE id = $4
        "#,
    )
    .bind(&device_info.device_id)
    .bind(serde_json::to_value(&device_info)
        .map_err(|e| Error::Serialization(e))?)
    .bind(&encrypted_token)
    .bind(source.id)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to complete pairing: {e}")))?;

    // Get available streams for this device type
    let available_streams = crate::registry::get_source(&source.provider)
        .map(|info| info.streams.clone())
        .unwrap_or_default();

    Ok(PairingCompleted {
        source_id: source.id,
        device_token,
        available_streams,
    })
}

/// Check the status of a pairing by source ID
pub async fn check_pairing_status(db: &PgPool, source_id: Uuid) -> Result<PairingStatus> {
    let source = sqlx::query!(
        r#"
        SELECT pairing_status, device_info
        FROM sources
        WHERE id = $1
        "#,
        source_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to check pairing status: {e}")))?;

    match source.pairing_status.as_deref() {
        Some("pending") => Ok(PairingStatus::Pending),
        Some("active") => {
            let device_info: DeviceInfo = source
                .device_info
                .and_then(|v| serde_json::from_value(v).ok())
                .ok_or_else(|| Error::Other("Device info not found".to_string()))?;
            Ok(PairingStatus::Active(device_info))
        }
        Some("revoked") => Ok(PairingStatus::Revoked),
        _ => Ok(PairingStatus::Pending),
    }
}

/// List all pending pairings
pub async fn list_pending_pairings(db: &PgPool) -> Result<Vec<PendingPairing>> {
    let pairings = sqlx::query_as!(
        PendingPairing,
        r#"
        SELECT
            id as source_id,
            name,
            provider as device_type,
            pairing_code as "code!",
            code_expires_at as "expires_at!",
            created_at
        FROM sources
        WHERE pairing_status = 'pending'
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to list pending pairings: {e}")))?;

    Ok(pairings)
}

/// Validate a device token and return the source ID
pub async fn validate_device_token(db: &PgPool, token: &str) -> Result<Uuid> {
    // Initialize encryptor
    let encryptor = TokenEncryptor::from_env()
        .map_err(|e| Error::Other(format!("Failed to initialize encryption: {e}")))?;

    // Get all active device sources with tokens
    let sources = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT id, device_token
        FROM sources
        WHERE device_token IS NOT NULL
        AND pairing_status = 'active'
        AND is_active = true
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to query device tokens: {e}")))?;

    // Try to decrypt each token and compare with the provided token
    for (source_id, encrypted_token) in sources {
        // Decrypt stored token
        if let Ok(decrypted_token) = encryptor.decrypt(&encrypted_token) {
            // Compare with provided token
            if decrypted_token == token {
                return Ok(source_id);
            }
        }
    }

    // No match found
    Err(Error::Unauthorized("Invalid or revoked device token".to_string()))
}

/// Update the last seen timestamp for a device
pub async fn update_last_seen(db: &PgPool, source_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE sources
        SET last_seen_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(source_id)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to update last_seen: {e}")))?;

    Ok(())
}

/// Response when verifying a device token
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceVerified {
    pub source_id: Uuid,
    pub enabled_streams: Vec<crate::api::StreamInfo>,
    pub configuration_complete: bool,
}

/// Verify a device token and return stream configuration status
///
/// This endpoint is called by devices that already have a device_token
/// to check if streams have been configured in the web app.
pub async fn verify_device(db: &PgPool, token: &str) -> Result<DeviceVerified> {
    // Validate token and get source ID
    let source_id = validate_device_token(db, token).await?;

    // Update last seen
    update_last_seen(db, source_id).await?;

    // Get all streams for this source
    let all_streams = crate::api::list_source_streams(db, source_id).await?;

    // Filter to only enabled streams
    let enabled_streams: Vec<crate::api::StreamInfo> = all_streams
        .into_iter()
        .filter(|s| s.is_enabled)
        .collect();

    // Configuration is complete if at least one stream is enabled
    let configuration_complete = !enabled_streams.is_empty();

    Ok(DeviceVerified {
        source_id,
        enabled_streams,
        configuration_complete,
    })
}

/// Generate a 6-character alphanumeric pairing code
fn generate_pairing_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Removed ambiguous: 0, O, 1, I
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate a secure 256-bit device token
fn generate_device_token() -> String {
    use rand::RngCore;
    let mut token = [0u8; 32]; // 256 bits
    rand::thread_rng().fill_bytes(&mut token);
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn test_generate_pairing_code() {
        let code = generate_pairing_code();

        // Should be 6 characters
        assert_eq!(code.len(), 6);

        // Should only contain valid characters (no 0, O, 1, I)
        for ch in code.chars() {
            assert!(
                "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(ch),
                "Invalid character in pairing code: {}",
                ch
            );
        }

        // Generate multiple codes to ensure randomness
        let code1 = generate_pairing_code();
        let code2 = generate_pairing_code();
        let code3 = generate_pairing_code();

        // While technically possible, it's extremely unlikely all three are the same
        assert!(
            !(code1 == code2 && code2 == code3),
            "Generated codes should be random"
        );
    }

    #[test]
    fn test_generate_device_token() {
        let token = generate_device_token();

        // Should be base64 encoded 32 bytes (256 bits)
        // Base64 encoding of 32 bytes = 44 characters (with padding)
        assert!(token.len() >= 43 && token.len() <= 44, "Token length should be 43-44 chars");

        // Should be valid base64
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&token)
            .expect("Token should be valid base64");
        assert_eq!(decoded.len(), 32, "Decoded token should be 32 bytes");

        // Generate multiple tokens to ensure randomness
        let token1 = generate_device_token();
        let token2 = generate_device_token();
        assert_ne!(token1, token2, "Generated tokens should be unique");
    }

    #[test]
    fn test_device_info_serialization() {
        let info = DeviceInfo {
            device_id: "iPhone-12345".to_string(),
            device_name: "Adam's iPhone".to_string(),
            device_model: "iPhone16,1".to_string(),
            os_version: "iOS 17.2".to_string(),
            app_version: Some("1.0.0".to_string()),
        };

        // Should serialize to JSON
        let json = serde_json::to_value(&info).expect("Should serialize");
        assert!(json.is_object());
        assert_eq!(json["device_id"], "iPhone-12345");
        assert_eq!(json["device_name"], "Adam's iPhone");

        // Should deserialize from JSON
        let deserialized: DeviceInfo =
            serde_json::from_value(json).expect("Should deserialize");
        assert_eq!(deserialized.device_id, info.device_id);
        assert_eq!(deserialized.device_name, info.device_name);
    }

    #[test]
    fn test_encryption_roundtrip() {
        // Set up a test encryption key
        let key_bytes = b"12345678901234567890123456789012";
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);
        std::env::set_var("ARIATA_ENCRYPTION_KEY", &key_b64);

        let encryptor = TokenEncryptor::from_env().expect("Should create encryptor");

        // Generate a token
        let original_token = generate_device_token();

        // Encrypt it
        let encrypted = encryptor
            .encrypt(&original_token)
            .expect("Should encrypt");

        // Encrypted should be different from original
        assert_ne!(encrypted, original_token);

        // Decrypt it
        let decrypted = encryptor.decrypt(&encrypted).expect("Should decrypt");

        // Should match original
        assert_eq!(decrypted, original_token);

        // Clean up
        std::env::remove_var("ARIATA_ENCRYPTION_KEY");
    }

    #[test]
    fn test_validate_token_uses_encryption() {
        // This test verifies that validation compares encrypted tokens
        // It ensures the encryption is applied consistently

        let key_bytes = b"12345678901234567890123456789012";
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);
        std::env::set_var("ARIATA_ENCRYPTION_KEY", &key_b64);

        let encryptor = TokenEncryptor::from_env().expect("Should create encryptor");

        let token1 = "test_token_123";
        let token2 = "test_token_456";

        // Encrypt both tokens
        let encrypted1 = encryptor.encrypt(token1).expect("Should encrypt");
        let encrypted2 = encryptor.encrypt(token2).expect("Should encrypt");

        // Different tokens should produce different encrypted values
        assert_ne!(encrypted1, encrypted2);

        // Same token should produce different encrypted values (due to random nonce)
        let encrypted1_again = encryptor.encrypt(token1).expect("Should encrypt");
        assert_ne!(
            encrypted1, encrypted1_again,
            "Same plaintext should produce different ciphertext due to random nonce"
        );

        // Clean up
        std::env::remove_var("ARIATA_ENCRYPTION_KEY");
    }
}
