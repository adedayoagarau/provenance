//! Bias audit framework — tests Provenance for fairness across demographics,
//! registers, and language backgrounds.
//!
//! A bias audit evaluates whether Provenance produces systematically different
//! results for different population groups when the underlying authorship is the same.

use serde::{Deserialize, Serialize};

/// A demographic/contextual group for bias testing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationGroup {
    /// Group identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Category of the group.
    pub category: GroupCategory,
    /// Number of samples in the test set.
    pub sample_count: usize,
}

/// Categories for population grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroupCategory {
    /// Native vs non-native English speaker.
    LanguageBackground,
    /// Text register (academic, literary, technical, etc.).
    Register,
    /// Writer's experience level.
    ExperienceLevel,
    /// Document format.
    DocumentFormat,
    /// Text length category.
    TextLength,
    /// Writing style (formal, informal, mixed).
    WritingStyle,
}

impl GroupCategory {
    pub fn label(&self) -> &'static str {
        match self {
            GroupCategory::LanguageBackground => "Language Background",
            GroupCategory::Register => "Register",
            GroupCategory::ExperienceLevel => "Experience Level",
            GroupCategory::DocumentFormat => "Document Format",
            GroupCategory::TextLength => "Text Length",
            GroupCategory::WritingStyle => "Writing Style",
        }
    }
}

/// Result of a bias audit for one metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasTestResult {
    /// Metric being tested (e.g., "ACS", "PII", "false_positive_rate").
    pub metric_name: String,
    /// Per-group results.
    pub group_results: Vec<GroupMetricResult>,
    /// Whether significant bias was detected.
    pub bias_detected: bool,
    /// Maximum difference between any two groups.
    pub max_disparity: f64,
    /// Statistical significance level (p-value equivalent, simplified).
    pub significance: f64,
    /// Assessment text.
    pub assessment: String,
}

/// A metric result for one population group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMetricResult {
    /// Group identifier.
    pub group: PopulationGroup,
    /// Mean metric value for this group.
    pub mean_value: f64,
    /// Standard deviation of metric for this group.
    pub std_dev: f64,
    /// 95% confidence interval.
    pub ci_low: f64,
    pub ci_high: f64,
    /// Sample count.
    pub n: usize,
}

/// Full bias audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasAuditReport {
    /// Date of the audit.
    pub audit_date: String,
    /// Provenance version tested.
    pub software_version: String,
    /// Total samples tested.
    pub total_samples: usize,
    /// Per-metric bias test results.
    pub tests: Vec<BiasTestResult>,
    /// Groups that were tested.
    pub groups_tested: Vec<PopulationGroup>,
    /// Overall fairness score (0.0-1.0, 1.0 = perfectly fair).
    pub fairness_score: f64,
    /// Known limitations and caveats.
    pub limitations: Vec<String>,
    /// Documented failures (where bias IS expected or known).
    pub documented_failures: Vec<DocumentedFailure>,
}

/// A known and documented bias or limitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentedFailure {
    /// Description of the failure.
    pub description: String,
    /// Which groups are affected.
    pub affected_groups: Vec<String>,
    /// Severity (low/medium/high).
    pub severity: String,
    /// Mitigation strategy.
    pub mitigation: String,
}

/// Standard population groups for bias testing.
pub fn standard_groups() -> Vec<PopulationGroup> {
    vec![
        // Language background
        PopulationGroup {
            id: "native-en".into(),
            name: "Native English Speaker".into(),
            category: GroupCategory::LanguageBackground,
            sample_count: 0,
        },
        PopulationGroup {
            id: "esl-high".into(),
            name: "ESL — High Proficiency".into(),
            category: GroupCategory::LanguageBackground,
            sample_count: 0,
        },
        PopulationGroup {
            id: "esl-mid".into(),
            name: "ESL — Intermediate Proficiency".into(),
            category: GroupCategory::LanguageBackground,
            sample_count: 0,
        },
        // Registers
        PopulationGroup {
            id: "reg-academic".into(),
            name: "Academic Writing".into(),
            category: GroupCategory::Register,
            sample_count: 0,
        },
        PopulationGroup {
            id: "reg-literary".into(),
            name: "Literary/Creative Writing".into(),
            category: GroupCategory::Register,
            sample_count: 0,
        },
        PopulationGroup {
            id: "reg-technical".into(),
            name: "Technical Documentation".into(),
            category: GroupCategory::Register,
            sample_count: 0,
        },
        PopulationGroup {
            id: "reg-casual".into(),
            name: "Casual/Blog Writing".into(),
            category: GroupCategory::Register,
            sample_count: 0,
        },
        PopulationGroup {
            id: "reg-journalistic".into(),
            name: "Journalistic Writing".into(),
            category: GroupCategory::Register,
            sample_count: 0,
        },
        // Experience levels
        PopulationGroup {
            id: "exp-student".into(),
            name: "Student Writer".into(),
            category: GroupCategory::ExperienceLevel,
            sample_count: 0,
        },
        PopulationGroup {
            id: "exp-professional".into(),
            name: "Professional Writer".into(),
            category: GroupCategory::ExperienceLevel,
            sample_count: 0,
        },
        PopulationGroup {
            id: "exp-academic".into(),
            name: "Academic Researcher".into(),
            category: GroupCategory::ExperienceLevel,
            sample_count: 0,
        },
        // Text lengths
        PopulationGroup {
            id: "len-short".into(),
            name: "Short Text (<500 words)".into(),
            category: GroupCategory::TextLength,
            sample_count: 0,
        },
        PopulationGroup {
            id: "len-medium".into(),
            name: "Medium Text (500-2000 words)".into(),
            category: GroupCategory::TextLength,
            sample_count: 0,
        },
        PopulationGroup {
            id: "len-long".into(),
            name: "Long Text (2000+ words)".into(),
            category: GroupCategory::TextLength,
            sample_count: 0,
        },
    ]
}

/// Compute a bias test from per-group metric values.
pub fn compute_bias_test(
    metric_name: &str,
    group_values: &[(PopulationGroup, Vec<f64>)],
) -> BiasTestResult {
    let group_results: Vec<GroupMetricResult> = group_values
        .iter()
        .map(|(group, values)| {
            let n = values.len();
            let mean = if n > 0 {
                values.iter().sum::<f64>() / n as f64
            } else {
                0.0
            };
            let variance = if n > 1 {
                values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64
            } else {
                0.0
            };
            let std_dev = variance.sqrt();
            let se = if n > 0 { std_dev / (n as f64).sqrt() } else { 0.0 };

            GroupMetricResult {
                group: group.clone(),
                mean_value: mean,
                std_dev,
                ci_low: mean - 1.96 * se,
                ci_high: mean + 1.96 * se,
                n,
            }
        })
        .collect();

    // Compute max disparity
    let means: Vec<f64> = group_results.iter().map(|r| r.mean_value).collect();
    let max_disparity = if means.len() >= 2 {
        let max = means.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = means.iter().cloned().fold(f64::INFINITY, f64::min);
        max - min
    } else {
        0.0
    };

    // Simplified significance test: check if confidence intervals overlap
    let significance = compute_significance(&group_results);
    let bias_detected = max_disparity > 10.0 && significance < 0.05;

    let assessment = if !bias_detected {
        format!(
            "No significant bias detected for {metric_name} across tested groups (max disparity: {max_disparity:.1})"
        )
    } else {
        let highest = group_results
            .iter()
            .max_by(|a, b| a.mean_value.partial_cmp(&b.mean_value).unwrap())
            .map(|r| r.group.name.as_str())
            .unwrap_or("unknown");
        let lowest = group_results
            .iter()
            .min_by(|a, b| a.mean_value.partial_cmp(&b.mean_value).unwrap())
            .map(|r| r.group.name.as_str())
            .unwrap_or("unknown");
        format!(
            "Significant bias detected for {metric_name}: {highest} scored {max_disparity:.1} points higher than {lowest}"
        )
    };

    BiasTestResult {
        metric_name: metric_name.to_string(),
        group_results,
        bias_detected,
        max_disparity,
        significance,
        assessment,
    }
}

/// Generate the standard documented failures/limitations.
pub fn standard_documented_failures() -> Vec<DocumentedFailure> {
    vec![
        DocumentedFailure {
            description: "ESL writers may show higher stylometric anomaly rates due to non-native patterns that resemble AI-generated text".into(),
            affected_groups: vec!["esl-mid".into(), "esl-high".into()],
            severity: "medium".into(),
            mitigation: "Register-aware baselines (Phase 13) adjust expectations for ESL text. Forensic signals are unaffected by language background.".into(),
        },
        DocumentedFailure {
            description: "Short texts (<300 words) produce less reliable assessments across all groups".into(),
            affected_groups: vec!["len-short".into()],
            severity: "high".into(),
            mitigation: "Text length is reported in every assessment. Short texts receive 'preliminary' designation with wider confidence intervals.".into(),
        },
        DocumentedFailure {
            description: "Technical documentation may be flagged for low vocabulary diversity, which is natural for the register".into(),
            affected_groups: vec!["reg-technical".into()],
            severity: "low".into(),
            mitigation: "Register classification adjusts baselines for technical text. Metrics are compared against register-specific norms.".into(),
        },
        DocumentedFailure {
            description: "Highly formulaic academic writing (legal briefs, grant applications) may show patterns similar to template-based AI text".into(),
            affected_groups: vec!["reg-academic".into()],
            severity: "medium".into(),
            mitigation: "DOCX forensics and process capture are not affected by text formulaicity. The system reports reduced text-signal reliability for formulaic registers.".into(),
        },
        DocumentedFailure {
            description: "Non-DOCX formats (PDF, plain text) lack forensic evidence, increasing reliance on text-only signals".into(),
            affected_groups: vec!["all non-DOCX submissions".into()],
            severity: "medium".into(),
            mitigation: "PII score reflects reduced evidence. Reports state 'text analysis only' with appropriate disclaimers.".into(),
        },
    ]
}

/// Simplified significance computation.
fn compute_significance(results: &[GroupMetricResult]) -> f64 {
    if results.len() < 2 {
        return 1.0; // Not significant
    }

    // Check if all confidence intervals overlap
    let all_overlap = results.windows(2).all(|pair| {
        pair[0].ci_high >= pair[1].ci_low && pair[1].ci_high >= pair[0].ci_low
    });

    if all_overlap {
        0.10 // Not significant
    } else {
        0.01 // Significant
    }
}

/// Generate a template bias audit report.
pub fn generate_audit_template() -> BiasAuditReport {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    BiasAuditReport {
        audit_date: crate::scoring::engine::format_epoch_public(now),
        software_version: env!("CARGO_PKG_VERSION").to_string(),
        total_samples: 0,
        tests: Vec::new(),
        groups_tested: standard_groups(),
        fairness_score: 0.0,
        limitations: vec![
            "This audit framework requires a labeled evaluation dataset (Phase 12.5) to produce meaningful results.".into(),
            "Current results are based on theoretical analysis, not empirical measurement.".into(),
            "Bias testing should be repeated quarterly as the system evolves.".into(),
        ],
        documented_failures: standard_documented_failures(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_groups() {
        let groups = standard_groups();
        assert!(groups.len() >= 10);

        let categories: Vec<GroupCategory> = groups.iter().map(|g| g.category).collect();
        assert!(categories.contains(&GroupCategory::LanguageBackground));
        assert!(categories.contains(&GroupCategory::Register));
        assert!(categories.contains(&GroupCategory::TextLength));
    }

    #[test]
    fn test_compute_bias_no_disparity() {
        let groups = vec![
            (
                PopulationGroup {
                    id: "a".into(),
                    name: "Group A".into(),
                    category: GroupCategory::Register,
                    sample_count: 10,
                },
                vec![70.0, 72.0, 68.0, 71.0, 73.0, 69.0, 70.5, 71.5, 72.5, 68.5],
            ),
            (
                PopulationGroup {
                    id: "b".into(),
                    name: "Group B".into(),
                    category: GroupCategory::Register,
                    sample_count: 10,
                },
                vec![71.0, 69.0, 73.0, 70.0, 72.0, 68.0, 71.5, 70.5, 69.5, 72.5],
            ),
        ];

        let result = compute_bias_test("ACS", &groups);
        assert!(!result.bias_detected, "Similar groups should show no bias");
        assert!(result.max_disparity < 5.0);
    }

    #[test]
    fn test_compute_bias_with_disparity() {
        let groups = vec![
            (
                PopulationGroup {
                    id: "a".into(),
                    name: "Group A".into(),
                    category: GroupCategory::Register,
                    sample_count: 10,
                },
                vec![80.0, 82.0, 78.0, 81.0, 83.0, 79.0, 80.5, 81.5, 82.5, 78.5],
            ),
            (
                PopulationGroup {
                    id: "b".into(),
                    name: "Group B".into(),
                    category: GroupCategory::Register,
                    sample_count: 10,
                },
                vec![55.0, 58.0, 52.0, 56.0, 54.0, 57.0, 53.0, 55.5, 56.5, 54.5],
            ),
        ];

        let result = compute_bias_test("ACS", &groups);
        assert!(result.bias_detected, "Groups with 25-point gap should show bias");
        assert!(result.max_disparity > 20.0);
    }

    #[test]
    fn test_documented_failures() {
        let failures = standard_documented_failures();
        assert!(failures.len() >= 4);
        assert!(failures.iter().all(|f| !f.mitigation.is_empty()));
    }

    #[test]
    fn test_audit_template() {
        let audit = generate_audit_template();
        assert!(audit.groups_tested.len() >= 10);
        assert!(audit.documented_failures.len() >= 4);
        assert!(!audit.limitations.is_empty());
    }
}
