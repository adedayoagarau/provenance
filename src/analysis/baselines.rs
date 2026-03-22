//! Per-register baselines for text analysis metrics.
//!
//! Each register has expected ranges for key metrics. When a metric falls within
//! the expected range, it is "expected for this text type — not anomalous."
//! When it falls outside, it may warrant attention.

use serde::{Deserialize, Serialize};

use super::register::Register;
use super::AnalysisResult;

/// Expected range for a metric within a register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBaseline {
    pub metric_name: &'static str,
    pub expected_mean: f64,
    pub expected_stddev: f64,
    pub description: &'static str,
}

/// Assessment of a metric relative to its register baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    pub metric_name: String,
    pub actual_value: f64,
    pub expected_mean: f64,
    pub expected_stddev: f64,
    pub z_score: f64,
    pub assessment: MetricAssessment,
    pub explanation: String,
}

/// Color-coded assessment level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricAssessment {
    /// Within expected range for register (within 1 stddev).
    Expected,
    /// Atypical but within normal variance (1-2 stddev).
    Atypical,
    /// Significantly anomalous (> 2 stddev).
    Anomalous,
}

impl MetricAssessment {
    pub fn label(&self) -> &'static str {
        match self {
            MetricAssessment::Expected => "expected",
            MetricAssessment::Atypical => "atypical",
            MetricAssessment::Anomalous => "anomalous",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            MetricAssessment::Expected => "blue",
            MetricAssessment::Atypical => "yellow",
            MetricAssessment::Anomalous => "red",
        }
    }
}

/// Complete baseline comparison for all metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterBaselineReport {
    pub register_label: String,
    pub comparisons: Vec<BaselineComparison>,
    pub anomalous_count: usize,
    pub atypical_count: usize,
    pub expected_count: usize,
}

/// Per-register baselines.
/// Hand-curated from published corpus statistics.
struct RegisterBaselines {
    avg_sentence_length: MetricBaseline,
    passive_voice_ratio: MetricBaseline,
    contraction_ratio: MetricBaseline,
    type_token_ratio: MetricBaseline,
    avg_word_length: MetricBaseline,
    flesch_kincaid_grade: MetricBaseline,
    discourse_marker_ratio: MetricBaseline,
    hedge_word_ratio: MetricBaseline,
    paragraph_length_cv: MetricBaseline,
}

fn baselines_for_register(register: &Register) -> RegisterBaselines {
    let category = register.broad_category();

    match category {
        "academic" => RegisterBaselines {
            avg_sentence_length: MetricBaseline {
                metric_name: "avg_sentence_length",
                expected_mean: 24.0,
                expected_stddev: 5.0,
                description: "Academic writing typically uses longer sentences",
            },
            passive_voice_ratio: MetricBaseline {
                metric_name: "passive_voice_ratio",
                expected_mean: 0.15,
                expected_stddev: 0.06,
                description: "Passive voice is common in academic writing",
            },
            contraction_ratio: MetricBaseline {
                metric_name: "contraction_ratio",
                expected_mean: 0.001,
                expected_stddev: 0.002,
                description: "Contractions are rare in academic writing",
            },
            type_token_ratio: MetricBaseline {
                metric_name: "type_token_ratio",
                expected_mean: 0.55,
                expected_stddev: 0.10,
                description: "Academic texts have moderate vocabulary diversity",
            },
            avg_word_length: MetricBaseline {
                metric_name: "avg_word_length",
                expected_mean: 5.2,
                expected_stddev: 0.5,
                description: "Academic vocabulary uses longer words",
            },
            flesch_kincaid_grade: MetricBaseline {
                metric_name: "flesch_kincaid_grade",
                expected_mean: 14.0,
                expected_stddev: 3.0,
                description: "Academic text reads at graduate level",
            },
            discourse_marker_ratio: MetricBaseline {
                metric_name: "discourse_marker_ratio",
                expected_mean: 0.015,
                expected_stddev: 0.008,
                description: "Academic writing uses discourse markers frequently",
            },
            hedge_word_ratio: MetricBaseline {
                metric_name: "hedge_word_ratio",
                expected_mean: 0.012,
                expected_stddev: 0.006,
                description: "Hedging is common in academic argumentation",
            },
            paragraph_length_cv: MetricBaseline {
                metric_name: "paragraph_length_cv",
                expected_mean: 0.5,
                expected_stddev: 0.2,
                description: "Academic paragraphs show moderate length variation",
            },
        },
        "literary" => RegisterBaselines {
            avg_sentence_length: MetricBaseline {
                metric_name: "avg_sentence_length",
                expected_mean: 16.0,
                expected_stddev: 5.0,
                description: "Literary prose varies widely in sentence length",
            },
            passive_voice_ratio: MetricBaseline {
                metric_name: "passive_voice_ratio",
                expected_mean: 0.06,
                expected_stddev: 0.04,
                description: "Literary writing favors active voice",
            },
            contraction_ratio: MetricBaseline {
                metric_name: "contraction_ratio",
                expected_mean: 0.015,
                expected_stddev: 0.010,
                description: "Contractions are common in dialogue and narration",
            },
            type_token_ratio: MetricBaseline {
                metric_name: "type_token_ratio",
                expected_mean: 0.60,
                expected_stddev: 0.12,
                description: "Literary texts often have rich vocabularies",
            },
            avg_word_length: MetricBaseline {
                metric_name: "avg_word_length",
                expected_mean: 4.5,
                expected_stddev: 0.5,
                description: "Literary prose uses varied word lengths",
            },
            flesch_kincaid_grade: MetricBaseline {
                metric_name: "flesch_kincaid_grade",
                expected_mean: 8.0,
                expected_stddev: 3.0,
                description: "Literary prose targets general readability",
            },
            discourse_marker_ratio: MetricBaseline {
                metric_name: "discourse_marker_ratio",
                expected_mean: 0.008,
                expected_stddev: 0.005,
                description: "Literary writing uses fewer explicit discourse markers",
            },
            hedge_word_ratio: MetricBaseline {
                metric_name: "hedge_word_ratio",
                expected_mean: 0.008,
                expected_stddev: 0.005,
                description: "Hedging varies by narrative style",
            },
            paragraph_length_cv: MetricBaseline {
                metric_name: "paragraph_length_cv",
                expected_mean: 0.7,
                expected_stddev: 0.25,
                description: "Literary paragraphs show high length variation",
            },
        },
        "technical" => RegisterBaselines {
            avg_sentence_length: MetricBaseline {
                metric_name: "avg_sentence_length",
                expected_mean: 18.0,
                expected_stddev: 4.0,
                description: "Technical writing uses moderate sentence lengths",
            },
            passive_voice_ratio: MetricBaseline {
                metric_name: "passive_voice_ratio",
                expected_mean: 0.10,
                expected_stddev: 0.05,
                description: "Some passive voice in technical descriptions",
            },
            contraction_ratio: MetricBaseline {
                metric_name: "contraction_ratio",
                expected_mean: 0.002,
                expected_stddev: 0.003,
                description: "Contractions are rare in technical writing",
            },
            type_token_ratio: MetricBaseline {
                metric_name: "type_token_ratio",
                expected_mean: 0.50,
                expected_stddev: 0.10,
                description: "Technical texts may reuse domain-specific terms",
            },
            avg_word_length: MetricBaseline {
                metric_name: "avg_word_length",
                expected_mean: 5.5,
                expected_stddev: 0.5,
                description: "Technical terminology tends toward longer words",
            },
            flesch_kincaid_grade: MetricBaseline {
                metric_name: "flesch_kincaid_grade",
                expected_mean: 12.0,
                expected_stddev: 3.0,
                description: "Technical writing reads at college level",
            },
            discourse_marker_ratio: MetricBaseline {
                metric_name: "discourse_marker_ratio",
                expected_mean: 0.008,
                expected_stddev: 0.005,
                description: "Technical writing uses fewer discourse markers",
            },
            hedge_word_ratio: MetricBaseline {
                metric_name: "hedge_word_ratio",
                expected_mean: 0.004,
                expected_stddev: 0.003,
                description: "Technical writing is assertive",
            },
            paragraph_length_cv: MetricBaseline {
                metric_name: "paragraph_length_cv",
                expected_mean: 0.6,
                expected_stddev: 0.25,
                description: "Technical paragraphs show moderate variation",
            },
        },
        "casual" => RegisterBaselines {
            avg_sentence_length: MetricBaseline {
                metric_name: "avg_sentence_length",
                expected_mean: 13.0,
                expected_stddev: 4.0,
                description: "Casual writing uses shorter sentences",
            },
            passive_voice_ratio: MetricBaseline {
                metric_name: "passive_voice_ratio",
                expected_mean: 0.04,
                expected_stddev: 0.03,
                description: "Casual writing strongly favors active voice",
            },
            contraction_ratio: MetricBaseline {
                metric_name: "contraction_ratio",
                expected_mean: 0.025,
                expected_stddev: 0.012,
                description: "Contractions are very common in casual writing",
            },
            type_token_ratio: MetricBaseline {
                metric_name: "type_token_ratio",
                expected_mean: 0.55,
                expected_stddev: 0.12,
                description: "Casual texts have moderate vocabulary diversity",
            },
            avg_word_length: MetricBaseline {
                metric_name: "avg_word_length",
                expected_mean: 4.2,
                expected_stddev: 0.4,
                description: "Casual writing uses shorter, common words",
            },
            flesch_kincaid_grade: MetricBaseline {
                metric_name: "flesch_kincaid_grade",
                expected_mean: 7.0,
                expected_stddev: 2.5,
                description: "Casual writing targets broad readability",
            },
            discourse_marker_ratio: MetricBaseline {
                metric_name: "discourse_marker_ratio",
                expected_mean: 0.010,
                expected_stddev: 0.006,
                description: "Casual writing uses informal connectors",
            },
            hedge_word_ratio: MetricBaseline {
                metric_name: "hedge_word_ratio",
                expected_mean: 0.010,
                expected_stddev: 0.006,
                description: "Casual writing includes hedging language",
            },
            paragraph_length_cv: MetricBaseline {
                metric_name: "paragraph_length_cv",
                expected_mean: 0.8,
                expected_stddev: 0.3,
                description: "Casual paragraphs show high variation",
            },
        },
        // Journalistic, professional, educational use moderate defaults
        _ => RegisterBaselines {
            avg_sentence_length: MetricBaseline {
                metric_name: "avg_sentence_length",
                expected_mean: 18.0,
                expected_stddev: 5.0,
                description: "Moderate sentence lengths",
            },
            passive_voice_ratio: MetricBaseline {
                metric_name: "passive_voice_ratio",
                expected_mean: 0.08,
                expected_stddev: 0.05,
                description: "Moderate passive voice usage",
            },
            contraction_ratio: MetricBaseline {
                metric_name: "contraction_ratio",
                expected_mean: 0.008,
                expected_stddev: 0.006,
                description: "Moderate contraction usage",
            },
            type_token_ratio: MetricBaseline {
                metric_name: "type_token_ratio",
                expected_mean: 0.55,
                expected_stddev: 0.10,
                description: "Moderate vocabulary diversity",
            },
            avg_word_length: MetricBaseline {
                metric_name: "avg_word_length",
                expected_mean: 4.8,
                expected_stddev: 0.5,
                description: "Average word length",
            },
            flesch_kincaid_grade: MetricBaseline {
                metric_name: "flesch_kincaid_grade",
                expected_mean: 10.0,
                expected_stddev: 3.0,
                description: "Moderate reading difficulty",
            },
            discourse_marker_ratio: MetricBaseline {
                metric_name: "discourse_marker_ratio",
                expected_mean: 0.010,
                expected_stddev: 0.006,
                description: "Moderate discourse marker usage",
            },
            hedge_word_ratio: MetricBaseline {
                metric_name: "hedge_word_ratio",
                expected_mean: 0.008,
                expected_stddev: 0.005,
                description: "Moderate hedging",
            },
            paragraph_length_cv: MetricBaseline {
                metric_name: "paragraph_length_cv",
                expected_mean: 0.6,
                expected_stddev: 0.25,
                description: "Moderate paragraph length variation",
            },
        },
    }
}

/// Compare analysis metrics against register-specific baselines.
pub fn compare(
    analysis: &AnalysisResult,
    register: &Register,
) -> RegisterBaselineReport {
    let baselines = baselines_for_register(register);

    // Compute paragraph length CV from stylometric profile
    let para_cv = if analysis.stylometric.avg_paragraph_length > 0.0 {
        analysis.stylometric.paragraph_length_variance.sqrt()
            / analysis.stylometric.avg_paragraph_length
    } else {
        0.0
    };

    let metrics: Vec<(&MetricBaseline, f64)> = vec![
        (&baselines.avg_sentence_length, analysis.syntactic.avg_words_per_sentence),
        (&baselines.passive_voice_ratio, analysis.syntactic.passive_voice_ratio),
        (&baselines.contraction_ratio, analysis.stylometric.contraction_ratio),
        (&baselines.type_token_ratio, analysis.lexical.type_token_ratio),
        (&baselines.avg_word_length, analysis.lexical.avg_word_length),
        (&baselines.flesch_kincaid_grade, analysis.semantic.flesch_kincaid_grade),
        (&baselines.discourse_marker_ratio, analysis.semantic.discourse_marker_ratio),
        (&baselines.hedge_word_ratio, analysis.stylometric.hedge_word_ratio),
        (&baselines.paragraph_length_cv, para_cv),
    ];

    let mut comparisons = Vec::new();

    for (baseline, actual) in &metrics {
        let z_score = if baseline.expected_stddev > 0.0 {
            (actual - baseline.expected_mean) / baseline.expected_stddev
        } else {
            0.0
        };

        let assessment = if z_score.abs() <= 1.0 {
            MetricAssessment::Expected
        } else if z_score.abs() <= 2.0 {
            MetricAssessment::Atypical
        } else {
            MetricAssessment::Anomalous
        };

        let explanation = format_explanation(
            baseline.metric_name,
            *actual,
            &assessment,
            register.label(),
            baseline.description,
        );

        comparisons.push(BaselineComparison {
            metric_name: baseline.metric_name.to_string(),
            actual_value: *actual,
            expected_mean: baseline.expected_mean,
            expected_stddev: baseline.expected_stddev,
            z_score,
            assessment,
            explanation,
        });
    }

    let anomalous_count = comparisons
        .iter()
        .filter(|c| c.assessment == MetricAssessment::Anomalous)
        .count();
    let atypical_count = comparisons
        .iter()
        .filter(|c| c.assessment == MetricAssessment::Atypical)
        .count();
    let expected_count = comparisons
        .iter()
        .filter(|c| c.assessment == MetricAssessment::Expected)
        .count();

    RegisterBaselineReport {
        register_label: register.label().to_string(),
        comparisons,
        anomalous_count,
        atypical_count,
        expected_count,
    }
}

fn format_explanation(
    metric: &str,
    actual: f64,
    assessment: &MetricAssessment,
    register_label: &str,
    baseline_desc: &str,
) -> String {
    let metric_label = match metric {
        "avg_sentence_length" => "Average sentence length",
        "passive_voice_ratio" => "Passive voice ratio",
        "contraction_ratio" => "Contraction frequency",
        "type_token_ratio" => "Vocabulary diversity (TTR)",
        "avg_word_length" => "Average word length",
        "flesch_kincaid_grade" => "Reading grade level",
        "discourse_marker_ratio" => "Discourse marker density",
        "hedge_word_ratio" => "Hedge word frequency",
        "paragraph_length_cv" => "Paragraph length variation",
        _ => metric,
    };

    match assessment {
        MetricAssessment::Expected => {
            format!(
                "{metric_label}: {actual:.3} — expected for {register_label} ({baseline_desc})"
            )
        }
        MetricAssessment::Atypical => {
            format!(
                "{metric_label}: {actual:.3} — atypical for {register_label}, but within normal variance"
            )
        }
        MetricAssessment::Anomalous => {
            format!(
                "{metric_label}: {actual:.3} — significantly outside expected range for {register_label}"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::register;

    #[test]
    fn test_baseline_comparison_academic() {
        let text = "The court held that the defendant's actions constituted a breach of \
                    fiduciary duty. Furthermore, the appellate court affirmed the lower \
                    court's decision, noting that the evidence was sufficient to establish \
                    liability. The doctrine of respondeat superior was applied, whereby \
                    the employer was held vicariously liable for the acts committed by the \
                    employee within the scope of employment.";
        let analysis = crate::analysis::analyze_text(text).unwrap();
        let classification = register::classify(&analysis);
        let report = compare(&analysis, &classification.primary);

        // Academic legal text should have most metrics within expected range
        assert!(
            report.expected_count >= report.anomalous_count,
            "Academic text should have more expected than anomalous metrics, got {} expected vs {} anomalous",
            report.expected_count,
            report.anomalous_count
        );
    }

    #[test]
    fn test_baseline_comparison_casual_with_formality() {
        // Casual text that's unusually formal should flag some atypical metrics
        let text = "One might consider the implications of such a decision. Furthermore, \
                    the ramifications extend beyond the immediate context. It is worth \
                    noting that the consequences were not anticipated by the participants.";
        let analysis = crate::analysis::analyze_text(text).unwrap();
        let casual = Register::Casual(register::CasualSubtype::BlogPost);
        let report = compare(&analysis, &casual);

        // Forcing casual baseline on formal text should produce some atypical/anomalous
        let non_expected = report.atypical_count + report.anomalous_count;
        assert!(
            non_expected > 0,
            "Formal text compared against casual baseline should have non-expected metrics"
        );
    }

    #[test]
    fn test_z_score_computation() {
        let text = "Hello world. This is a test. Short sentences only.";
        let analysis = crate::analysis::analyze_text(text).unwrap();
        let register = Register::Academic(register::AcademicSubtype::General);
        let report = compare(&analysis, &register);

        // Verify z-scores are computed
        for comparison in &report.comparisons {
            assert!(comparison.z_score.is_finite());
        }
    }

    #[test]
    fn test_explanation_format() {
        let explanation = format_explanation(
            "avg_sentence_length",
            25.0,
            &MetricAssessment::Expected,
            "Academic / Legal",
            "Academic writing typically uses longer sentences",
        );
        assert!(explanation.contains("expected for Academic / Legal"));
    }
}
