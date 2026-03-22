use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResult;
use crate::utils::errors::{ProvenanceError, Result};

/// An author's stylistic profile built from verified writing samples.
///
/// Stores averaged feature values across all analyzed samples. This is a
/// summary profile — Phase 3 will add full feature vectors with per-feature
/// mean and standard deviation for Burrows' Delta computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorProfile {
    pub name: String,
    pub sample_count: usize,

    // Lexical features
    pub avg_type_token_ratio: f64,
    pub avg_mattr: f64,
    pub avg_yules_k: f64,
    pub avg_honores_r: f64,
    pub avg_hapax_ratio: f64,
    pub avg_word_length: f64,

    // Syntactic features
    pub avg_sentence_length: f64,
    pub avg_words_per_sentence: f64,
    pub avg_sentence_length_variance: f64,
    pub avg_passive_voice_ratio: f64,

    // Stylometric features
    pub avg_exclamation_ratio: f64,
    pub avg_question_ratio: f64,
    pub avg_comma_ratio: f64,
    pub avg_semicolon_ratio: f64,
    pub avg_contraction_ratio: f64,
    pub avg_hedge_word_ratio: f64,
    pub avg_intensifier_ratio: f64,

    // Discourse features
    pub avg_discourse_marker_ratio: f64,
    pub avg_flesch_kincaid_grade: f64,

    // Function word features
    pub avg_function_word_ratio: f64,
    pub avg_function_word_diversity: f64,
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
        avg_mattr: avg(|a| a.lexical.mattr),
        avg_yules_k: avg(|a| a.lexical.yules_k),
        avg_honores_r: avg(|a| a.lexical.honores_r),
        avg_hapax_ratio: avg(|a| a.lexical.hapax_ratio),
        avg_word_length: avg(|a| a.lexical.avg_word_length),

        avg_sentence_length: avg(|a| a.syntactic.avg_sentence_length),
        avg_words_per_sentence: avg(|a| a.syntactic.avg_words_per_sentence),
        avg_sentence_length_variance: avg(|a| a.syntactic.sentence_length_variance),
        avg_passive_voice_ratio: avg(|a| a.syntactic.passive_voice_ratio),

        avg_exclamation_ratio: avg(|a| a.stylometric.exclamation_ratio),
        avg_question_ratio: avg(|a| a.stylometric.question_ratio),
        avg_comma_ratio: avg(|a| a.stylometric.comma_ratio),
        avg_semicolon_ratio: avg(|a| a.stylometric.semicolon_ratio),
        avg_contraction_ratio: avg(|a| a.stylometric.contraction_ratio),
        avg_hedge_word_ratio: avg(|a| a.stylometric.hedge_word_ratio),
        avg_intensifier_ratio: avg(|a| a.stylometric.intensifier_ratio),

        avg_discourse_marker_ratio: avg(|a| a.semantic.discourse_marker_ratio),
        avg_flesch_kincaid_grade: avg(|a| a.semantic.flesch_kincaid_grade),

        avg_function_word_ratio: avg(|a| a.function_words.function_word_ratio),
        avg_function_word_diversity: avg(|a| a.function_words.function_word_diversity as f64),
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
