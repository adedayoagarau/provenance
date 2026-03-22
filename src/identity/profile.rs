use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResult;
use crate::utils::errors::{ProvenanceError, Result};

/// An author's stylistic profile built from verified writing samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorProfile {
    pub name: String,
    pub sample_count: usize,
    pub avg_type_token_ratio: f64,
    pub avg_hapax_ratio: f64,
    pub avg_word_length: f64,
    pub avg_sentence_length: f64,
    pub avg_words_per_sentence: f64,
    pub avg_exclamation_ratio: f64,
    pub avg_question_ratio: f64,
    pub avg_comma_ratio: f64,
}

/// Build an author profile from multiple analysis results.
pub fn build(name: &str, analyses: &[AnalysisResult]) -> Result<AuthorProfile> {
    if analyses.is_empty() {
        return Err(ProvenanceError::ProfileError {
            reason: "No analyses provided to build profile".to_string(),
        });
    }

    let n = analyses.len() as f64;
    let avg = |f: fn(&AnalysisResult) -> f64| -> f64 {
        analyses.iter().map(f).sum::<f64>() / n
    };

    Ok(AuthorProfile {
        name: name.to_string(),
        sample_count: analyses.len(),
        avg_type_token_ratio: avg(|a| a.lexical.type_token_ratio),
        avg_hapax_ratio: avg(|a| a.lexical.hapax_ratio),
        avg_word_length: avg(|a| a.lexical.avg_word_length),
        avg_sentence_length: avg(|a| a.syntactic.avg_sentence_length),
        avg_words_per_sentence: avg(|a| a.syntactic.avg_words_per_sentence),
        avg_exclamation_ratio: avg(|a| a.stylometric.exclamation_ratio),
        avg_question_ratio: avg(|a| a.stylometric.question_ratio),
        avg_comma_ratio: avg(|a| a.stylometric.comma_ratio),
    })
}

/// Load an author profile from a JSON file.
pub fn load(path: &str) -> Result<AuthorProfile> {
    let content = std::fs::read_to_string(path)?;
    let profile: AuthorProfile = serde_json::from_str(&content)?;
    Ok(profile)
}

/// Save an author profile to a JSON file, returning the file path.
pub fn save(profile: &AuthorProfile) -> Result<String> {
    let dir = "profiles";
    std::fs::create_dir_all(dir)?;

    let filename = format!("{}/{}.json", dir, profile.name.to_lowercase().replace(' ', "_"));
    let json = serde_json::to_string_pretty(profile)?;
    std::fs::write(&filename, json)?;

    Ok(filename)
}
