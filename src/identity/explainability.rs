//! Feature importance and explainability for authorship decisions.
//!
//! When the system says "72% confidence", this module explains WHY —
//! which specific features drove the decision, what the most discriminating
//! patterns were, and where the text matched or diverged from the profile.

use serde::{Deserialize, Serialize};

use super::comparison::{ComparisonResult, FeatureDistance};
use crate::analysis::AnalysisResult;

/// Complete explanation of why a particular score was assigned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainedDecision {
    /// Top features supporting the match (closest to profile).
    pub supporting_features: Vec<FeatureExplanation>,
    /// Top features against the match (most divergent from profile).
    pub diverging_features: Vec<FeatureExplanation>,
    /// Natural language summary of the decision.
    pub narrative: String,
    /// Overall feature agreement ratio (what fraction of features are close).
    pub agreement_ratio: f64,
    /// Feature importance ranking (all features, sorted by discriminative power).
    pub ranked_features: Vec<RankedFeature>,
}

/// Human-readable explanation of a single feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureExplanation {
    /// Feature name.
    pub feature_name: String,
    /// Human-readable label.
    pub display_name: String,
    /// Value in the query document.
    pub query_value: f64,
    /// Expected value from the author profile.
    pub profile_value: f64,
    /// Normalized distance (0 = perfect match, 1 = maximum divergence).
    pub distance: f64,
    /// Plain-language interpretation.
    pub interpretation: String,
}

/// Feature ranked by discriminative power.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedFeature {
    pub rank: usize,
    pub feature_name: String,
    pub distance: f64,
    pub direction: FeatureDirection,
}

/// Whether a feature value is above or below the profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureDirection {
    /// Query value is higher than profile.
    Above,
    /// Query value is lower than profile.
    Below,
    /// Query value matches profile closely.
    Match,
}

/// Generate an explained decision from a comparison result.
pub fn explain(
    comparison: &ComparisonResult,
    analysis: &AnalysisResult,
) -> ExplainedDecision {
    let mut ranked: Vec<_> = comparison
        .feature_distances
        .iter()
        .enumerate()
        .map(|(i, fd)| RankedFeature {
            rank: i + 1,
            feature_name: fd.feature_name.clone(),
            distance: fd.normalized_distance,
            direction: if (fd.actual - fd.expected).abs() < 0.01 {
                FeatureDirection::Match
            } else if fd.actual > fd.expected {
                FeatureDirection::Above
            } else {
                FeatureDirection::Below
            },
        })
        .collect();

    // Sort by distance (closest first for supporting, farthest for diverging)
    ranked.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

    // Re-assign ranks
    for (i, r) in ranked.iter_mut().enumerate() {
        r.rank = i + 1;
    }

    // Supporting: closest features (distance < 0.15 or top 5)
    let supporting: Vec<_> = comparison
        .feature_distances
        .iter()
        .filter(|fd| fd.normalized_distance < 0.15)
        .take(5)
        .map(|fd| explain_feature(fd, true))
        .collect();

    // Diverging: most distant features (distance > 0.3 or top 5 from end)
    let mut diverging_candidates: Vec<_> = comparison
        .feature_distances
        .iter()
        .filter(|fd| fd.normalized_distance > 0.3)
        .collect();
    diverging_candidates.sort_by(|a, b| {
        b.normalized_distance
            .partial_cmp(&a.normalized_distance)
            .unwrap()
    });
    let diverging: Vec<_> = diverging_candidates
        .iter()
        .take(5)
        .map(|fd| explain_feature(fd, false))
        .collect();

    // Agreement ratio
    let close_count = comparison
        .feature_distances
        .iter()
        .filter(|fd| fd.normalized_distance < 0.20)
        .count();
    let total = comparison.feature_distances.len().max(1);
    let agreement_ratio = close_count as f64 / total as f64;

    let narrative = generate_narrative(
        &comparison.author_name,
        comparison.confidence.value,
        &supporting,
        &diverging,
        agreement_ratio,
        analysis.lexical.total_words,
    );

    ExplainedDecision {
        supporting_features: supporting,
        diverging_features: diverging,
        narrative,
        agreement_ratio,
        ranked_features: ranked,
    }
}

fn explain_feature(fd: &FeatureDistance, is_supporting: bool) -> FeatureExplanation {
    let display_name = human_readable_name(&fd.feature_name);
    let interpretation = if is_supporting {
        format_supporting_interpretation(&display_name, fd)
    } else {
        format_diverging_interpretation(&display_name, fd)
    };

    FeatureExplanation {
        feature_name: fd.feature_name.clone(),
        display_name,
        query_value: fd.actual,
        profile_value: fd.expected,
        distance: fd.normalized_distance,
        interpretation,
    }
}

fn human_readable_name(feature: &str) -> String {
    match feature {
        "mattr" => "Vocabulary diversity (MATTR)".into(),
        "yules_k" => "Vocabulary richness (Yule's K)".into(),
        "hapax_ratio" => "Rare word usage".into(),
        "avg_word_length" => "Average word length".into(),
        "avg_words_per_sentence" => "Sentence length".into(),
        "sentence_length_variance" => "Sentence length variation".into(),
        "passive_voice_ratio" => "Passive voice usage".into(),
        "comma_ratio" => "Comma frequency".into(),
        "semicolon_ratio" => "Semicolon frequency".into(),
        "contraction_ratio" => "Contraction usage".into(),
        "hedge_word_ratio" => "Hedging language".into(),
        "intensifier_ratio" => "Intensifier usage".into(),
        "exclamation_ratio" => "Exclamation usage".into(),
        "question_ratio" => "Question frequency".into(),
        "discourse_marker_ratio" => "Discourse marker density".into(),
        "flesch_kincaid_grade" => "Reading difficulty level".into(),
        "function_word_ratio" => "Function word density".into(),
        _ => feature.replace('_', " ").to_string(),
    }
}

fn format_supporting_interpretation(display_name: &str, fd: &FeatureDistance) -> String {
    let closeness = if fd.normalized_distance < 0.05 {
        "closely matches"
    } else if fd.normalized_distance < 0.10 {
        "is very similar to"
    } else {
        "is consistent with"
    };
    format!(
        "{display_name} in this document ({:.3}) {closeness} the author's typical pattern ({:.3})",
        fd.actual, fd.expected,
    )
}

fn format_diverging_interpretation(display_name: &str, fd: &FeatureDistance) -> String {
    let direction = if fd.actual > fd.expected {
        "higher than"
    } else {
        "lower than"
    };
    format!(
        "{display_name} in this document ({:.3}) is {direction} the author's typical pattern ({:.3})",
        fd.actual, fd.expected,
    )
}

fn generate_narrative(
    author_name: &str,
    confidence: f64,
    supporting: &[FeatureExplanation],
    diverging: &[FeatureExplanation],
    agreement_ratio: f64,
    word_count: usize,
) -> String {
    let mut parts = Vec::new();

    // Opening
    let confidence_pct = confidence * 100.0;
    if confidence_pct >= 85.0 {
        parts.push(format!(
            "The analysis shows strong stylometric consistency with {author_name}'s \
             established writing patterns ({confidence_pct:.0}% confidence)."
        ));
    } else if confidence_pct >= 65.0 {
        parts.push(format!(
            "The analysis shows moderate stylometric consistency with {author_name}'s \
             writing patterns ({confidence_pct:.0}% confidence)."
        ));
    } else if confidence_pct >= 40.0 {
        parts.push(format!(
            "The analysis shows limited consistency with {author_name}'s \
             writing patterns ({confidence_pct:.0}% confidence)."
        ));
    } else {
        parts.push(format!(
            "The analysis does not show strong consistency with {author_name}'s \
             established patterns ({confidence_pct:.0}% confidence)."
        ));
    }

    // Supporting evidence
    if !supporting.is_empty() {
        let top_names: Vec<&str> = supporting
            .iter()
            .take(3)
            .map(|f| f.display_name.as_str())
            .collect();
        parts.push(format!(
            "Key matching patterns include: {}.",
            top_names.join(", "),
        ));
    }

    // Diverging evidence
    if !diverging.is_empty() {
        let div_names: Vec<&str> = diverging
            .iter()
            .take(3)
            .map(|f| f.display_name.as_str())
            .collect();
        parts.push(format!(
            "Notable differences were observed in: {}.",
            div_names.join(", "),
        ));
    }

    // Agreement summary
    parts.push(format!(
        "{:.0}% of measured features fall within the expected range for this author.",
        agreement_ratio * 100.0,
    ));

    // Text length caveat
    if word_count < 500 {
        parts.push(format!(
            "Note: This text contains only {word_count} words. Stylometric analysis is \
             more reliable with longer samples (500+ words)."
        ));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::comparison::FeatureDistance;
    use crate::identity::confidence::{ConfidenceLevel, ConfidenceScore};

    fn mock_comparison() -> ComparisonResult {
        ComparisonResult {
            author_name: "Alice".into(),
            confidence: ConfidenceScore {
                value: 0.78,
                level: ConfidenceLevel::Medium,
                word_count_factor: 0.95,
                feature_count: 16,
            },
            feature_distances: vec![
                FeatureDistance {
                    feature_name: "mattr".into(),
                    expected: 0.75,
                    actual: 0.74,
                    normalized_distance: 0.013,
                },
                FeatureDistance {
                    feature_name: "avg_words_per_sentence".into(),
                    expected: 18.0,
                    actual: 17.5,
                    normalized_distance: 0.028,
                },
                FeatureDistance {
                    feature_name: "passive_voice_ratio".into(),
                    expected: 0.08,
                    actual: 0.12,
                    normalized_distance: 0.50,
                },
                FeatureDistance {
                    feature_name: "contraction_ratio".into(),
                    expected: 0.02,
                    actual: 0.001,
                    normalized_distance: 0.95,
                },
                FeatureDistance {
                    feature_name: "comma_ratio".into(),
                    expected: 0.04,
                    actual: 0.038,
                    normalized_distance: 0.05,
                },
            ],
        }
    }

    fn mock_analysis() -> AnalysisResult {
        crate::analysis::analyze_text(
            "This is a sample text for testing. It contains several sentences. \
             The analysis should work correctly for explainability testing purposes."
        ).unwrap()
    }

    #[test]
    fn test_explain_generates_supporting_and_diverging() {
        let comparison = mock_comparison();
        let analysis = mock_analysis();
        let explained = explain(&comparison, &analysis);

        assert!(
            !explained.supporting_features.is_empty(),
            "Should have supporting features"
        );
        assert!(
            !explained.diverging_features.is_empty(),
            "Should have diverging features"
        );
    }

    #[test]
    fn test_ranking_is_sorted_by_distance() {
        let comparison = mock_comparison();
        let analysis = mock_analysis();
        let explained = explain(&comparison, &analysis);

        for window in explained.ranked_features.windows(2) {
            assert!(
                window[0].distance <= window[1].distance,
                "Ranked features should be sorted by distance: {} ({}) vs {} ({})",
                window[0].feature_name,
                window[0].distance,
                window[1].feature_name,
                window[1].distance,
            );
        }
    }

    #[test]
    fn test_narrative_mentions_author() {
        let comparison = mock_comparison();
        let analysis = mock_analysis();
        let explained = explain(&comparison, &analysis);

        assert!(
            explained.narrative.contains("Alice"),
            "Narrative should mention author: {}",
            explained.narrative
        );
    }

    #[test]
    fn test_narrative_includes_confidence() {
        let comparison = mock_comparison();
        let analysis = mock_analysis();
        let explained = explain(&comparison, &analysis);

        assert!(
            explained.narrative.contains("78%") || explained.narrative.contains("confidence"),
            "Narrative should include confidence: {}",
            explained.narrative
        );
    }

    #[test]
    fn test_human_readable_names() {
        assert_eq!(
            human_readable_name("passive_voice_ratio"),
            "Passive voice usage"
        );
        assert_eq!(
            human_readable_name("mattr"),
            "Vocabulary diversity (MATTR)"
        );
        assert_eq!(
            human_readable_name("unknown_feature"),
            "unknown feature"
        );
    }

    #[test]
    fn test_agreement_ratio() {
        let comparison = mock_comparison();
        let analysis = mock_analysis();
        let explained = explain(&comparison, &analysis);

        assert!(
            explained.agreement_ratio >= 0.0 && explained.agreement_ratio <= 1.0,
            "Agreement ratio must be 0-1"
        );
    }

    #[test]
    fn test_feature_direction() {
        let comparison = mock_comparison();
        let analysis = mock_analysis();
        let explained = explain(&comparison, &analysis);

        // passive_voice: actual (0.12) > expected (0.08) → Above
        let passive = explained
            .ranked_features
            .iter()
            .find(|r| r.feature_name == "passive_voice_ratio")
            .unwrap();
        assert_eq!(passive.direction, FeatureDirection::Above);

        // contraction_ratio: actual (0.001) < expected (0.02) → Below
        let contraction = explained
            .ranked_features
            .iter()
            .find(|r| r.feature_name == "contraction_ratio")
            .unwrap();
        assert_eq!(contraction.direction, FeatureDirection::Below);
    }
}
