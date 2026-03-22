//! Anomaly Flag System — passage-level, localized anomaly detection.
//!
//! Replaces binary AI/human classification with specific, localized anomaly flags.
//! The system should NEVER say "this document is 78% AI". It SHOULD say
//! "paragraphs 12-15, 23-28, and 41-47 show clustered anomalies."

use serde::{Deserialize, Serialize};

use crate::analysis::baselines::{MetricAssessment, RegisterBaselineReport};
use crate::forensics::docx::profile::DocumentConstructionProfile;

/// A localized anomaly flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFlag {
    /// Type of anomaly.
    pub anomaly_type: AnomalyType,
    /// Location in the document (paragraph range, if applicable).
    pub location: AnomalyLocation,
    /// Severity level.
    pub severity: AnomalySeverity,
    /// Human-readable description (framed as question, not accusation).
    pub description: String,
    /// Recommended action for the evaluator.
    pub recommended_action: String,
    /// Which detection methods contributed to this flag.
    pub contributing_signals: Vec<String>,
}

/// Types of anomalies that can be flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Bulk text insertion detected via RSID analysis.
    Paste,
    /// Vocabulary or style inconsistency.
    Vocabulary,
    /// Formatting differs from document baseline.
    Formatting,
    /// Temporal metrics are disproportionate.
    Temporal,
    /// Style deviates from writer's established pattern.
    Style,
    /// Register inconsistency across sections.
    Register,
}

impl AnomalyType {
    pub fn label(&self) -> &'static str {
        match self {
            AnomalyType::Paste => "Paste Anomaly",
            AnomalyType::Vocabulary => "Vocabulary Anomaly",
            AnomalyType::Formatting => "Formatting Anomaly",
            AnomalyType::Temporal => "Temporal Anomaly",
            AnomalyType::Style => "Style Anomaly",
            AnomalyType::Register => "Register Anomaly",
        }
    }
}

/// Where in the document the anomaly was detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyLocation {
    /// Document-wide anomaly.
    DocumentLevel,
    /// Specific paragraph range.
    ParagraphRange { start: usize, end: usize },
    /// Specific section.
    Section(String),
}

/// Severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
}

impl AnomalySeverity {
    pub fn label(&self) -> &'static str {
        match self {
            AnomalySeverity::Low => "Low",
            AnomalySeverity::Medium => "Medium",
            AnomalySeverity::High => "High",
        }
    }
}

/// Complete anomaly report for a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    /// All flagged anomalies.
    pub flags: Vec<AnomalyFlag>,
    /// Total count by severity.
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    /// Summary text.
    pub summary: String,
}

/// Collect anomaly flags from all available evidence sources.
pub fn collect_anomalies(
    docx_profile: Option<&DocumentConstructionProfile>,
    baseline_report: Option<&RegisterBaselineReport>,
) -> AnomalyReport {
    let mut flags = Vec::new();

    // Collect from DOCX forensics
    if let Some(profile) = docx_profile {
        collect_forensic_anomalies(profile, &mut flags);
    }

    // Collect from register baseline comparisons
    if let Some(report) = baseline_report {
        collect_baseline_anomalies(report, &mut flags);
    }

    // Sort by severity (High first)
    flags.sort_by(|a, b| b.severity.cmp(&a.severity));

    let high_count = flags.iter().filter(|f| f.severity == AnomalySeverity::High).count();
    let medium_count = flags.iter().filter(|f| f.severity == AnomalySeverity::Medium).count();
    let low_count = flags.iter().filter(|f| f.severity == AnomalySeverity::Low).count();

    let summary = if flags.is_empty() {
        "No anomalies detected in the available evidence.".to_string()
    } else {
        let regions: Vec<String> = flags
            .iter()
            .filter(|f| matches!(f.location, AnomalyLocation::ParagraphRange { .. }))
            .filter(|f| f.severity >= AnomalySeverity::Medium)
            .map(|f| match &f.location {
                AnomalyLocation::ParagraphRange { start, end } => {
                    format!("paragraphs {start}-{end}")
                }
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect();

        if regions.is_empty() {
            format!(
                "{} anomal{} detected ({} high, {} medium, {} low).",
                flags.len(),
                if flags.len() == 1 { "y" } else { "ies" },
                high_count,
                medium_count,
                low_count,
            )
        } else {
            format!(
                "{} anomal{} detected. Regions of interest: {}.",
                flags.len(),
                if flags.len() == 1 { "y" } else { "ies" },
                regions.join(", "),
            )
        }
    };

    AnomalyReport {
        flags,
        high_count,
        medium_count,
        low_count,
        summary,
    }
}

fn collect_forensic_anomalies(
    profile: &DocumentConstructionProfile,
    flags: &mut Vec<AnomalyFlag>,
) {
    // RSID-based anomalies
    if let Some(ref rsid) = profile.rsid_analysis {
        for paste in &rsid.paste_events {
            if paste.is_anomalous {
                flags.push(AnomalyFlag {
                    anomaly_type: AnomalyType::Paste,
                    location: AnomalyLocation::ParagraphRange {
                        start: paste.start_paragraph,
                        end: paste.end_paragraph,
                    },
                    severity: if paste.block_size > 15 {
                        AnomalySeverity::High
                    } else {
                        AnomalySeverity::Medium
                    },
                    description: format!(
                        "Paragraphs {}-{} share a single RSID with no subsequent revision, \
                         suggesting bulk text insertion ({} paragraphs)",
                        paste.start_paragraph, paste.end_paragraph, paste.block_size
                    ),
                    recommended_action: "Discuss this section with the author".to_string(),
                    contributing_signals: vec!["RSID analysis".to_string()],
                });
            }
        }
    }

    // Formatting anomalies
    if let Some(ref fmt) = profile.formatting_analysis {
        for anomaly in &fmt.anomalies {
            flags.push(AnomalyFlag {
                anomaly_type: AnomalyType::Formatting,
                location: AnomalyLocation::ParagraphRange {
                    start: anomaly.start_paragraph,
                    end: anomaly.end_paragraph,
                },
                severity: AnomalySeverity::Medium,
                description: anomaly.details.clone(),
                recommended_action: "Check if this section was pasted from an external source"
                    .to_string(),
                contributing_signals: vec!["Formatting analysis".to_string()],
            });
        }
    }

    // Metadata-based temporal anomalies
    for anomaly in &profile.anomalies {
        use crate::forensics::docx::profile::ForensicAnomalyType;
        let anomaly_type = match anomaly.anomaly_type {
            ForensicAnomalyType::Temporal | ForensicAnomalyType::Metadata => AnomalyType::Temporal,
            ForensicAnomalyType::Rsid => AnomalyType::Paste,
            ForensicAnomalyType::Formatting => AnomalyType::Formatting,
            ForensicAnomalyType::Structural => AnomalyType::Style,
        };

        let severity = match anomaly.severity {
            crate::forensics::docx::profile::ForensicSeverity::Low => AnomalySeverity::Low,
            crate::forensics::docx::profile::ForensicSeverity::Medium => AnomalySeverity::Medium,
            crate::forensics::docx::profile::ForensicSeverity::High => AnomalySeverity::High,
        };

        let location = anomaly
            .location
            .map(|(s, e)| AnomalyLocation::ParagraphRange { start: s, end: e })
            .unwrap_or(AnomalyLocation::DocumentLevel);

        flags.push(AnomalyFlag {
            anomaly_type,
            location,
            severity,
            description: anomaly.description.clone(),
            recommended_action: "Request additional process evidence".to_string(),
            contributing_signals: vec!["Document metadata".to_string()],
        });
    }
}

fn collect_baseline_anomalies(
    report: &RegisterBaselineReport,
    flags: &mut Vec<AnomalyFlag>,
) {
    // Only flag metrics that are significantly anomalous
    let anomalous_metrics: Vec<_> = report
        .comparisons
        .iter()
        .filter(|c| c.assessment == MetricAssessment::Anomalous)
        .collect();

    if anomalous_metrics.len() >= 3 {
        // Multiple anomalous metrics = register inconsistency
        let metric_names: Vec<&str> = anomalous_metrics
            .iter()
            .map(|m| m.metric_name.as_str())
            .collect();

        flags.push(AnomalyFlag {
            anomaly_type: AnomalyType::Register,
            location: AnomalyLocation::DocumentLevel,
            severity: AnomalySeverity::Medium,
            description: format!(
                "Multiple text metrics ({}) fall significantly outside expected range \
                 for {} writing",
                metric_names.join(", "),
                report.register_label,
            ),
            recommended_action: "Consider whether the text type was correctly identified"
                .to_string(),
            contributing_signals: vec!["Register baseline comparison".to_string()],
        });
    }

    // Individual severe anomalies
    for metric in &anomalous_metrics {
        if metric.z_score.abs() > 3.0 {
            flags.push(AnomalyFlag {
                anomaly_type: AnomalyType::Vocabulary,
                location: AnomalyLocation::DocumentLevel,
                severity: AnomalySeverity::Low,
                description: metric.explanation.clone(),
                recommended_action: "This metric warrants further examination".to_string(),
                contributing_signals: vec![format!(
                    "Register baseline (z={:.1})",
                    metric.z_score
                )],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::docx::profile::{ConstructionPattern, DocumentConstructionProfile};
    use crate::forensics::docx::rsid::{PasteEvent, RsidAnalysis};
    use std::collections::HashMap;

    fn mock_profile_with_paste() -> DocumentConstructionProfile {
        DocumentConstructionProfile {
            metadata: None,
            rsid_analysis: Some(RsidAnalysis {
                paragraph_rsids: Vec::new(),
                run_rsids: Vec::new(),
                settings_rsids: Vec::new(),
                rsid_to_paragraphs: HashMap::new(),
                rsid_diversity: 0.1,
                unique_rsid_count: 3,
                total_paragraphs: 30,
                rsid_blocks: Vec::new(),
                largest_block_size: 20,
                progression_correlation: None,
                revision_scatter: 0.3,
                paste_events: vec![PasteEvent {
                    start_paragraph: 5,
                    end_paragraph: 24,
                    rsid: "00A11111".into(),
                    block_size: 20,
                    is_anomalous: true,
                }],
                anomalies: Vec::new(),
            }),
            formatting_analysis: None,
            structural_forensics: None,
            construction_pattern: ConstructionPattern::Hybrid,
            anomalies: Vec::new(),
            process_integrity_score: 50.0,
            assessment_text: "Hybrid".into(),
            editing_velocity: None,
            saves_per_hour: None,
            creation_to_modification_hours: None,
            words_per_save: None,
        }
    }

    #[test]
    fn test_anomaly_localization() {
        let profile = mock_profile_with_paste();
        let report = collect_anomalies(Some(&profile), None);

        assert!(!report.flags.is_empty());
        let paste_flag = report
            .flags
            .iter()
            .find(|f| f.anomaly_type == AnomalyType::Paste)
            .expect("Should find paste anomaly");

        match &paste_flag.location {
            AnomalyLocation::ParagraphRange { start, end } => {
                assert_eq!(*start, 5);
                assert_eq!(*end, 24);
            }
            _ => panic!("Expected paragraph range"),
        }
    }

    #[test]
    fn test_anomaly_severity() {
        let profile = mock_profile_with_paste();
        let report = collect_anomalies(Some(&profile), None);

        // 20-paragraph paste should be high severity
        let paste_flag = report
            .flags
            .iter()
            .find(|f| f.anomaly_type == AnomalyType::Paste)
            .unwrap();
        assert_eq!(paste_flag.severity, AnomalySeverity::High);
    }

    #[test]
    fn test_anomaly_framing() {
        let profile = mock_profile_with_paste();
        let report = collect_anomalies(Some(&profile), None);

        // Verify no flag uses accusation language
        for flag in &report.flags {
            assert!(
                !flag.description.to_lowercase().contains("ai-generated"),
                "Anomaly description should not say 'AI-generated': {}",
                flag.description
            );
            assert!(
                !flag.description.to_lowercase().contains("cheating"),
                "Anomaly description should not say 'cheating': {}",
                flag.description
            );
        }
    }

    #[test]
    fn test_no_anomalies() {
        let report = collect_anomalies(None, None);
        assert!(report.flags.is_empty());
        assert!(report.summary.contains("No anomalies"));
    }

    #[test]
    fn test_summary_includes_regions() {
        let profile = mock_profile_with_paste();
        let report = collect_anomalies(Some(&profile), None);
        assert!(
            report.summary.contains("paragraphs"),
            "Summary should mention paragraph ranges: {}",
            report.summary
        );
    }
}
