//! Input validation for API parameters

use crate::error::{Error, Result};
use uuid::Uuid;

/// Validate provider name exists in registry
pub fn validate_provider_name(provider: &str) -> Result<()> {
    if provider.is_empty() {
        return Err(Error::InvalidInput(
            "Provider name cannot be empty".to_string(),
        ));
    }

    // Check if provider exists in registry
    crate::registry::get_source(provider)
        .ok_or_else(|| Error::InvalidInput(format!("Unknown provider: {provider}")))?;

    Ok(())
}

/// Validate redirect URI against allowed hosts
pub fn validate_redirect_uri(uri: &str, allowed_hosts: &[&str]) -> Result<()> {
    if uri.is_empty() {
        return Err(Error::InvalidInput(
            "Redirect URI cannot be empty".to_string(),
        ));
    }

    // Parse URI
    let parsed = uri
        .parse::<url::Url>()
        .map_err(|e| Error::InvalidInput(format!("Invalid redirect URI: {e}")))?;

    // Ensure HTTPS (except localhost for dev)
    if parsed.scheme() != "https" && parsed.host_str() != Some("localhost") {
        return Err(Error::InvalidInput(
            "Redirect URI must use HTTPS (except localhost)".to_string(),
        ));
    }

    // Check against whitelist
    if let Some(host) = parsed.host_str() {
        if !allowed_hosts.iter().any(|allowed| host.ends_with(allowed)) {
            return Err(Error::InvalidInput(format!(
                "Redirect URI host not allowed: {host}"
            )));
        }
    }

    Ok(())
}

/// Validate UUID string
pub fn validate_uuid(id: &str) -> Result<Uuid> {
    id.parse::<Uuid>()
        .map_err(|e| Error::InvalidInput(format!("Invalid UUID: {e}")))
}

/// Validate source/stream name (alphanumeric + underscore/dash)
pub fn validate_name(name: &str, field: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidInput(format!("{field} cannot be empty")));
    }

    if name.len() > 255 {
        return Err(Error::InvalidInput(format!(
            "{field} too long (max 255 chars)"
        )));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ' ')
    {
        return Err(Error::InvalidInput(format!(
            "{field} contains invalid characters (use alphanumeric, _, -, ., space)"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_provider() {
        assert!(validate_provider_name("google").is_ok());
        assert!(validate_provider_name("notion").is_ok());
        assert!(validate_provider_name("invalid-provider").is_err());
        assert!(validate_provider_name("").is_err());
    }

    #[test]
    fn test_validate_redirect_uri() {
        let allowed = &["auth.ariata.com", "localhost"];

        assert!(validate_redirect_uri("https://auth.ariata.com/callback", allowed).is_ok());
        assert!(validate_redirect_uri("http://localhost:3000/callback", allowed).is_ok());
        assert!(validate_redirect_uri("http://evil.com/callback", allowed).is_err());
        assert!(validate_redirect_uri("not-a-url", allowed).is_err());
    }

    #[test]
    fn test_validate_uuid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("invalid").is_err());
    }

    #[test]
    fn test_validate_name() {
        assert!(validate_name("My Device", "Device name").is_ok());
        assert!(validate_name("device_123", "Device name").is_ok());
        assert!(validate_name("", "Device name").is_err());
        assert!(validate_name(&"a".repeat(300), "Device name").is_err());
    }
}
