//! Data quality validation for ingested records
//!
//! Provides simple, obvious validation checks before database writes.
//! Catches common data quality issues early with clear error messages.

use crate::error::{Error, Result};

/// Validation result with detailed error messages
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Convert to Result type
    pub fn into_result(self) -> Result<()> {
        if self.is_valid {
            Ok(())
        } else {
            Err(Error::Other(format!(
                "Validation failed: {}",
                self.errors.join("; ")
            )))
        }
    }
}

/// Validate latitude is within valid range
pub fn validate_latitude(lat: f64) -> Result<()> {
    if !(-90.0..=90.0).contains(&lat) {
        return Err(Error::Other(format!(
            "Invalid latitude: {lat}. Must be between -90 and 90"
        )));
    }
    Ok(())
}

/// Validate longitude is within valid range
pub fn validate_longitude(lon: f64) -> Result<()> {
    if !(-180.0..=180.0).contains(&lon) {
        return Err(Error::Other(format!(
            "Invalid longitude: {lon}. Must be between -180 and 180"
        )));
    }
    Ok(())
}

/// Validate heart rate is reasonable (30-250 bpm)
pub fn validate_heart_rate(bpm: f64) -> Result<()> {
    if !(30.0..=250.0).contains(&bpm) {
        return Err(Error::Other(format!(
            "Invalid heart rate: {bpm} bpm. Must be between 30 and 250"
        )));
    }
    Ok(())
}

/// Validate email address has basic structure
pub fn validate_email(email: &str) -> Result<()> {
    if email.is_empty() {
        return Err(Error::Other("Email cannot be empty".to_string()));
    }

    if !email.contains('@') {
        return Err(Error::Other(format!(
            "Invalid email: {email}. Missing @ symbol"
        )));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(Error::Other(format!("Invalid email format: {email}")));
    }

    if !parts[1].contains('.') {
        return Err(Error::Other(format!(
            "Invalid email domain: {email}. Missing domain extension"
        )));
    }

    Ok(())
}

/// Validate URL has basic structure
pub fn validate_url(url: &str) -> Result<()> {
    if url.is_empty() {
        return Err(Error::Other("URL cannot be empty".to_string()));
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(Error::Other(format!(
            "Invalid URL: {url}. Must start with http:// or https://"
        )));
    }

    Ok(())
}

/// Validate timestamp is not in the future (with 5 minute tolerance)
pub fn validate_timestamp_not_future(timestamp: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let now = chrono::Utc::now();
    let tolerance = chrono::Duration::minutes(5);

    if timestamp > now + tolerance {
        return Err(Error::Other(format!(
            "Invalid timestamp: {timestamp}. Cannot be more than 5 minutes in the future"
        )));
    }

    Ok(())
}

/// Validate timestamp is within reasonable range (not before 2000, not more than 5 min in future)
pub fn validate_timestamp_reasonable(timestamp: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let min_date = chrono::DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let max_date = chrono::Utc::now() + chrono::Duration::minutes(5);

    if timestamp < min_date {
        return Err(Error::Other(format!(
            "Invalid timestamp: {timestamp}. Cannot be before 2000-01-01"
        )));
    }

    if timestamp > max_date {
        return Err(Error::Other(format!(
            "Invalid timestamp: {timestamp}. Cannot be more than 5 minutes in the future"
        )));
    }

    Ok(())
}

/// Validate required string field is not empty
pub fn validate_required_string(field_name: &str, value: Option<&str>) -> Result<()> {
    match value {
        None | Some("") => Err(Error::Other(format!(
            "Required field '{field_name}' is missing or empty"
        ))),
        Some(_) => Ok(()),
    }
}

/// Validate positive number
pub fn validate_positive(field_name: &str, value: f64) -> Result<()> {
    if value < 0.0 {
        return Err(Error::Other(format!(
            "Field '{field_name}' must be positive, got: {value}"
        )));
    }
    Ok(())
}

/// Validate percentage (0-100)
pub fn validate_percentage(field_name: &str, value: f64) -> Result<()> {
    if !(0.0..=100.0).contains(&value) {
        return Err(Error::Other(format!(
            "Field '{field_name}' must be between 0 and 100, got: {value}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_latitude() {
        assert!(validate_latitude(0.0).is_ok());
        assert!(validate_latitude(45.5).is_ok());
        assert!(validate_latitude(-45.5).is_ok());
        assert!(validate_latitude(90.0).is_ok());
        assert!(validate_latitude(-90.0).is_ok());

        assert!(validate_latitude(90.1).is_err());
        assert!(validate_latitude(-90.1).is_err());
        assert!(validate_latitude(180.0).is_err());
    }

    #[test]
    fn test_validate_longitude() {
        assert!(validate_longitude(0.0).is_ok());
        assert!(validate_longitude(122.5).is_ok());
        assert!(validate_longitude(-122.5).is_ok());
        assert!(validate_longitude(180.0).is_ok());
        assert!(validate_longitude(-180.0).is_ok());

        assert!(validate_longitude(180.1).is_err());
        assert!(validate_longitude(-180.1).is_err());
    }

    #[test]
    fn test_validate_heart_rate() {
        assert!(validate_heart_rate(60.0).is_ok());
        assert!(validate_heart_rate(30.0).is_ok());
        assert!(validate_heart_rate(250.0).is_ok());

        assert!(validate_heart_rate(29.9).is_err());
        assert!(validate_heart_rate(250.1).is_err());
        assert!(validate_heart_rate(0.0).is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name@example.co.uk").is_ok());

        assert!(validate_email("").is_err());
        assert!(validate_email("notanemail").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@nodomain").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("https://example.com").is_ok());

        assert!(validate_url("").is_err());
        assert!(validate_url("example.com").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_percentage() {
        assert!(validate_percentage("test", 0.0).is_ok());
        assert!(validate_percentage("test", 50.0).is_ok());
        assert!(validate_percentage("test", 100.0).is_ok());

        assert!(validate_percentage("test", -0.1).is_err());
        assert!(validate_percentage("test", 100.1).is_err());
    }
}
