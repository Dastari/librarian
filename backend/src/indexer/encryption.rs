//! Credential encryption utilities
//!
//! Uses AES-256-GCM for encrypting sensitive credentials like cookies and API keys.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;

/// AES-256-GCM nonce size (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;
/// AES-256 key size (256 bits = 32 bytes)
const KEY_SIZE: usize = 32;

/// Encryption service for indexer credentials
#[derive(Clone)]
pub struct CredentialEncryption {
    cipher: Aes256Gcm,
}

impl CredentialEncryption {
    /// Create a new encryption service with the given key
    ///
    /// The key should be a 32-byte (256-bit) key, typically stored as an environment variable.
    /// If a shorter key is provided, it will be padded with zeros.
    pub fn new(key: &[u8]) -> Result<Self> {
        let mut key_bytes = [0u8; KEY_SIZE];
        let len = key.len().min(KEY_SIZE);
        key_bytes[..len].copy_from_slice(&key[..len]);

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        Ok(Self { cipher })
    }

    /// Create from a base64-encoded key
    pub fn from_base64_key(key_b64: &str) -> Result<Self> {
        let key = BASE64
            .decode(key_b64)
            .map_err(|e| anyhow!("Invalid base64 key: {}", e))?;
        Self::new(&key)
    }

    /// Create from a hex-encoded key
    pub fn from_hex_key(key_hex: &str) -> Result<Self> {
        let key = hex::decode(key_hex).map_err(|e| anyhow!("Invalid hex key: {}", e))?;
        Self::new(&key)
    }

    /// Generate a random encryption key (for initial setup)
    pub fn generate_key() -> String {
        let mut key = [0u8; KEY_SIZE];
        rand::thread_rng().fill_bytes(&mut key);
        BASE64.encode(key)
    }

    /// Encrypt a plaintext value
    ///
    /// Returns a tuple of (encrypted_data_base64, nonce_base64)
    pub fn encrypt(&self, plaintext: &str) -> Result<(String, String)> {
        // Generate a random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Encode to base64
        let encrypted_b64 = BASE64.encode(&ciphertext);
        let nonce_b64 = BASE64.encode(nonce_bytes);

        Ok((encrypted_b64, nonce_b64))
    }

    /// Decrypt an encrypted value
    ///
    /// Takes the encrypted data (base64) and nonce (base64)
    pub fn decrypt(&self, encrypted_b64: &str, nonce_b64: &str) -> Result<String> {
        // Decode from base64
        let ciphertext = BASE64
            .decode(encrypted_b64)
            .map_err(|e| anyhow!("Invalid encrypted data: {}", e))?;
        let nonce_bytes = BASE64
            .decode(nonce_b64)
            .map_err(|e| anyhow!("Invalid nonce: {}", e))?;

        if nonce_bytes.len() != NONCE_SIZE {
            return Err(anyhow!(
                "Invalid nonce length: expected {}, got {}",
                NONCE_SIZE,
                nonce_bytes.len()
            ));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| anyhow!("Invalid UTF-8 in decrypted data: {}", e))
    }
}

// Implement Debug without exposing the cipher
impl std::fmt::Debug for CredentialEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialEncryption")
            .field("cipher", &"[REDACTED]")
            .finish()
    }
}

/// Encrypted credential value with its nonce
#[derive(Debug, Clone)]
pub struct EncryptedCredential {
    pub encrypted_value: String,
    pub nonce: String,
}

impl EncryptedCredential {
    pub fn new(encrypted_value: String, nonce: String) -> Self {
        Self {
            encrypted_value,
            nonce,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = CredentialEncryption::generate_key();
        let encryption = CredentialEncryption::from_base64_key(&key).unwrap();

        let plaintext = "my-secret-cookie-value";
        let (encrypted, nonce) = encryption.encrypt(plaintext).unwrap();

        // Encrypted should be different from plaintext
        assert_ne!(encrypted, plaintext);

        // Decryption should return original value
        let decrypted = encryption.decrypt(&encrypted, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces() {
        let key = CredentialEncryption::generate_key();
        let encryption = CredentialEncryption::from_base64_key(&key).unwrap();

        let plaintext = "same-value";
        let (encrypted1, nonce1) = encryption.encrypt(plaintext).unwrap();
        let (encrypted2, nonce2) = encryption.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertext (different nonces)
        assert_ne!(encrypted1, encrypted2);
        assert_ne!(nonce1, nonce2);

        // Both should decrypt to the same value
        assert_eq!(encryption.decrypt(&encrypted1, &nonce1).unwrap(), plaintext);
        assert_eq!(encryption.decrypt(&encrypted2, &nonce2).unwrap(), plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = CredentialEncryption::generate_key();
        let key2 = CredentialEncryption::generate_key();

        let encryption1 = CredentialEncryption::from_base64_key(&key1).unwrap();
        let encryption2 = CredentialEncryption::from_base64_key(&key2).unwrap();

        let plaintext = "secret";
        let (encrypted, nonce) = encryption1.encrypt(plaintext).unwrap();

        // Decrypting with wrong key should fail
        assert!(encryption2.decrypt(&encrypted, &nonce).is_err());
    }
}

// Add hex crate for hex decoding
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("Invalid hex string length".to_string());
        }

        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}
