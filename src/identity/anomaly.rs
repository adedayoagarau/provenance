use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResult;
use super::profile::AuthorProfile;

/// An anomaly detected in a document section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub section: String,
    pub deviation_score: f64,
    pub description: String,
}

/// Detect anomalous sections that deviate from the author's expected style.
pub fn detect(analysis: &AnalysisResult, profile: &AuthorProfile) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    // Check for significant deviations in key metrics
    let ttr_diff = (analysis.lexical.type_token_ratio - profile.avg_type_token_ratio).abs();
    if ttr_diff > 0.15 {
        anomalies.push(Anomaly {
            section: "vocabulary".to_string(),
            deviation_score: ttr_diff,
            description: format!(
                "Type-token ratio ({:.3}) deviates significantly from profile ({:.3})",
                analysis.lexical.type_token_ratio, profile.avg_type_token_ratio
            ),
        });
    }

    let wl_diff = (analysis.lexical.avg_word_length - profile.avg_word_length).abs();
    if wl_diff > 1.0 {
        anomalies.push(Anomaly {
            section: "word_choice".to_string(),
            deviation_score: wl_diff,
            description: format!(
                "Average word length ({:.2}) deviates from profile ({:.2})",
                analysis.lexical.avg_word_length, profile.avg_word_length
            ),
        });
    }

    anomalies
}
