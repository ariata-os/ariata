//! Token encryption using AES-256-GCM
//!
//! This module provides authenticated encryption for OAuth tokens stored in the database.
//! It uses AES-256-GCM with a key derived from the ARIATA_ENCRYPTION_KEY environment variable.

use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

use crate::error::{Error, Result};

/// Length of the nonce in bytes (96 bits for GCM)
const NONCE_LENGTH: usize = 12;

/// Token encryptor/decryptor
pub struct TokenEncryptor {
    key: Option<LessSafeKey>,
    rng: SystemRandom,
}

impl TokenEncryptor {
    /// Create a new token encryptor from the environment
    ///
    /// Expects ARIATA_ENCRYPTION_KEY to be a 32-byte base64-encoded key
    pub fn from_env() -> Result<Self> {
        let key_b64 = std::env::var("ARIATA_ENCRYPTION_KEY")
            .map_err(|_| Error::Other(
                "ARIATA_ENCRYPTION_KEY not set. Generate with: openssl rand -base64 32".to_string()
            ))?;

        Self::from_base64_key(&key_b64)
    }

    /// Create a new token encryptor from a base64-encoded key
    pub fn from_base64_key(key_b64: &str) -> Result<Self> {
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(key_b64)
            .map_err(|e| Error::Other(format!("Invalid base64 key: {e}")))?;

        if key_bytes.len() != 32 {
            return Err(Error::Other(format!(
                "Invalid key length: expected 32 bytes, got {}",
                key_bytes.len()
            )));
        }

        let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
            .map_err(|_| Error::Other("Failed to create encryption key".to_string()))?;

        Ok(Self {
            key: Some(LessSafeKey::new(unbound_key)),
            rng: SystemRandom::new(),
        })
    }

    /// Create an insecure encryptor for testing (no actual encryption)
    ///
    /// # Warning
    /// This does NOT encrypt tokens - they are stored in plaintext (base64 encoded).
    /// NEVER use in production!
    #[cfg(test)]
    pub fn new_insecure() -> Self {
        Self {
            key: None,
            rng: SystemRandom::new(),
        }
    }

    /// Encrypt a plaintext token
    ///
    /// Returns base64-encoded ciphertext with format: nonce || encrypted_data || tag
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }

        // If no key (insecure mode), just base64 encode without encryption
        let Some(ref key) = self.key else {
            return Ok(base64::engine::general_purpose::STANDARD.encode(plaintext));
        };

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        self.rng
            .fill(&mut nonce_bytes)
            .map_err(|_| Error::Other("Failed to generate nonce".to_string()))?;

        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Prepare data for encryption (needs extra space for authentication tag)
        let mut in_out = plaintext.as_bytes().to_vec();
        in_out.reserve(AES_256_GCM.tag_len());

        // Encrypt in place
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| Error::Other("Encryption failed".to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&in_out);

        // Encode as base64
        Ok(base64::engine::general_purpose::STANDARD.encode(&result))
    }

    /// Decrypt a base64-encoded ciphertext
    ///
    /// Expects format: nonce || encrypted_data || tag
    pub fn decrypt(&self, ciphertext_b64: &str) -> Result<String> {
        if ciphertext_b64.is_empty() {
            return Ok(String::new());
        }

        // If no key (insecure mode), just base64 decode without decryption
        let Some(ref key) = self.key else {
            let plaintext_bytes = base64::engine::general_purpose::STANDARD
                .decode(ciphertext_b64)
                .map_err(|e| Error::Other(format!("Invalid base64 plaintext: {e}")))?;
            return String::from_utf8(plaintext_bytes)
                .map_err(|e| Error::Other(format!("Invalid UTF-8: {e}")));
        };

        // Decode base64
        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(ciphertext_b64)
            .map_err(|e| Error::Other(format!("Invalid base64 ciphertext: {e}")))?;

        if ciphertext.len() < NONCE_LENGTH {
            return Err(Error::Other("Ciphertext too short".to_string()));
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted) = ciphertext.split_at(NONCE_LENGTH);
        let mut nonce_array = [0u8; NONCE_LENGTH];
        nonce_array.copy_from_slice(nonce_bytes);
        let nonce = Nonce::assume_unique_for_key(nonce_array);

        // Decrypt in place
        let mut in_out = encrypted.to_vec();
        let plaintext = key
            .open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| Error::Other("Decryption failed or data tampered".to_string()))?;

        // Convert to string
        String::from_utf8(plaintext.to_vec())
            .map_err(|e| Error::Other(format!("Invalid UTF-8 after decryption: {e}")))
    }
}

/// Helper to encrypt an optional token
pub fn encrypt_optional(encryptor: &TokenEncryptor, token: Option<&str>) -> Result<Option<String>> {
    match token {
        Some(t) if !t.is_empty() => Ok(Some(encryptor.encrypt(t)?)),
        Some(_) => Ok(Some(String::new())), // Empty string -> Some("")
        None => Ok(None),
    }
}

/// Helper to decrypt an optional token
pub fn decrypt_optional(encryptor: &TokenEncryptor, token: Option<&str>) -> Result<Option<String>> {
    match token {
        Some(t) if !t.is_empty() => Ok(Some(encryptor.decrypt(t)?)),
        Some(_) => Ok(Some(String::new())), // Empty string -> Some("")
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        // Generate a random 32-byte key for testing
        let key_bytes = b"12345678901234567890123456789012";
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);

        let encryptor = TokenEncryptor::from_base64_key(&key_b64).unwrap();

        let plaintext = "ya29.a0AfH6SMB...secret_token...xyz";
        let ciphertext = encryptor.encrypt(plaintext).unwrap();

        // Ciphertext should be different from plaintext
        assert_ne!(ciphertext, plaintext);

        // Should decrypt back to original
        let decrypted = encryptor.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_empty_string() {
        let key_bytes = b"12345678901234567890123456789012";
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);
        let encryptor = TokenEncryptor::from_base64_key(&key_b64).unwrap();

        let ciphertext = encryptor.encrypt("").unwrap();
        assert_eq!(ciphertext, "");

        let decrypted = encryptor.decrypt("").unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_tampered_ciphertext() {
        let key_bytes = b"12345678901234567890123456789012";
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);
        let encryptor = TokenEncryptor::from_base64_key(&key_b64).unwrap();

        let plaintext = "secret_token";
        let mut ciphertext = encryptor.encrypt(plaintext).unwrap();

        // Tamper with the ciphertext
        ciphertext.push('X');

        // Should fail to decrypt
        assert!(encryptor.decrypt(&ciphertext).is_err());
    }

    #[test]
    fn test_optional_helpers() {
        let key_bytes = b"12345678901234567890123456789012";
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key_bytes);
        let encryptor = TokenEncryptor::from_base64_key(&key_b64).unwrap();

        // Test Some(token)
        let encrypted = encrypt_optional(&encryptor, Some("secret")).unwrap();
        assert!(encrypted.is_some());
        let decrypted = decrypt_optional(&encryptor, encrypted.as_deref()).unwrap();
        assert_eq!(decrypted.as_deref(), Some("secret"));

        // Test None
        assert_eq!(encrypt_optional(&encryptor, None).unwrap(), None);
        assert_eq!(decrypt_optional(&encryptor, None).unwrap(), None);

        // Test empty string
        assert_eq!(encrypt_optional(&encryptor, Some("")).unwrap(), Some(String::new()));
    }
}
