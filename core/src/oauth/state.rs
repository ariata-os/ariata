//! OAuth state parameter generation and validation using HMAC signatures
//!
//! Protects against CSRF attacks during OAuth flows by generating tamper-proof
//! state tokens that encode timestamp and optional session data.

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{Error, Result};

type HmacSha256 = Hmac<Sha256>;

/// Duration for which state tokens are valid (10 minutes)
const STATE_VALIDITY_MINUTES: i64 = 10;

/// Generate a signed OAuth state token
///
/// Format: base64(timestamp || hmac(timestamp || session_data))
///
/// # Arguments
/// * `session_data` - Optional session identifier or user context
pub fn generate_state(session_data: Option<&str>) -> Result<String> {
    let secret = get_signing_secret()?;
    let timestamp = Utc::now().timestamp();

    // Build payload: timestamp || session_data
    let payload = match session_data {
        Some(data) => format!("{}:{}", timestamp, data),
        None => timestamp.to_string(),
    };

    // Sign the payload
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| Error::Configuration("Invalid signing secret".to_string()))?;
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();

    // Combine payload + signature and encode
    let mut combined = payload.as_bytes().to_vec();
    combined.extend_from_slice(&signature);

    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&combined))
}

/// Validate a signed OAuth state token
///
/// Returns Ok(()) if valid, Err if expired or tampered
pub fn validate_state(state_token: &str) -> Result<()> {
    let secret = get_signing_secret()?;

    // Decode base64
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(state_token)
        .map_err(|_| Error::InvalidInput("Invalid state token format".to_string()))?;

    // Split payload and signature (last 32 bytes are HMAC-SHA256 signature)
    if decoded.len() < 32 {
        return Err(Error::InvalidInput("State token too short".to_string()));
    }

    let (payload_bytes, signature) = decoded.split_at(decoded.len() - 32);
    let payload = String::from_utf8(payload_bytes.to_vec())
        .map_err(|_| Error::InvalidInput("Invalid state payload".to_string()))?;

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| Error::Configuration("Invalid signing secret".to_string()))?;
    mac.update(payload_bytes);

    mac.verify_slice(signature)
        .map_err(|_| Error::InvalidInput("State token signature invalid - possible tampering".to_string()))?;

    // Extract timestamp from payload (format: "timestamp" or "timestamp:session_data")
    let timestamp_str = payload.split(':').next()
        .ok_or_else(|| Error::InvalidInput("Invalid state payload format".to_string()))?;

    let timestamp = timestamp_str.parse::<i64>()
        .map_err(|_| Error::InvalidInput("Invalid timestamp in state".to_string()))?;

    // Check expiration (10 minutes)
    let issued_at = DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| Error::InvalidInput("Invalid timestamp value".to_string()))?;

    let expires_at = issued_at + Duration::minutes(STATE_VALIDITY_MINUTES);

    if Utc::now() > expires_at {
        return Err(Error::InvalidInput("State token expired".to_string()));
    }

    Ok(())
}

/// Get the signing secret from environment
fn get_signing_secret() -> Result<String> {
    // Reuse encryption key as signing secret for simplicity
    // In production, could use separate OAUTH_STATE_SECRET
    std::env::var("ARIATA_ENCRYPTION_KEY")
        .map_err(|_| Error::Configuration(
            "ARIATA_ENCRYPTION_KEY required for OAuth state signing".to_string()
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use serial_test::serial;

    fn setup_test_key() {
        env::set_var("ARIATA_ENCRYPTION_KEY", "dGVzdC1zZWNyZXQtZm9yLWhtYWMtc2lnbmluZw==");
    }

    #[test]
    #[serial]
    fn test_generate_and_validate_state() {
        setup_test_key();

        let state = generate_state(None).unwrap();
        let result = validate_state(&state);
        if let Err(ref e) = result {
            eprintln!("Validation error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_validate_with_session_data() {
        setup_test_key();

        let state = generate_state(Some("user123")).unwrap();
        let result = validate_state(&state);
        if let Err(ref e) = result {
            eprintln!("Validation error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_reject_tampered_state() {
        setup_test_key();

        let state = generate_state(None).unwrap();
        let mut tampered = state.clone();
        tampered.push('X'); // Tamper with token

        assert!(validate_state(&tampered).is_err());
    }

    #[test]
    #[serial]
    fn test_reject_invalid_signature() {
        setup_test_key();

        let state = generate_state(None).unwrap();

        // Change secret and try to validate
        env::set_var("ARIATA_ENCRYPTION_KEY", "ZGlmZmVyZW50LXNlY3JldC1rZXk=");
        assert!(validate_state(&state).is_err());

        // Restore original key for other tests
        setup_test_key();
    }
}
