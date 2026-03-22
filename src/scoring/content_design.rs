//! Content Design Layer — evaluator-facing framing, reliability disclosures,
//! and plain-language interpretation of scores.
//!
//! This module translates raw scores into evaluator-usable prose.
//! It enforces framing principles:
//! - Present evidence and observations, not conclusions
//! - Never claim to detect AI authorship
//! - Always disclose known limitations
//! - Frame anomalies as questions, not accusations

use serde::{Deserialize, Serialize};

use crate::scoring::acs::AuthorshipConfidenceScore;
use crate::scoring::anomalies::AnomalyReport;
use crate::scoring::pii::ProcessIntegrityIndex;
use crate::analysis::register::RegisterClassification;

/// Complete evaluator-facing report content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorReport {
    /// Top-line summary for the evaluator.
    pub executive_summary: String,
    /// Reliability disclosure — what this tool can and cannot do.
    pub reliability_disclosure: ReliabilityDisclosure,
    /// Register identification and what it means.
    pub register_context: RegisterContext,
    /// ACS interpretation for evaluators.
    pub authorship_assessment: AuthorshipAssessment,
    /// PII interpretation for evaluators.
    pub integrity_assessment: IntegrityAssessment,
    /// Anomaly narrative for evaluators.
    pub anomaly_narrative: AnomalyNarrative,
    /// Recommendations for the evaluator.
    pub recommended_actions: Vec<String>,
}

/// Reliability disclosure — always shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityDisclosure {
    pub header: String,
    pub points: Vec<String>,
    pub evidence_basis: String,
}

/// Register context for the evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterContext {
    pub summary: String,
    pub implication: String,
}

/// ACS interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorshipAssessment {
    pub summary: String,
    pub score_display: String,
    pub confidence_interval: String,
    pub interpretation: String,
    pub caveats: Vec<String>,
}

/// PII interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityAssessment {
    pub summary: String,
    pub level: String,
    pub guidance: String,
}

/// Anomaly narrative for evaluators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyNarrative {
    pub summary: String,
    pub details: Vec<String>,
    pub framing_note: String,
}

/// Generate the evaluator-facing report.
pub fn generate(
    acs: &AuthorshipConfidenceScore,
    pii_score: &ProcessIntegrityIndex,
    anomalies: &AnomalyReport,
    register: &RegisterClassification,
) -> EvaluatorReport {
    let executive_summary = generate_executive_summary(acs, pii_score, anomalies);
    let reliability_disclosure = generate_reliability_disclosure(pii_score);
    let register_context = generate_register_context(register);
    let authorship_assessment = generate_authorship_assessment(acs, pii_score);
    let integrity_assessment = generate_integrity_assessment(pii_score);
    let anomaly_narrative = generate_anomaly_narrative(anomalies);
    let recommended_actions = generate_recommendations(acs, pii_score, anomalies);

    EvaluatorReport {
        executive_summary,
        reliability_disclosure,
        register_context,
        authorship_assessment,
        integrity_assessment,
        anomaly_narrative,
        recommended_actions,
    }
}

fn generate_executive_summary(
    acs: &AuthorshipConfidenceScore,
    pii_score: &ProcessIntegrityIndex,
    anomalies: &AnomalyReport,
) -> String {
    let evidence_quality = pii_score.level.label();
    let anomaly_count = anomalies.flags.len();

    if anomaly_count == 0 {
        format!(
            "This analysis found no specific anomalies in the document. \
             Process evidence quality is {}. Authorship confidence score: {:.0}/100 \
             (±{:.0}).",
            evidence_quality.to_lowercase(),
            acs.score,
            acs.margin,
        )
    } else {
        format!(
            "This analysis identified {} anomal{} warranting further review. \
             Process evidence quality is {}. Authorship confidence score: {:.0}/100 \
             (±{:.0}). See anomaly details below.",
            anomaly_count,
            if anomaly_count == 1 { "y" } else { "ies" },
            evidence_quality.to_lowercase(),
            acs.score,
            acs.margin,
        )
    }
}

fn generate_reliability_disclosure(pii_score: &ProcessIntegrityIndex) -> ReliabilityDisclosure {
    let mut points = vec![
        "This tool analyzes writing style patterns and document metadata. \
         It does not and cannot determine whether text was written by a human or machine."
            .to_string(),
        "Stylometric analysis is a statistical technique with known error rates. \
         Results should be treated as one input among many, not as definitive evidence."
            .to_string(),
        "Short texts (under 500 words) produce less reliable results due to \
         insufficient data for stable stylometric measurement."
            .to_string(),
    ];

    if pii_score.score < 30.0 {
        points.push(
            "Limited process evidence is available for this document. \
             The assessment relies primarily on text analysis, which is the \
             least reliable evidence category."
                .to_string(),
        );
    }

    let evidence_basis = if pii_score.score > 70.0 {
        "This assessment is based on document metadata, process forensics, \
         and text analysis — multiple independent evidence sources."
    } else if pii_score.score > 30.0 {
        "This assessment is based on partial process evidence and text analysis."
    } else {
        "This assessment is based on text analysis only. No process evidence \
         (document metadata, revision history, editing forensics) was available."
    };

    ReliabilityDisclosure {
        header: "Important: Reliability Disclosure".to_string(),
        points,
        evidence_basis: evidence_basis.to_string(),
    }
}

fn generate_register_context(register: &RegisterClassification) -> RegisterContext {
    let summary = format!(
        "Detected register: {} (confidence: {:.0}%).",
        register.label, register.confidence * 100.0,
    );

    let implication = format!(
        "All metrics below are compared against baselines for {} writing. \
         Metrics that are typical for this register are marked as expected; \
         metrics that deviate significantly are flagged for review.",
        register.primary.broad_category(),
    );

    RegisterContext {
        summary,
        implication,
    }
}

fn generate_authorship_assessment(
    acs: &AuthorshipConfidenceScore,
    pii_score: &ProcessIntegrityIndex,
) -> AuthorshipAssessment {
    let score_display = format!("{:.0}/100", acs.score);
    let confidence_interval = format!(
        "{:.0}–{:.0} (±{:.0})",
        acs.score_low, acs.score_high, acs.margin
    );

    let interpretation = if acs.score >= 75.0 {
        "Available evidence supports consistent authorship patterns."
    } else if acs.score >= 50.0 {
        "Available evidence shows some authorship indicators but is not conclusive."
    } else if acs.score >= 25.0 {
        "Available evidence shows limited authorship indicators. \
         Additional evidence may be needed."
    } else {
        "Available evidence does not strongly support authorship claims. \
         This may reflect insufficient evidence rather than a definitive finding."
    };

    let mut caveats = Vec::new();
    if acs.margin > 15.0 {
        caveats.push(
            "The wide confidence interval reflects limited evidence. \
             More data sources would narrow this range."
                .to_string(),
        );
    }
    if !acs.evidence_sources.has_forensics {
        caveats.push(
            "No document process evidence (revision history, RSID data) \
             was available. Score is based on text analysis only."
                .to_string(),
        );
    }
    if !acs.evidence_sources.has_stylometric_profile {
        caveats.push(
            "No reference writing sample was provided. Stylometric comparison \
             was not possible."
                .to_string(),
        );
    }
    if pii_score.score < 30.0 {
        caveats.push(format!(
            "Process Integrity Index is {:.0}/100 ({}). \
             This score should be interpreted with extra caution.",
            pii_score.score,
            pii_score.level.label().to_lowercase(),
        ));
    }

    AuthorshipAssessment {
        summary: format!("Authorship Confidence Score: {score_display}"),
        score_display,
        confidence_interval,
        interpretation: interpretation.to_string(),
        caveats,
    }
}

fn generate_integrity_assessment(pii_score: &ProcessIntegrityIndex) -> IntegrityAssessment {
    IntegrityAssessment {
        summary: format!(
            "Process Integrity Index: {:.0}/100 ({})",
            pii_score.score,
            pii_score.level.label(),
        ),
        level: pii_score.level.label().to_string(),
        guidance: pii_score.guidance.clone(),
    }
}

fn generate_anomaly_narrative(anomalies: &AnomalyReport) -> AnomalyNarrative {
    let details: Vec<String> = anomalies
        .flags
        .iter()
        .map(|flag| {
            format!(
                "[{}] {} — {}",
                flag.severity.label(),
                flag.anomaly_type.label(),
                flag.description,
            )
        })
        .collect();

    AnomalyNarrative {
        summary: anomalies.summary.clone(),
        details,
        framing_note: "Anomalies indicate areas that may warrant further discussion \
                       with the author. They do not constitute evidence of misconduct."
            .to_string(),
    }
}

fn generate_recommendations(
    acs: &AuthorshipConfidenceScore,
    pii_score: &ProcessIntegrityIndex,
    anomalies: &AnomalyReport,
) -> Vec<String> {
    let mut recs = Vec::new();

    if pii_score.score < 30.0 {
        recs.push(
            "Request the original document file (.docx) for process evidence analysis."
                .to_string(),
        );
    }

    if !acs.evidence_sources.has_stylometric_profile {
        recs.push(
            "Provide a reference writing sample for stylometric comparison."
                .to_string(),
        );
    }

    if anomalies.high_count > 0 {
        recs.push(
            "Discuss flagged high-severity anomalies with the author before drawing conclusions."
                .to_string(),
        );
    }

    if acs.margin > 15.0 {
        recs.push(
            "Collect additional evidence to narrow the confidence interval."
                .to_string(),
        );
    }

    if recs.is_empty() {
        recs.push(
            "No specific follow-up actions required based on current evidence."
                .to_string(),
        );
    }

    recs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::acs::{AuthorshipConfidenceScore, EvidenceSources, ScoreComponents, ScoreWeights};
    use crate::scoring::anomalies::{AnomalyFlag, AnomalyLocation, AnomalyReport, AnomalySeverity, AnomalyType};
    use crate::scoring::pii::{IntegrityComponents, IntegrityLevel, ProcessIntegrityIndex};
    use crate::analysis::register::{Register, AcademicSubtype, RegisterClassification, RegisterFeatures};

    fn mock_acs() -> AuthorshipConfidenceScore {
        AuthorshipConfidenceScore {
            score: 72.0,
            margin: 12.0,
            score_low: 60.0,
            score_high: 84.0,
            evidence_sources: EvidenceSources {
                has_forensics: true,
                has_stylometric_profile: false,
                has_text_analysis: true,
                has_register_baselines: true,
            },
            weights: ScoreWeights {
                forensics_weight: 0.55,
                stylometric_weight: 0.0,
                text_analysis_weight: 0.45,
            },
            components: ScoreComponents {
                forensics_score: Some(80.0),
                stylometric_score: None,
                text_analysis_score: 62.0,
            },
        }
    }

    fn mock_pii() -> ProcessIntegrityIndex {
        ProcessIntegrityIndex {
            score: 55.0,
            level: IntegrityLevel::Moderate,
            guidance: "Moderate process evidence".into(),
            components: IntegrityComponents {
                metadata_completeness: 20.0,
                rsid_richness: 15.0,
                formatting_quality: 10.0,
                revision_evidence: 5.0,
                additional_evidence: 5.0,
            },
        }
    }

    fn mock_anomalies(count: usize) -> AnomalyReport {
        let flags: Vec<AnomalyFlag> = (0..count)
            .map(|i| AnomalyFlag {
                anomaly_type: AnomalyType::Paste,
                location: AnomalyLocation::ParagraphRange {
                    start: i * 5,
                    end: i * 5 + 4,
                },
                severity: if i == 0 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                },
                description: format!("Test anomaly {i}"),
                recommended_action: "Review".into(),
                contributing_signals: vec!["Test".into()],
            })
            .collect();
        AnomalyReport {
            high_count: if count > 0 { 1 } else { 0 },
            medium_count: count.saturating_sub(1),
            low_count: 0,
            summary: format!("{count} anomalies"),
            flags,
        }
    }

    fn mock_register() -> RegisterClassification {
        RegisterClassification {
            primary: Register::Academic(AcademicSubtype::General),
            confidence: 0.72,
            label: "Academic / General".into(),
            feature_scores: RegisterFeatures {
                formality_score: 0.65,
                avg_sentence_length: 22.0,
                passive_voice_ratio: 0.12,
                technical_term_density: 0.4,
                personal_pronoun_freq: 8.0,
                question_ratio: 0.02,
                contraction_ratio: 0.001,
                discourse_marker_ratio: 0.015,
                reading_grade: 14.0,
                hedge_ratio: 0.01,
                avg_word_length: 5.3,
            },
        }
    }

    #[test]
    fn test_evaluator_report_generation() {
        let report = generate(
            &mock_acs(),
            &mock_pii(),
            &mock_anomalies(2),
            &mock_register(),
        );
        assert!(!report.executive_summary.is_empty());
        assert!(!report.reliability_disclosure.points.is_empty());
        assert!(report.register_context.summary.contains("Academic"));
        assert!(report.authorship_assessment.score_display.contains("72"));
    }

    #[test]
    fn test_framing_never_claims_ai() {
        let report = generate(
            &mock_acs(),
            &mock_pii(),
            &mock_anomalies(3),
            &mock_register(),
        );

        let full_text = format!(
            "{} {} {} {} {} {}",
            report.executive_summary,
            report.reliability_disclosure.points.join(" "),
            report.authorship_assessment.interpretation,
            report.anomaly_narrative.summary,
            report.anomaly_narrative.framing_note,
            report.anomaly_narrative.details.join(" "),
        )
        .to_lowercase();

        assert!(
            !full_text.contains("ai-generated"),
            "Report must not claim AI generation"
        );
        assert!(
            !full_text.contains("written by ai"),
            "Report must not claim AI authorship"
        );
        assert!(
            !full_text.contains("plagiarism"),
            "Report must not use accusation language"
        );
    }

    #[test]
    fn test_reliability_disclosure_always_present() {
        let report = generate(
            &mock_acs(),
            &mock_pii(),
            &mock_anomalies(0),
            &mock_register(),
        );
        assert!(report.reliability_disclosure.points.len() >= 3);
        assert!(report
            .reliability_disclosure
            .points[0]
            .contains("does not and cannot determine"));
    }

    #[test]
    fn test_low_evidence_caveats() {
        let mut acs = mock_acs();
        acs.evidence_sources.has_forensics = false;
        acs.evidence_sources.has_stylometric_profile = false;
        acs.margin = 25.0;

        let mut pii_val = mock_pii();
        pii_val.score = 10.0;
        pii_val.level = IntegrityLevel::Insufficient;

        let report = generate(&acs, &pii_val, &mock_anomalies(0), &mock_register());

        assert!(
            report.authorship_assessment.caveats.len() >= 2,
            "Low evidence should produce multiple caveats"
        );
        assert!(
            report.recommended_actions.len() >= 2,
            "Low evidence should produce multiple recommendations"
        );
    }

    #[test]
    fn test_no_anomalies_narrative() {
        let report = generate(
            &mock_acs(),
            &mock_pii(),
            &mock_anomalies(0),
            &mock_register(),
        );
        assert!(report.executive_summary.contains("no specific anomalies"));
    }

    #[test]
    fn test_recommendations_with_anomalies() {
        let report = generate(
            &mock_acs(),
            &mock_pii(),
            &mock_anomalies(2),
            &mock_register(),
        );
        assert!(
            report
                .recommended_actions
                .iter()
                .any(|r| r.contains("Discuss")),
            "Should recommend discussing anomalies"
        );
    }
}
