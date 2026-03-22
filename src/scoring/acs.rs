//! Authorship Confidence Score (ACS) — a positive claim about authorship.
//!
//! ACS is computed on a 0-100 scale combining process evidence, stylometric match,
//! and text analysis signals. It is a POSITIVE claim about authorship, not a negative
//! claim about machines.

use serde::{Deserialize, Serialize};

use crate::analysis::baselines::RegisterBaselineReport;
use crate::forensics::docx::profile::DocumentConstructionProfile;
use crate::identity::comparison::ComparisonResult;

/// Authorship Confidence Score result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorshipConfidenceScore {
    /// ACS on 0-100 scale.
    pub score: f64,
    /// Confidence interval (±).
    pub margin: f64,
    /// Low end of confidence interval.
    pub score_low: f64,
    /// High end of confidence interval.
    pub score_high: f64,
    /// What evidence was available.
    pub evidence_sources: EvidenceSources,
    /// Weight allocation used.
    pub weights: ScoreWeights,
    /// Per-component scores.
    pub components: ScoreComponents,
}

/// Which evidence sources were available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSources {
    pub has_forensics: bool,
    pub has_stylometric_profile: bool,
    pub has_text_analysis: bool,
    pub has_register_baselines: bool,
}

/// Weights applied to each component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub forensics_weight: f64,
    pub stylometric_weight: f64,
    pub text_analysis_weight: f64,
}

/// Individual component scores (0-100 each).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub forensics_score: Option<f64>,
    pub stylometric_score: Option<f64>,
    pub text_analysis_score: f64,
}

/// Compute the Authorship Confidence Score.
pub fn compute(
    docx_profile: Option<&DocumentConstructionProfile>,
    comparison: Option<&ComparisonResult>,
    baseline_report: Option<&RegisterBaselineReport>,
) -> AuthorshipConfidenceScore {
    let has_forensics = docx_profile.is_some();
    let has_profile = comparison.is_some();
    let has_baselines = baseline_report.is_some();

    // Determine weights based on available evidence
    let weights = determine_weights(has_forensics, has_profile);

    // Compute component scores
    let forensics_score = docx_profile.map(|p| compute_forensics_score(p));
    let stylometric_score = comparison.map(|c| compute_stylometric_score(c));
    let text_analysis_score = compute_text_analysis_score(baseline_report);

    // Weighted combination
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    if let Some(fs) = forensics_score {
        weighted_sum += fs * weights.forensics_weight;
        total_weight += weights.forensics_weight;
    }
    if let Some(ss) = stylometric_score {
        weighted_sum += ss * weights.stylometric_weight;
        total_weight += weights.stylometric_weight;
    }
    weighted_sum += text_analysis_score * weights.text_analysis_weight;
    total_weight += weights.text_analysis_weight;

    let score = if total_weight > 0.0 {
        (weighted_sum / total_weight).clamp(0.0, 100.0)
    } else {
        50.0 // No evidence = maximally uncertain
    };

    // Confidence interval based on evidence availability
    let margin = compute_margin(has_forensics, has_profile, has_baselines);

    AuthorshipConfidenceScore {
        score,
        margin,
        score_low: (score - margin).max(0.0),
        score_high: (score + margin).min(100.0),
        evidence_sources: EvidenceSources {
            has_forensics,
            has_stylometric_profile: has_profile,
            has_text_analysis: true,
            has_register_baselines: has_baselines,
        },
        weights,
        components: ScoreComponents {
            forensics_score,
            stylometric_score,
            text_analysis_score,
        },
    }
}

fn determine_weights(has_forensics: bool, has_profile: bool) -> ScoreWeights {
    match (has_forensics, has_profile) {
        (true, true) => ScoreWeights {
            forensics_weight: 0.40,
            stylometric_weight: 0.30,
            text_analysis_weight: 0.30,
        },
        (true, false) => ScoreWeights {
            forensics_weight: 0.55,
            stylometric_weight: 0.0,
            text_analysis_weight: 0.45,
        },
        (false, true) => ScoreWeights {
            forensics_weight: 0.0,
            stylometric_weight: 0.50,
            text_analysis_weight: 0.50,
        },
        (false, false) => ScoreWeights {
            forensics_weight: 0.0,
            stylometric_weight: 0.0,
            text_analysis_weight: 1.0,
        },
    }
}

fn compute_forensics_score(profile: &DocumentConstructionProfile) -> f64 {
    use crate::forensics::docx::profile::ConstructionPattern;

    let base = match profile.construction_pattern {
        ConstructionPattern::Organic => 85.0,
        ConstructionPattern::Hybrid => 55.0,
        ConstructionPattern::BulkInsertion => 20.0,
        ConstructionPattern::Insufficient => 50.0,
    };

    // Adjust based on process integrity
    let integrity_factor = profile.process_integrity_score / 100.0;
    let adjusted = base * (0.5 + 0.5 * integrity_factor);

    // Penalize for anomalies
    let anomaly_penalty = profile.anomalies.len() as f64 * 3.0;

    (adjusted - anomaly_penalty).clamp(0.0, 100.0)
}

fn compute_stylometric_score(comparison: &ComparisonResult) -> f64 {
    // Convert 0.0-1.0 confidence to 0-100 scale
    (comparison.confidence.value * 100.0).clamp(0.0, 100.0)
}

fn compute_text_analysis_score(baseline_report: Option<&RegisterBaselineReport>) -> f64 {
    let Some(report) = baseline_report else {
        return 60.0; // Default moderate score without baselines
    };

    let total = report.comparisons.len();
    if total == 0 {
        return 60.0;
    }

    // Score based on how many metrics are within expected range
    let expected_ratio = report.expected_count as f64 / total as f64;
    let anomalous_ratio = report.anomalous_count as f64 / total as f64;

    // Mostly expected = high score; anomalous = lower score
    let base = 40.0 + expected_ratio * 50.0 - anomalous_ratio * 30.0;
    base.clamp(0.0, 100.0)
}

fn compute_margin(has_forensics: bool, has_profile: bool, has_baselines: bool) -> f64 {
    let mut margin: f64 = 25.0; // Start wide

    if has_forensics {
        margin -= 8.0;
    }
    if has_profile {
        margin -= 7.0;
    }
    if has_baselines {
        margin -= 3.0;
    }

    margin.max(5.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::docx::profile::{ConstructionPattern, DocumentConstructionProfile};

    fn mock_organic_profile() -> DocumentConstructionProfile {
        DocumentConstructionProfile {
            metadata: None,
            rsid_analysis: None,
            formatting_analysis: None,
            structural_forensics: None,
            construction_pattern: ConstructionPattern::Organic,
            anomalies: Vec::new(),
            process_integrity_score: 80.0,
            assessment_text: "Organic".into(),
            editing_velocity: None,
            saves_per_hour: None,
            creation_to_modification_hours: None,
            words_per_save: None,
        }
    }

    fn mock_bulk_profile() -> DocumentConstructionProfile {
        DocumentConstructionProfile {
            construction_pattern: ConstructionPattern::BulkInsertion,
            process_integrity_score: 40.0,
            anomalies: vec![
                crate::forensics::docx::profile::ForensicAnomaly {
                    anomaly_type: crate::forensics::docx::profile::ForensicAnomalyType::Rsid,
                    location: Some((0, 20)),
                    severity: crate::forensics::docx::profile::ForensicSeverity::High,
                    description: "Bulk paste".into(),
                },
            ],
            ..mock_organic_profile()
        }
    }

    #[test]
    fn test_acs_full_evidence_organic() {
        let profile = mock_organic_profile();
        let acs = compute(Some(&profile), None, None);
        assert!(acs.score > 60.0, "Organic profile should yield ACS > 60, got {}", acs.score);
    }

    #[test]
    fn test_acs_text_only() {
        let acs = compute(None, None, None);
        // Text only = wide margin
        assert!(acs.margin > 20.0);
        assert!(acs.weights.text_analysis_weight > 0.9);
    }

    #[test]
    fn test_acs_bulk_insertion_low() {
        let profile = mock_bulk_profile();
        let acs = compute(Some(&profile), None, None);
        assert!(acs.score < 50.0, "Bulk insertion should yield ACS < 50, got {}", acs.score);
    }

    #[test]
    fn test_acs_bounds() {
        let acs = compute(None, None, None);
        assert!(acs.score >= 0.0 && acs.score <= 100.0);
        assert!(acs.score_low >= 0.0);
        assert!(acs.score_high <= 100.0);
        assert!(acs.score_low <= acs.score);
        assert!(acs.score_high >= acs.score);
    }
}
