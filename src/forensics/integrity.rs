use anyhow::Result;
use sha2::{Sha256, Digest};
use std::path::Path;

/// Result of file integrity checks.
#[derive(Debug, Clone)]
pub struct IntegrityResult {
    pub sha256: String,
    pub file_size: u64,
    pub is_valid: bool,
}

/// Compute integrity checks for a file.
pub fn check(path: &Path) -> Result<IntegrityResult> {
    let content = std::fs::read(path)?;
    let file_size = content.len() as u64;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = hasher.finalize();
    let sha256 = format!("{hash:x}");

    Ok(IntegrityResult {
        sha256,
        file_size,
        is_valid: true,
    })
}
