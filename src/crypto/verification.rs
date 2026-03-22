use super::hashing;

/// Verify that a file matches an expected SHA-256 hash.
pub fn verify_hash(path: &str, expected_hash: &str) -> anyhow::Result<bool> {
    let actual = hashing::sha256_file(path)?;
    Ok(actual == expected_hash)
}
