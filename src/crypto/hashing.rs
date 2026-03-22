use sha2::{Sha256, Digest};
use crate::utils::errors::{self, Result};

/// Compute SHA-256 hash of a byte slice.
pub fn sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{result:x}")
}

/// Compute SHA-256 hash of a file.
pub fn sha256_file(path: &str) -> Result<String> {
    let data = errors::read_file(std::path::Path::new(path))?;
    Ok(sha256(&data))
}
