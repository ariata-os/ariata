//! Device pairing API - Secure device registration and authentication

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};
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
        INSERT INTO sources (type, name, auth_type, pairing_code, pairing_status, code_expires_at, is_active)
        VALUES ($1, $2, 'device', $3, 'pending', $4, false)
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
        SELECT id, type, code_expires_at
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
    .bind(&device_token)
    .bind(source.id)
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to complete pairing: {e}")))?;

    // Get available streams for this device type
    let available_streams = crate::registry::get_source(&source.r#type)
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
            type as device_type,
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
    let source_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM sources
        WHERE device_token = $1 AND pairing_status = 'active' AND is_active = true
        "#,
    )
    .bind(token)
    .fetch_optional(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to validate device token: {e}")))?
    .ok_or_else(|| Error::Unauthorized("Invalid or revoked device token".to_string()))?;

    Ok(source_id)
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
