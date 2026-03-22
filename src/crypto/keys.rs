//! Ed25519 key management for Provenance certificates.
//!
//! Generates, saves, and loads Ed25519 signing keypairs. Private keys are stored
//! in PEM-like base64 format; public keys are stored separately for distribution.

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::utils::errors::{ProvenanceError, Result};

/// A serializable public key for certificate verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicKeyInfo {
    /// Base64-encoded Ed25519 public key (32 bytes).
    pub key_base64: String,
    /// Key fingerprint (SHA-256 of the public key bytes, truncated to 16 hex chars).
    pub fingerprint: String,
}

impl PublicKeyInfo {
    /// Create from an Ed25519 verifying key.
    pub fn from_verifying_key(vk: &VerifyingKey) -> Self {
        use base64::Engine;
        use sha2::{Digest, Sha256};

        let bytes = vk.to_bytes();
        let key_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);

        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let fingerprint = format!("{:x}", hash).chars().take(16).collect();

        Self {
            key_base64,
            fingerprint,
        }
    }

    /// Decode back to a VerifyingKey.
    pub fn to_verifying_key(&self) -> Result<VerifyingKey> {
        use base64::Engine;

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.key_base64)
            .map_err(|e| ProvenanceError::IntegrityFailure {
                reason: format!("Invalid public key base64: {e}"),
            })?;

        if bytes.len() != 32 {
            return Err(ProvenanceError::IntegrityFailure {
                reason: format!("Public key must be 32 bytes, got {}", bytes.len()),
            });
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);

        VerifyingKey::from_bytes(&key_bytes).map_err(|e| ProvenanceError::IntegrityFailure {
            reason: format!("Invalid Ed25519 public key: {e}"),
        })
    }
}

/// Generate a new Ed25519 signing keypair.
pub fn generate_keypair() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

/// Save a signing key to a file (base64-encoded).
pub fn save_signing_key(key: &SigningKey, path: &std::path::Path) -> Result<()> {
    use base64::Engine;

    let encoded = base64::engine::general_purpose::STANDARD.encode(key.to_bytes());
    let content = format!("-----BEGIN PROVENANCE SIGNING KEY-----\n{encoded}\n-----END PROVENANCE SIGNING KEY-----\n");

    std::fs::write(path, content).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })
}

/// Load a signing key from a file.
pub fn load_signing_key(path: &std::path::Path) -> Result<SigningKey> {
    use base64::Engine;

    let content = crate::utils::errors::read_file_string(path)?;

    // Extract base64 between PEM markers (or treat whole file as base64)
    let b64 = content
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| ProvenanceError::IntegrityFailure {
            reason: format!("Invalid signing key base64: {e}"),
        })?;

    if bytes.len() != 32 {
        return Err(ProvenanceError::IntegrityFailure {
            reason: format!("Signing key must be 32 bytes, got {}", bytes.len()),
        });
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);

    Ok(SigningKey::from_bytes(&key_bytes))
}

/// Save a public key info to a JSON file.
pub fn save_public_key(info: &PublicKeyInfo, path: &std::path::Path) -> Result<()> {
    let json = serde_json::to_string_pretty(info)?;
    std::fs::write(path, json).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })
}

/// Load a public key info from a JSON file.
pub fn load_public_key(path: &std::path::Path) -> Result<PublicKeyInfo> {
    let content = crate::utils::errors::read_file_string(path)?;
    let info: PublicKeyInfo = serde_json::from_str(&content)?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn test_keypair_generation() {
        let key = generate_keypair();
        let vk = key.verifying_key();
        let info = PublicKeyInfo::from_verifying_key(&vk);

        assert_eq!(info.fingerprint.len(), 16);
        assert!(!info.key_base64.is_empty());

        // Roundtrip
        let recovered = info.to_verifying_key().unwrap();
        assert_eq!(vk, recovered);
    }

    #[test]
    fn test_signing_key_save_load() {
        let key = generate_keypair();
        let dir = std::env::temp_dir().join("provenance_test_keys");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_signing.key");

        save_signing_key(&key, &path).unwrap();
        let loaded = load_signing_key(&path).unwrap();

        assert_eq!(key.to_bytes(), loaded.to_bytes());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_public_key_save_load() {
        let key = generate_keypair();
        let info = PublicKeyInfo::from_verifying_key(&key.verifying_key());

        let dir = std::env::temp_dir().join("provenance_test_pubkey");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_public.json");

        save_public_key(&info, &path).unwrap();
        let loaded = load_public_key(&path).unwrap();

        assert_eq!(info, loaded);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_invalid_public_key() {
        let info = PublicKeyInfo {
            key_base64: base64::engine::general_purpose::STANDARD.encode(b"too short"),
            fingerprint: "abc".into(),
        };
        assert!(info.to_verifying_key().is_err());
    }
}
