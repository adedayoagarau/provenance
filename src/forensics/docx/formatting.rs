use super::parser::{ParsedParagraph, StyleDefaults};
use serde::{Deserialize, Serialize};

/// A formatting anomaly detected in a run or paragraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingAnomaly {
    pub start_paragraph: usize,
    pub end_paragraph: usize,
    pub anomaly_type: FormattingAnomalyType,
    pub details: String,
}

/// Types of formatting anomalies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormattingAnomalyType {
    NonBaselineFont,
    NonBaselineFontSize,
    InconsistentLanguage,
    InlineFormattingOverride,
}

/// Results of formatting consistency analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingAnalysis {
    /// Fraction of runs matching the baseline formatting (0.0 - 1.0)
    pub formatting_consistency_score: f64,
    /// Total runs analyzed
    pub total_runs: usize,
    /// Runs matching baseline
    pub runs_matching_baseline: usize,
    /// Baseline description
    pub baseline_font: Option<String>,
    pub baseline_font_size: Option<u32>,
    pub baseline_language: Option<String>,
    /// Detected anomaly clusters
    pub anomalies: Vec<FormattingAnomaly>,
}

impl FormattingAnalysis {
    /// Analyze formatting consistency across all paragraphs.
    pub fn analyze(paragraphs: &[ParsedParagraph], defaults: &StyleDefaults) -> Self {
        let mut total_runs = 0usize;
        let mut matching_runs = 0usize;

        // Track per-paragraph font deviations for clustering
        let mut para_deviations: Vec<(usize, Vec<FormattingAnomalyType>)> = Vec::new();

        for (para_idx, para) in paragraphs.iter().enumerate() {
            let mut para_anomaly_types = Vec::new();

            for run in &para.runs {
                if run.text.trim().is_empty() {
                    continue;
                }
                total_runs += 1;
                let mut matches = true;

                // Check font
                if let Some(ref run_font) = run.font_name {
                    if let Some(ref default_font) = defaults.default_font {
                        if !fonts_match(run_font, default_font) {
                            matches = false;
                            para_anomaly_types.push(FormattingAnomalyType::NonBaselineFont);
                        }
                    }
                }

                // Check font size
                if let Some(run_size) = run.font_size {
                    if let Some(default_size) = defaults.default_font_size {
                        if run_size != default_size {
                            matches = false;
                            para_anomaly_types.push(FormattingAnomalyType::NonBaselineFontSize);
                        }
                    }
                }

                // Check language
                if let Some(ref run_lang) = run.language {
                    if let Some(ref default_lang) = defaults.default_language {
                        if run_lang != default_lang {
                            matches = false;
                            para_anomaly_types.push(FormattingAnomalyType::InconsistentLanguage);
                        }
                    }
                }

                // Check for inline formatting overrides (bold/italic/color not from styles)
                if run.bold || run.italic || run.color.is_some() {
                    // These are direct formatting overrides — not necessarily anomalous
                    // but worth tracking in context
                    para_anomaly_types.push(FormattingAnomalyType::InlineFormattingOverride);
                }

                if matches {
                    matching_runs += 1;
                }
            }

            if !para_anomaly_types.is_empty() {
                para_deviations.push((para_idx, para_anomaly_types));
            }
        }

        // Cluster anomalies into contiguous ranges
        let anomaly_clusters = cluster_anomalies(&para_deviations);

        let consistency_score = if total_runs > 0 {
            matching_runs as f64 / total_runs as f64
        } else {
            1.0 // No runs to analyze
        };

        FormattingAnalysis {
            formatting_consistency_score: consistency_score,
            total_runs,
            runs_matching_baseline: matching_runs,
            baseline_font: defaults.default_font.clone(),
            baseline_font_size: defaults.default_font_size,
            baseline_language: defaults.default_language.clone(),
            anomalies: anomaly_clusters,
        }
    }
}

/// Check if two font names match (case-insensitive, ignoring minor variations).
fn fonts_match(a: &str, b: &str) -> bool {
    let normalize = |s: &str| s.to_lowercase().replace(' ', "");
    normalize(a) == normalize(b)
}

/// Cluster per-paragraph anomalies into contiguous ranges.
fn cluster_anomalies(
    para_deviations: &[(usize, Vec<FormattingAnomalyType>)],
) -> Vec<FormattingAnomaly> {
    if para_deviations.is_empty() {
        return Vec::new();
    }

    let mut clusters: Vec<FormattingAnomaly> = Vec::new();
    let mut current_start = para_deviations[0].0;
    let mut current_end = para_deviations[0].0;
    let mut current_types: Vec<FormattingAnomalyType> = Vec::new();

    for &(para_idx, ref types) in para_deviations {
        if para_idx <= current_end + 2 {
            // Within 1 paragraph gap — extend cluster
            current_end = para_idx;
            for t in types {
                if !current_types.contains(t) {
                    current_types.push(t.clone());
                }
            }
        } else {
            // Gap — finalize current cluster and start new one
            if !current_types.is_empty() {
                for anomaly_type in &current_types {
                    clusters.push(FormattingAnomaly {
                        start_paragraph: current_start,
                        end_paragraph: current_end,
                        anomaly_type: anomaly_type.clone(),
                        details: describe_anomaly(anomaly_type, current_start, current_end),
                    });
                }
            }
            current_start = para_idx;
            current_end = para_idx;
            current_types = types.clone();
        }
    }

    // Finalize last cluster
    for anomaly_type in &current_types {
        clusters.push(FormattingAnomaly {
            start_paragraph: current_start,
            end_paragraph: current_end,
            anomaly_type: anomaly_type.clone(),
            details: describe_anomaly(anomaly_type, current_start, current_end),
        });
    }

    clusters
}

fn describe_anomaly(
    anomaly_type: &FormattingAnomalyType,
    start: usize,
    end: usize,
) -> String {
    let range = if start == end {
        format!("Paragraph {start}")
    } else {
        format!("Paragraphs {start}-{end}")
    };
    match anomaly_type {
        FormattingAnomalyType::NonBaselineFont => {
            format!("{range} contain a font that differs from the document baseline")
        }
        FormattingAnomalyType::NonBaselineFontSize => {
            format!("{range} contain a font size that differs from the document baseline")
        }
        FormattingAnomalyType::InconsistentLanguage => {
            format!("{range} contain language tags inconsistent with the document baseline")
        }
        FormattingAnomalyType::InlineFormattingOverride => {
            format!("{range} contain direct formatting overrides (bold, italic, or color not from styles)")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::docx::parser::{ParsedParagraph, ParsedRun, StyleDefaults};

    fn make_run(text: &str, font: Option<&str>, size: Option<u32>) -> ParsedRun {
        ParsedRun {
            text: text.to_string(),
            font_name: font.map(String::from),
            font_size: size,
            ..Default::default()
        }
    }

    #[test]
    fn test_formatting_consistency_all_matching() {
        let defaults = StyleDefaults {
            default_font: Some("Calibri".to_string()),
            default_font_size: Some(22),
            ..Default::default()
        };
        let paragraphs = vec![
            ParsedParagraph {
                runs: vec![
                    make_run("Hello ", Some("Calibri"), Some(22)),
                    make_run("world", Some("Calibri"), Some(22)),
                ],
                ..Default::default()
            },
        ];
        let analysis = FormattingAnalysis::analyze(&paragraphs, &defaults);
        assert_eq!(analysis.formatting_consistency_score, 1.0);
        assert!(analysis.anomalies.is_empty());
    }

    #[test]
    fn test_formatting_consistency_font_mismatch() {
        let defaults = StyleDefaults {
            default_font: Some("Calibri".to_string()),
            default_font_size: Some(22),
            ..Default::default()
        };
        let paragraphs = vec![
            ParsedParagraph {
                runs: vec![
                    make_run("Hello ", Some("Calibri"), Some(22)),
                    make_run("pasted text", Some("Times New Roman"), Some(24)),
                ],
                ..Default::default()
            },
        ];
        let analysis = FormattingAnalysis::analyze(&paragraphs, &defaults);
        assert!(analysis.formatting_consistency_score < 1.0);
        assert!(!analysis.anomalies.is_empty());
    }
}
