use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Minimum number of words required for meaningful analysis
    pub min_words: usize,

    /// SHA-256 hash algorithm identifier
    pub hash_algorithm: String,

    /// Confidence threshold for positive authorship match (0.0 - 1.0)
    pub confidence_threshold: f64,

    /// Directory for storing author profiles
    pub profiles_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_words: 500,
            hash_algorithm: "sha256".to_string(),
            confidence_threshold: 0.85,
            profiles_dir: "profiles".to_string(),
        }
    }
}
