use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResult;
use super::confidence::ConfidenceScore;
use super::profile::AuthorProfile;

/// Result of comparing a document's analysis against an author profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub author_name: String,
    pub confidence: ConfidenceScore,
    pub feature_distances: Vec<FeatureDistance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDistance {
    pub feature_name: String,
    pub expected: f64,
    pub actual: f64,
    pub normalized_distance: f64,
}

/// Compare an analysis result against an author profile.
///
/// Uses all available features from the expanded analysis suite.
/// Phase 3 will replace this with proper Burrows' Delta computation.
pub fn compare(analysis: &AnalysisResult, profile: &AuthorProfile) -> ComparisonResult {
    let mut distances = Vec::new();

    // Lexical features
    distances.push(fd("mattr", profile.avg_mattr, analysis.lexical.mattr));
    distances.push(fd("yules_k", profile.avg_yules_k, analysis.lexical.yules_k));
    distances.push(fd("hapax_ratio", profile.avg_hapax_ratio, analysis.lexical.hapax_ratio));
    distances.push(fd("avg_word_length", profile.avg_word_length, analysis.lexical.avg_word_length));

    // Syntactic features
    distances.push(fd("avg_words_per_sentence", profile.avg_words_per_sentence, analysis.syntactic.avg_words_per_sentence));
    distances.push(fd("sentence_length_variance", profile.avg_sentence_length_variance, analysis.syntactic.sentence_length_variance));
    distances.push(fd("passive_voice_ratio", profile.avg_passive_voice_ratio, analysis.syntactic.passive_voice_ratio));

    // Stylometric features
    distances.push(fd("comma_ratio", profile.avg_comma_ratio, analysis.stylometric.comma_ratio));
    distances.push(fd("semicolon_ratio", profile.avg_semicolon_ratio, analysis.stylometric.semicolon_ratio));
    distances.push(fd("contraction_ratio", profile.avg_contraction_ratio, analysis.stylometric.contraction_ratio));
    distances.push(fd("hedge_word_ratio", profile.avg_hedge_word_ratio, analysis.stylometric.hedge_word_ratio));
    distances.push(fd("intensifier_ratio", profile.avg_intensifier_ratio, analysis.stylometric.intensifier_ratio));
    distances.push(fd("exclamation_ratio", profile.avg_exclamation_ratio, analysis.stylometric.exclamation_ratio));
    distances.push(fd("question_ratio", profile.avg_question_ratio, analysis.stylometric.question_ratio));

    // Discourse features
    distances.push(fd("discourse_marker_ratio", profile.avg_discourse_marker_ratio, analysis.semantic.discourse_marker_ratio));
    distances.push(fd("flesch_kincaid_grade", profile.avg_flesch_kincaid_grade, analysis.semantic.flesch_kincaid_grade));

    // Function word features
    distances.push(fd("function_word_ratio", profile.avg_function_word_ratio, analysis.function_words.function_word_ratio));

    let avg_distance: f64 = distances.iter().map(|d| d.normalized_distance).sum::<f64>()
        / distances.len() as f64;

    let confidence_value = (1.0 - avg_distance).max(0.0).min(1.0);
    let confidence = super::confidence::compute(confidence_value, analysis.lexical.total_words);

    ComparisonResult {
        author_name: profile.name.clone(),
        confidence,
        feature_distances: distances,
    }
}

fn fd(name: &str, expected: f64, actual: f64) -> FeatureDistance {
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
