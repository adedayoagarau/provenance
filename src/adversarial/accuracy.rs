//! Accuracy reporting — structured accuracy metrics with per-signal breakdown.
//!
//! Produces transparent, publishable accuracy reports following the Phase 23.4
//! specification: per-signal before/after attack scores, signal robustness matrix,
//! and quarterly comparison capability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::evasion::{self, SignalCategory};

/// A complete accuracy report for the Provenance system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyReport {
    /// Report metadata.
    pub metadata: ReportMetadata,
    /// Baseline accuracy (no adversarial attacks).
    pub baseline: BaselineAccuracy,
    /// Robustness matrix: per-attack, per-signal performance.
    pub robustness: evasion::RobustnessMatrix,
    /// Bias audit summary.
    pub bias_summary: BiasSummary,
    /// Comparison against competitors (when data available).
    pub competitor_comparison: Option<CompetitorComparison>,
    /// Known limitations (always present, always honest).
    pub limitations: Vec<String>,
    /// Changelog from previous report (if applicable).
    pub changelog: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// Report generation date (ISO 8601).
    pub date: String,
    /// Provenance version.
    pub software_version: String,
    /// Report version (for quarterly tracking).
    pub report_version: String,
    /// Dataset description.
    pub dataset_description: String,
    /// Total samples tested.
    pub total_samples: usize,
}

/// Baseline accuracy without adversarial attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineAccuracy {
    /// True positive rate: correctly identifies human-written as human.
    pub true_positive_rate: f64,
    /// False positive rate: incorrectly flags human-written as suspicious.
    pub false_positive_rate: f64,
    /// True negative rate: correctly identifies AI-generated as anomalous.
    pub true_negative_rate: f64,
    /// False negative rate: misses AI-generated text.
    pub false_negative_rate: f64,
    /// Overall accuracy.
    pub accuracy: f64,
    /// F1 score.
    pub f1_score: f64,
    /// Per-register breakdown.
    pub per_register: HashMap<String, RegisterAccuracy>,
}

/// Accuracy for a specific register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAccuracy {
    pub register: String,
    pub sample_count: usize,
    pub accuracy: f64,
    pub false_positive_rate: f64,
    pub false_negative_rate: f64,
}

/// Summary of bias audit findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasSummary {
    /// Overall fairness score (0.0-1.0).
    pub fairness_score: f64,
    /// Number of metrics tested.
    pub metrics_tested: usize,
    /// Number showing significant bias.
    pub metrics_with_bias: usize,
    /// Key findings.
    pub key_findings: Vec<String>,
}

/// Comparison against competitor tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorComparison {
    /// Tools compared.
    pub tools: Vec<CompetitorResult>,
    /// Dataset used for comparison.
    pub dataset_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorResult {
    pub tool_name: String,
    pub accuracy: f64,
    pub false_positive_rate: f64,
    pub false_negative_rate: f64,
    pub notes: String,
}

/// Per-signal accuracy breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAccuracy {
    pub signal: SignalCategory,
    pub individual_accuracy: f64,
    pub contribution_to_overall: f64,
    pub robustness_score: f64,
}

/// Generate a template accuracy report (to be populated with real data).
pub fn generate_template_report() -> AccuracyReport {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    AccuracyReport {
        metadata: ReportMetadata {
            date: crate::scoring::engine::format_epoch_public(now),
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            report_version: "v0.1-template".into(),
            dataset_description: "Requires Phase 12.5 evaluation dataset".into(),
            total_samples: 0,
        },
        baseline: BaselineAccuracy {
            true_positive_rate: 0.0,
            false_positive_rate: 0.0,
            true_negative_rate: 0.0,
            false_negative_rate: 0.0,
            accuracy: 0.0,
            f1_score: 0.0,
            per_register: HashMap::new(),
        },
        robustness: evasion::robustness_matrix(),
        bias_summary: BiasSummary {
            fairness_score: 0.0,
            metrics_tested: 0,
            metrics_with_bias: 0,
            key_findings: vec![
                "Bias audit requires evaluation dataset (Phase 12.5) for empirical results".into(),
            ],
        },
        competitor_comparison: None,
        limitations: standard_limitations(),
        changelog: Vec::new(),
    }
}

/// Standard limitations that must always be reported.
pub fn standard_limitations() -> Vec<String> {
    vec![
        "Provenance does not determine whether text was 'written by AI' or 'written by a human'. It evaluates process evidence and stylometric patterns.".into(),
        "Text-only signals (without DOCX forensics or process capture) have limited reliability and are susceptible to humanizer tools.".into(),
        "Short texts (<300 words) produce preliminary assessments with wide confidence intervals.".into(),
        "Non-English text is not supported for text-level analysis. Forensic signals remain reliable.".into(),
        "Register classification uncertainty reduces baseline comparison reliability.".into(),
        "Accuracy metrics are measured on the evaluation dataset and may not generalize to all text types.".into(),
        "Humanizer tools are evolving rapidly. Robustness scores reflect testing at report date only.".into(),
        "ESL writers may experience higher false positive rates on text-level signals. Forensic signals are unaffected.".into(),
    ]
}

/// Render an accuracy report as human-readable text.
pub fn render_text(report: &AccuracyReport) -> String {
    let mut out = String::new();

    out.push_str("═══════════════════════════════════════════════════════\n");
    out.push_str("  PROVENANCE ACCURACY REPORT\n");
    out.push_str("═══════════════════════════════════════════════════════\n\n");

    out.push_str(&format!("  Date:     {}\n", report.metadata.date));
    out.push_str(&format!("  Version:  {}\n", report.metadata.software_version));
    out.push_str(&format!("  Report:   {}\n", report.metadata.report_version));
    out.push_str(&format!("  Samples:  {}\n\n", report.metadata.total_samples));

    // Baseline accuracy
    out.push_str("── Baseline Accuracy ──\n");
    if report.baseline.accuracy > 0.0 {
        out.push_str(&format!("  Accuracy:     {:.1}%\n", report.baseline.accuracy * 100.0));
        out.push_str(&format!("  F1 Score:     {:.3}\n", report.baseline.f1_score));
        out.push_str(&format!("  FPR:          {:.1}%\n", report.baseline.false_positive_rate * 100.0));
        out.push_str(&format!("  FNR:          {:.1}%\n\n", report.baseline.false_negative_rate * 100.0));
    } else {
        out.push_str("  (Requires evaluation dataset — see Phase 12.5)\n\n");
    }

    // Robustness matrix
    out.push_str("── Robustness Matrix ──\n");
    out.push_str(&format!(
        "  Overall robustness: {:.0}% across {} scenarios\n\n",
        report.robustness.overall_score * 100.0,
        report.robustness.scenario_count
    ));

    out.push_str("  Attack                      | Overall | Forensic | Stylometric | Process\n");
    out.push_str("  ────────────────────────────|─────────|──────────|─────────────|────────\n");

    for entry in &report.robustness.entries {
        let forensic = entry
            .signal_robustness
            .get(&SignalCategory::DocxForensic)
            .copied()
            .unwrap_or(0.0);
        let stylometric = entry
            .signal_robustness
            .get(&SignalCategory::Stylometric)
            .copied()
            .unwrap_or(0.0);
        let process = entry
            .signal_robustness
            .get(&SignalCategory::ProcessCapture)
            .copied()
            .unwrap_or(0.0);

        out.push_str(&format!(
            "  {:<29}| {:>5.0}%  | {:>6.0}%  | {:>9.0}%  | {:>5.0}%\n",
            &entry.scenario_name[..entry.scenario_name.len().min(29)],
            entry.overall_robustness * 100.0,
            forensic * 100.0,
            stylometric * 100.0,
            process * 100.0,
        ));
    }
    out.push('\n');

    // Bias summary
    out.push_str("── Bias Audit ──\n");
    if report.bias_summary.metrics_tested > 0 {
        out.push_str(&format!(
            "  Fairness: {:.0}% | Metrics tested: {} | Metrics with bias: {}\n",
            report.bias_summary.fairness_score * 100.0,
            report.bias_summary.metrics_tested,
            report.bias_summary.metrics_with_bias,
        ));
    }
    for finding in &report.bias_summary.key_findings {
        out.push_str(&format!("  • {finding}\n"));
    }
    out.push('\n');

    // Limitations
    out.push_str("── Known Limitations ──\n");
    for limitation in &report.limitations {
        out.push_str(&format!("  • {limitation}\n"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_report() {
        let report = generate_template_report();
        assert!(!report.metadata.software_version.is_empty());
        assert!(report.robustness.scenario_count >= 10);
        assert!(report.limitations.len() >= 5);
    }

    #[test]
    fn test_standard_limitations() {
        let limitations = standard_limitations();
        assert!(limitations.len() >= 5);
        // Must never claim to detect "AI-generated" text
        for limitation in &limitations {
            assert!(!limitation.contains("AI-generated text"));
        }
    }

    #[test]
    fn test_render_report() {
        let report = generate_template_report();
        let text = render_text(&report);
        assert!(text.contains("ACCURACY REPORT"));
        assert!(text.contains("Robustness Matrix"));
        assert!(text.contains("Known Limitations"));
    }
}
