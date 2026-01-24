//! Cryptographic utilities for API key generation and credential encryption
//!
//! This module provides:
//! - Secure API key generation with configurable prefixes
//! - SHA-256 hashing for API key storage
//! - AES-256-GCM encryption for provider credentials (envelope encryption)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Crypto-related errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("invalid nonce length: expected {expected}, got {actual}")]
    InvalidNonceLength { expected: usize, actual: usize },

    #[error("base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
}

/// API key prefix for live/production keys
pub const API_KEY_PREFIX_LIVE: &str = "aura_live_";
/// API key prefix for test/development keys
pub const API_KEY_PREFIX_TEST: &str = "aura_test_";

/// Length of the random part of API keys (in bytes, will be hex-encoded)
const API_KEY_RANDOM_BYTES: usize = 24;

/// AES-256-GCM key size (32 bytes)
const AES_KEY_SIZE: usize = 32;
/// AES-GCM nonce size (12 bytes)
const NONCE_SIZE: usize = 12;

/// Result of generating a new API key
#[derive(Debug, Clone)]
pub struct GeneratedApiKey {
    /// The full API key to return to the user (only shown once)
    pub key: String,
    /// The key_id (prefix + identifier) for lookups
    pub key_id: String,
    /// The hash of the full key for secure storage
    pub key_hash: String,
}

/// Generate a new API key with the specified prefix
///
/// # Arguments
/// * `prefix` - The prefix to use (e.g., "aura_live_" or "aura_test_")
///
/// # Returns
/// A `GeneratedApiKey` containing:
/// - `key`: The full API key (shown to user once)
/// - `key_id`: The public identifier for lookups
/// - `key_hash`: SHA-256 hash for secure storage
///
/// # Example
/// ```
/// use aura_core::crypto::{generate_api_key, API_KEY_PREFIX_LIVE};
///
/// let api_key = generate_api_key(API_KEY_PREFIX_LIVE);
/// println!("Key (show once): {}", api_key.key);
/// println!("Key ID: {}", api_key.key_id);
/// // Store key_id and key_hash in database
/// ```
pub fn generate_api_key(prefix: &str) -> GeneratedApiKey {
    let mut random_bytes = [0u8; API_KEY_RANDOM_BYTES];
    OsRng.fill_bytes(&mut random_bytes);

    // The identifier part (public, used for lookups)
    let identifier = hex::encode(&random_bytes[..12]);

    // The secret part (only shown once)
    let secret = hex::encode(&random_bytes[12..]);

    // Full key: prefix + identifier + secret
    let key_id = format!("{}{}", prefix, identifier);
    let key = format!("{}{}", key_id, secret);

    // Hash the full key for secure storage
    let key_hash = hash_api_key(&key);

    GeneratedApiKey {
        key,
        key_id,
        key_hash,
    }
}

/// Hash an API key using SHA-256
///
/// This is used to securely store API keys - we never store the raw key,
/// only the hash. When validating, we hash the provided key and compare.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify an API key against a stored hash
pub fn verify_api_key(key: &str, stored_hash: &str) -> bool {
    let computed_hash = hash_api_key(key);
    // Use constant-time comparison to prevent timing attacks
    constant_time_compare(computed_hash.as_bytes(), stored_hash.as_bytes())
}

/// Extract the key_id from a full API key
///
/// The key_id is the prefix + first 24 characters of the random part
pub fn extract_key_id(full_key: &str) -> Option<String> {
    // API key format: prefix (10-11 chars) + identifier (24 chars) + secret (24 chars)
    // Total: ~58-59 characters
    if full_key.starts_with(API_KEY_PREFIX_LIVE) {
        let id_end = API_KEY_PREFIX_LIVE.len() + 24;
        if full_key.len() >= id_end {
            return Some(full_key[..id_end].to_string());
        }
    } else if full_key.starts_with(API_KEY_PREFIX_TEST) {
        let id_end = API_KEY_PREFIX_TEST.len() + 24;
        if full_key.len() >= id_end {
            return Some(full_key[..id_end].to_string());
        }
    }
    None
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

// ============================================================================
// Envelope Encryption for Provider Credentials
// ============================================================================

/// Data Encryption Key (DEK) for envelope encryption
#[derive(Debug, Clone)]
pub struct DataEncryptionKey {
    key: [u8; AES_KEY_SIZE],
}

impl DataEncryptionKey {
    /// Generate a new random DEK
    pub fn generate() -> Self {
        let mut key = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    /// Create DEK from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != AES_KEY_SIZE {
            return Err(CryptoError::InvalidKeyLength {
                expected: AES_KEY_SIZE,
                actual: bytes.len(),
            });
        }
        let mut key = [0u8; AES_KEY_SIZE];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }

    /// Get the raw key bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
}

/// Master Encryption Key (KEK) for wrapping DEKs
#[derive(Debug, Clone)]
pub struct MasterKey {
    key: [u8; AES_KEY_SIZE],
}

impl MasterKey {
    /// Create a master key from a hex-encoded string (from environment)
    pub fn from_hex(hex_key: &str) -> Result<Self, CryptoError> {
        let bytes =
            hex::decode(hex_key).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Create a master key from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != AES_KEY_SIZE {
            return Err(CryptoError::InvalidKeyLength {
                expected: AES_KEY_SIZE,
                actual: bytes.len(),
            });
        }
        let mut key = [0u8; AES_KEY_SIZE];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }

    /// Generate a new random master key (for initial setup)
    pub fn generate() -> Self {
        let mut key = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    /// Export the master key as a hex string (for storage in secure vault)
    pub fn to_hex(&self) -> String {
        hex::encode(self.key)
    }

    /// Wrap (encrypt) a DEK with this master key
    pub fn wrap_dek(&self, dek: &DataEncryptionKey) -> Result<Vec<u8>, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, dek.as_bytes())
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        // Return: nonce || ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Unwrap (decrypt) a wrapped DEK
    pub fn unwrap_dek(&self, wrapped_dek: &[u8]) -> Result<DataEncryptionKey, CryptoError> {
        if wrapped_dek.len() < NONCE_SIZE {
            return Err(CryptoError::DecryptionFailed(
                "Wrapped DEK too short".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

        let nonce = Nonce::from_slice(&wrapped_dek[..NONCE_SIZE]);
        let ciphertext = &wrapped_dek[NONCE_SIZE..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

        DataEncryptionKey::from_bytes(&plaintext)
    }
}

/// Encrypted credential data
#[derive(Debug, Clone)]
pub struct EncryptedCredential {
    /// The encrypted API key
    pub ciphertext: Vec<u8>,
    /// The wrapped DEK (encrypted with master key)
    pub wrapped_dek: Vec<u8>,
    /// The nonce used for encrypting the credential
    pub nonce: [u8; NONCE_SIZE],
}

impl EncryptedCredential {
    /// Get encryption parameters as JSON for storage
    pub fn encryption_params(&self) -> serde_json::Value {
        serde_json::json!({
            "nonce": hex::encode(self.nonce),
            "algorithm": "AES-256-GCM"
        })
    }
}

/// Encrypt a provider API key using envelope encryption
///
/// This uses a two-tier encryption approach:
/// 1. Generate a random Data Encryption Key (DEK)
/// 2. Encrypt the API key with the DEK (AES-256-GCM)
/// 3. Encrypt (wrap) the DEK with the Master Key
///
/// # Arguments
/// * `master_key` - The master key for wrapping the DEK
/// * `api_key` - The plaintext provider API key to encrypt
///
/// # Returns
/// An `EncryptedCredential` containing the ciphertext, wrapped DEK, and nonce
pub fn encrypt_credential(
    master_key: &MasterKey,
    api_key: &str,
) -> Result<EncryptedCredential, CryptoError> {
    // Generate a new DEK for this credential
    let dek = DataEncryptionKey::generate();

    // Wrap the DEK with the master key
    let wrapped_dek = master_key.wrap_dek(&dek)?;

    // Encrypt the API key with the DEK
    let cipher = Aes256Gcm::new_from_slice(dek.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, api_key.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    Ok(EncryptedCredential {
        ciphertext,
        wrapped_dek,
        nonce: nonce_bytes,
    })
}

/// Decrypt a provider API key using envelope encryption
///
/// # Arguments
/// * `master_key` - The master key for unwrapping the DEK
/// * `encrypted` - The encrypted credential data
///
/// # Returns
/// The decrypted API key as a string
pub fn decrypt_credential(
    master_key: &MasterKey,
    ciphertext: &[u8],
    wrapped_dek: &[u8],
    nonce: &[u8],
) -> Result<String, CryptoError> {
    if nonce.len() != NONCE_SIZE {
        return Err(CryptoError::InvalidNonceLength {
            expected: NONCE_SIZE,
            actual: nonce.len(),
        });
    }

    // Unwrap the DEK
    let dek = master_key.unwrap_dek(wrapped_dek)?;

    // Decrypt the credential with the DEK
    let cipher = Aes256Gcm::new_from_slice(dek.as_bytes())
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(nonce);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    String::from_utf8(plaintext).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
}

/// Parse encryption params JSON to extract nonce
pub fn parse_encryption_params(
    params: &serde_json::Value,
) -> Result<[u8; NONCE_SIZE], CryptoError> {
    let nonce_hex = params
        .get("nonce")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CryptoError::DecryptionFailed("Missing nonce in params".to_string()))?;

    let nonce_bytes =
        hex::decode(nonce_hex).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    if nonce_bytes.len() != NONCE_SIZE {
        return Err(CryptoError::InvalidNonceLength {
            expected: NONCE_SIZE,
            actual: nonce_bytes.len(),
        });
    }

    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&nonce_bytes);
    Ok(nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key(API_KEY_PREFIX_LIVE);

        assert!(key.key.starts_with(API_KEY_PREFIX_LIVE));
        assert!(key.key_id.starts_with(API_KEY_PREFIX_LIVE));
        assert!(key.key.starts_with(&key.key_id));
        assert!(!key.key_hash.is_empty());

        // Verify the hash
        assert!(verify_api_key(&key.key, &key.key_hash));
    }

    #[test]
    fn test_generate_test_api_key() {
        let key = generate_api_key(API_KEY_PREFIX_TEST);

        assert!(key.key.starts_with(API_KEY_PREFIX_TEST));
        assert!(key.key_id.starts_with(API_KEY_PREFIX_TEST));
    }

    #[test]
    fn test_extract_key_id() {
        let key = generate_api_key(API_KEY_PREFIX_LIVE);
        let extracted = extract_key_id(&key.key);

        assert_eq!(extracted, Some(key.key_id.clone()));
    }

    #[test]
    fn test_verify_api_key() {
        let key = generate_api_key(API_KEY_PREFIX_LIVE);

        // Correct key should verify
        assert!(verify_api_key(&key.key, &key.key_hash));

        // Wrong key should not verify
        assert!(!verify_api_key("wrong_key", &key.key_hash));
    }

    #[test]
    fn test_hash_consistency() {
        let key = "test_api_key_12345";
        let hash1 = hash_api_key(key);
        let hash2 = hash_api_key(key);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_master_key_generation() {
        let key = MasterKey::generate();
        let hex = key.to_hex();

        // Hex should be 64 characters (32 bytes * 2)
        assert_eq!(hex.len(), 64);

        // Should be able to recreate from hex
        let key2 = MasterKey::from_hex(&hex).unwrap();
        assert_eq!(key.key, key2.key);
    }

    #[test]
    fn test_dek_wrapping() {
        let master = MasterKey::generate();
        let dek = DataEncryptionKey::generate();

        let wrapped = master.wrap_dek(&dek).unwrap();
        let unwrapped = master.unwrap_dek(&wrapped).unwrap();

        assert_eq!(dek.as_bytes(), unwrapped.as_bytes());
    }

    #[test]
    fn test_credential_encryption() {
        let master = MasterKey::generate();
        let api_key = "sk-test-1234567890abcdef";

        let encrypted = encrypt_credential(&master, api_key).unwrap();

        let decrypted = decrypt_credential(
            &master,
            &encrypted.ciphertext,
            &encrypted.wrapped_dek,
            &encrypted.nonce,
        )
        .unwrap();

        assert_eq!(decrypted, api_key);
    }

    #[test]
    fn test_credential_encryption_with_params() {
        let master = MasterKey::generate();
        let api_key = "sk-ant-1234567890abcdef";

        let encrypted = encrypt_credential(&master, api_key).unwrap();
        let params = encrypted.encryption_params();

        // Parse params and decrypt
        let nonce = parse_encryption_params(&params).unwrap();
        let decrypted = decrypt_credential(
            &master,
            &encrypted.ciphertext,
            &encrypted.wrapped_dek,
            &nonce,
        )
        .unwrap();

        assert_eq!(decrypted, api_key);
    }

    #[test]
    fn test_wrong_master_key_fails() {
        let master1 = MasterKey::generate();
        let master2 = MasterKey::generate();
        let api_key = "sk-test-secret";

        let encrypted = encrypt_credential(&master1, api_key).unwrap();

        // Should fail with wrong master key
        let result = decrypt_credential(
            &master2,
            &encrypted.ciphertext,
            &encrypted.wrapped_dek,
            &encrypted.nonce,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_constant_time_compare() {
        let a = b"hello";
        let b = b"hello";
        let c = b"world";
        let d = b"hell";

        assert!(constant_time_compare(a, b));
        assert!(!constant_time_compare(a, c));
        assert!(!constant_time_compare(a, d));
    }
}
