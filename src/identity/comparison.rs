use anyhow::Result;

use crate::analysis::AnalysisResult;
use super::confidence::ConfidenceScore;
use super::profile::AuthorProfile;

/// Result of comparing a document's analysis against an author profile.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub author_name: String,
    pub confidence: ConfidenceScore,
    pub feature_distances: Vec<FeatureDistance>,
}

#[derive(Debug, Clone)]
pub struct FeatureDistance {
    pub feature_name: String,
    pub expected: f64,
    pub actual: f64,
    pub normalized_distance: f64,
}

/// Compare an analysis result against an author profile.
pub fn compare(analysis: &AnalysisResult, profile: &AuthorProfile) -> Result<ComparisonResult> {
    let mut distances = Vec::new();

    distances.push(feature_distance(
        "type_token_ratio",
        profile.avg_type_token_ratio,
        analysis.lexical.type_token_ratio,
    ));
    distances.push(feature_distance(
        "hapax_ratio",
        profile.avg_hapax_ratio,
        analysis.lexical.hapax_ratio,
    ));
    distances.push(feature_distance(
        "avg_word_length",
        profile.avg_word_length,
        analysis.lexical.avg_word_length,
    ));
    distances.push(feature_distance(
        "avg_sentence_length",
        profile.avg_sentence_length,
        analysis.syntactic.avg_sentence_length,
    ));
    distances.push(feature_distance(
        "avg_words_per_sentence",
        profile.avg_words_per_sentence,
        analysis.syntactic.avg_words_per_sentence,
    ));
    distances.push(feature_distance(
        "comma_ratio",
        profile.avg_comma_ratio,
        analysis.stylometric.comma_ratio,
    ));

    let avg_distance: f64 = distances.iter().map(|d| d.normalized_distance).sum::<f64>()
        / distances.len() as f64;

    let confidence_value = (1.0 - avg_distance).max(0.0).min(1.0);
    let confidence = super::confidence::compute(confidence_value, analysis.lexical.total_words);

    Ok(ComparisonResult {
        author_name: profile.name.clone(),
        confidence,
        feature_distances: distances,
    })
}

fn feature_distance(name: &str, expected: f64, actual: f64) -> FeatureDistance {
    let normalized_distance = if expected.abs() > f64::EPSILON {
        ((actual - expected) / expected).abs().min(1.0)
    } else {
        actual.abs().min(1.0)
    };

    FeatureDistance {
        feature_name: name.to_string(),
        expected,
        actual,
        normalized_distance,
    }
}
