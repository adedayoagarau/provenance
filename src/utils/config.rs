use serde::{Deserialize, Serialize};
use std::path::Path;

use super::errors::{ProvenanceError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Minimum number of words required for meaningful analysis
    #[serde(default = "default_min_words")]
    pub min_words: usize,

    /// Confidence threshold for positive authorship match (0.0 - 1.0)
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,

    /// Directory for storing author profiles
    #[serde(default = "default_profiles_dir")]
    pub profiles_dir: String,

    /// Feature set to use: "minimal", "standard", or "comprehensive"
    #[serde(default = "default_feature_set")]
    pub feature_set: String,
}

fn default_min_words() -> usize { 500 }
fn default_confidence_threshold() -> f64 { 0.85 }
fn default_profiles_dir() -> String { "profiles".to_string() }
fn default_feature_set() -> String { "standard".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            min_words: default_min_words(),
            confidence_threshold: default_confidence_threshold(),
            profiles_dir: default_profiles_dir(),
            feature_set: default_feature_set(),
        }
    }
}

impl Config {
    /// Load config from a TOML file, falling back to defaults for missing fields.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path).map_err(|e| ProvenanceError::ConfigError {
            reason: format!("Failed to read config file '{}': {e}", path.display()),
        })?;

        toml::from_str(&content).map_err(|e| ProvenanceError::ConfigError {
            reason: format!("Failed to parse config file '{}': {e}", path.display()),
        })
    }

    /// Load config from the default location (provenance.toml in current dir).
    pub fn load_default() -> Result<Self> {
        Self::load(Path::new("provenance.toml"))
    }
}
