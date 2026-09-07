//! Credential encryption utilities
//!
//! Uses AES-256-GCM for encrypting sensitive credentials like cookies and API keys.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::Rng;
use sha2::{Digest, Sha256};

/// AES-256-GCM nonce size (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;
/// AES-256 key size (256 bits = 32 bytes)
const KEY_SIZE: usize = 32;

/// Encryption service for source credentials
#[derive(Clone)]
pub struct CredentialEncryption {
    cipher: Aes256Gcm,
    key_id: String,
}

impl CredentialEncryption {
    /// Create a new encryption service with the given key
    ///
    /// The key must be exactly 32 bytes (256 bits). Malformed, short, and long
    /// keys fail closed rather than being padded or truncated.
    pub fn new(key: &[u8]) -> Result<Self> {
        if key.len() != KEY_SIZE {
            return Err(anyhow!(
                "Invalid credential key length: expected {KEY_SIZE} bytes, got {}",
                key.len()
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
        let key_id = hex_prefix(&Sha256::digest(key), 16);

        Ok(Self { cipher, key_id })
    }

    /// Create from a base64-encoded key
    pub fn from_base64_key(key_b64: &str) -> Result<Self> {
        let key = BASE64
            .decode(key_b64)
            .map_err(|e| anyhow!("Invalid base64 key: {}", e))?;
        Self::new(&key)
    }

    /// Generate a random encryption key (for initial setup)
    pub fn generate_key() -> String {
        let mut key = [0u8; KEY_SIZE];
        rand::rng().fill_bytes(&mut key);
        BASE64.encode(key)
    }

    /// Non-secret identifier embedded in versioned envelopes. It allows a
    /// wrong-key failure before attempting decryption without exposing key bytes.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Encrypt to the current versioned envelope:
    /// `v2:<key-id>:<nonce-base64>:<ciphertext-base64>`.
    pub fn encrypt_envelope(&self, plaintext: &str) -> Result<String> {
        let (ciphertext, nonce) = self.encrypt(plaintext)?;
        Ok(format!("v2:{}:{}:{}", self.key_id, nonce, ciphertext))
    }

    /// Decrypt a current versioned envelope. Legacy envelopes are intentionally
    /// handled by the migration compatibility path, not silently accepted here.
    pub fn decrypt_envelope(&self, envelope: &str) -> Result<String> {
        let mut parts = envelope.splitn(4, ':');
        let version = parts.next();
        let key_id = parts.next();
        let nonce = parts.next();
        let ciphertext = parts.next();
        if version != Some("v2") || key_id.is_none() || nonce.is_none() || ciphertext.is_none() {
            return Err(anyhow!("Unsupported credential envelope"));
        }
        if key_id != Some(self.key_id.as_str()) {
            return Err(anyhow!("Credential envelope key does not match active key"));
        }
        self.decrypt(ciphertext.unwrap_or_default(), nonce.unwrap_or_default())
    }

    /// Decrypt the historical `nonce:ciphertext` storage format.
    pub fn decrypt_legacy_envelope(&self, envelope: &str) -> Result<String> {
        let (nonce, ciphertext) = envelope
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid legacy credential envelope"))?;
        self.decrypt(ciphertext, nonce)
    }

    pub fn looks_like_envelope(value: &str) -> bool {
        if value.starts_with("v2:") {
            return value.split(':').count() == 4;
        }
        let Some((nonce, ciphertext)) = value.split_once(':') else {
            return false;
        };
        let Ok(nonce) = BASE64.decode(nonce) else {
            return false;
        };
        nonce.len() == NONCE_SIZE && !ciphertext.is_empty() && BASE64.decode(ciphertext).is_ok()
    }

    /// Encrypt a plaintext value
    ///
    /// Returns a tuple of (encrypted_data_base64, nonce_base64)
    pub fn encrypt(&self, plaintext: &str) -> Result<(String, String)> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        let encrypted_b64 = BASE64.encode(&ciphertext);
        let nonce_b64 = BASE64.encode(nonce_bytes);

        Ok((encrypted_b64, nonce_b64))
    }

    /// Decrypt an encrypted value
    ///
    /// Takes the encrypted data (base64) and nonce (base64)
    pub fn decrypt(&self, encrypted_b64: &str, nonce_b64: &str) -> Result<String> {
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

        let nonce_bytes: [u8; NONCE_SIZE] = nonce_bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid nonce length"))?;
        let nonce = Nonce::from(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| anyhow!("Invalid UTF-8 in decrypted data: {}", e))
    }
}

/// Context-aware credential encryption for Source write transforms.
///
/// Signature: `async fn(&Context, String) -> async_graphql::Result<String>`
///
/// Takes a plaintext JSON credential string and returns a versioned encrypted envelope.
/// The database service calls this from its ORM write transform.
pub async fn encrypt_credentials_ctx(
    ctx: &async_graphql::Context<'_>,
    plaintext: String,
) -> async_graphql::Result<String> {
    // If the plaintext is empty, store empty string
    if plaintext.is_empty() {
        return Ok(String::new());
    }

    let manager = ctx
        .data::<std::sync::Arc<crate::services::ServicesManager>>()
        .map_err(|_| async_graphql::Error::new("ServicesManager not available in context"))?;

    let sources_svc = manager
        .get_sources()
        .await
        .ok_or_else(|| async_graphql::Error::new("Sources service not available"))?;

    let sources_manager = sources_svc
        .get_manager()
        .await
        .ok_or_else(|| async_graphql::Error::new("Sources manager not initialized"))?;

    sources_manager
        .encryption()
        .encrypt_envelope(&plaintext)
        .map_err(|_| async_graphql::Error::new("Credential encryption failed"))
}

impl std::fmt::Debug for CredentialEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialEncryption")
            .field("cipher", &"[REDACTED]")
            .field("key_id", &"[REDACTED]")
            .finish()
    }
}

fn hex_prefix(bytes: &[u8], characters: usize) -> String {
    let mut output = String::with_capacity(characters);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut output, "{byte:02x}");
        if output.len() >= characters {
            output.truncate(characters);
            break;
        }
    }
    output
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

        assert_ne!(encrypted, plaintext);

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

        assert_ne!(encrypted1, encrypted2);
        assert_ne!(nonce1, nonce2);

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

        assert!(encryption2.decrypt(&encrypted, &nonce).is_err());
    }

    #[test]
    fn rejects_keys_that_are_not_exactly_32_bytes() {
        assert!(CredentialEncryption::new(&[0_u8; 31]).is_err());
        assert!(CredentialEncryption::new(&[0_u8; 33]).is_err());
        assert!(CredentialEncryption::from_base64_key("not-base64").is_err());
    }

    #[test]
    fn versioned_envelope_round_trips_and_rejects_wrong_key() {
        let encryption =
            CredentialEncryption::from_base64_key(&CredentialEncryption::generate_key()).unwrap();
        let wrong =
            CredentialEncryption::from_base64_key(&CredentialEncryption::generate_key()).unwrap();
        let envelope = encryption
            .encrypt_envelope("{\"ApiKey\":\"secret\"}")
            .unwrap();

        assert!(envelope.starts_with(&format!("v2:{}:", encryption.key_id())));
        assert_eq!(
            encryption.decrypt_envelope(&envelope).unwrap(),
            "{\"ApiKey\":\"secret\"}"
        );
        assert!(wrong.decrypt_envelope(&envelope).is_err());
    }
}
