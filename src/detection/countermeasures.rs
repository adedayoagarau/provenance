//! False positive countermeasure aggregation.
//!
//! Combines NNES detection, template detection, editing tool normalization,
//! humanizer detection, and confidence thresholding into a unified
//! countermeasure report that adjusts the final detection score.

use serde::{Deserialize, Serialize};

/// Aggregated countermeasure report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountermeasureReport {
    /// NNES score adjustment (negative).
    pub nnes_adjustment: f64,
    /// NNES detection confidence.
    pub nnes_confidence: f64,
    /// Template fraction detected.
    pub template_fraction: f64,
    /// Whether template detection resulted in re-scoring.
    pub template_rescored: bool,
    /// Template-based score adjustment (implicit via re-scoring).
    pub template_adjustment: f64,
    /// Editing tool score adjustment (negative).
    pub editing_tool_adjustment: f64,
    /// Editing tool detection confidence.
    pub editing_tool_confidence: f64,
    /// Humanizer artifact score adjustment (positive — increases AI score).
    pub humanizer_adjustment: f64,
    /// Humanizer detection confidence.
    pub humanizer_confidence: f64,
    /// Whether the result should be marked INCONCLUSIVE.
    pub inconclusive: bool,
    /// Reason for inconclusive status (if applicable).
    pub inconclusive_reason: Option<String>,
}

impl Default for CountermeasureReport {
    fn default() -> Self {
        CountermeasureReport {
            nnes_adjustment: 0.0,
            nnes_confidence: 0.0,
            template_fraction: 0.0,
            template_rescored: false,
            template_adjustment: 0.0,
            editing_tool_adjustment: 0.0,
            editing_tool_confidence: 0.0,
            humanizer_adjustment: 0.0,
            humanizer_confidence: 0.0,
            inconclusive: false,
            inconclusive_reason: None,
        }
    }
}

impl CountermeasureReport {
    /// Total score adjustment from all countermeasures.
    pub fn total_adjustment(&self) -> f64 {
        self.nnes_adjustment
            + self.editing_tool_adjustment
            + self.humanizer_adjustment
            + self.template_adjustment
    }

    /// Human-readable summary of adjustments applied.
    pub fn adjustment_summary(&self) -> String {
        let mut parts = Vec::new();

        if self.nnes_adjustment.abs() > f64::EPSILON {
            parts.push(format!("NNES adjustment ({:+.2})", self.nnes_adjustment));
        }
        if self.editing_tool_adjustment.abs() > f64::EPSILON {
            parts.push(format!(
                "editing tool normalization ({:+.2})",
                self.editing_tool_adjustment
            ));
        }
        if self.humanizer_adjustment > f64::EPSILON {
            parts.push(format!(
                "humanizer artifacts detected ({:+.2})",
                self.humanizer_adjustment
            ));
        }
        if self.template_rescored {
            parts.push(format!(
                "template content excluded ({:.0}%)",
                self.template_fraction * 100.0
            ));
        }

        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Run all countermeasures on the given text.
///
/// Returns the countermeasure report and optionally the text to re-score
/// (if template content was stripped).
pub fn run_countermeasures(text: &str, features_used: usize) -> (CountermeasureReport, Option<String>) {
    let mut report = CountermeasureReport::default();

    // 1. NNES detection
    let nnes = super::nnes::analyze(text);
    report.nnes_adjustment = nnes.score_adjustment;
    report.nnes_confidence = nnes.confidence;

    // 2. Template detection
    let template = super::template::analyze(text);
    report.template_fraction = template.template_fraction;
    let rescore_text = if template.detected && template.scorable_text.split_whitespace().count() >= 500 {
        report.template_rescored = true;
        Some(template.scorable_text)
    } else {
        None
    };

    // 3. Editing tool detection
    let editing = super::editing_tools::analyze(text);
    report.editing_tool_adjustment = editing.score_adjustment;
    report.editing_tool_confidence = editing.confidence;

    // 4. Humanizer detection
    let humanizer = crate::adversarial::humanizer::detect_humanizer(text);
    if humanizer.detected {
        report.humanizer_adjustment = 0.15;
        report.humanizer_confidence = humanizer.confidence;
    }

    // 5. Confidence thresholding
    if features_used < 5 {
        report.inconclusive = true;
        report.inconclusive_reason = Some(format!(
            "Only {} of 7 detection features could be computed. Minimum 5 required for reliable classification.",
            features_used
        ));
    }

    (report, rescore_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_report() {
        let report = CountermeasureReport::default();
        assert_eq!(report.total_adjustment(), 0.0);
        assert_eq!(report.adjustment_summary(), "none");
        assert!(!report.inconclusive);
    }

    #[test]
    fn test_adjustment_summary() {
        let report = CountermeasureReport {
            nnes_adjustment: -0.15,
            nnes_confidence: 0.5,
            humanizer_adjustment: 0.15,
            humanizer_confidence: 0.6,
            ..Default::default()
        };
        let summary = report.adjustment_summary();
        assert!(summary.contains("NNES"));
        assert!(summary.contains("humanizer"));
    }

    #[test]
    fn test_confidence_thresholding() {
        let (report, _) = run_countermeasures("Some text here that is long enough.", 3);
        assert!(report.inconclusive);
        assert!(report.inconclusive_reason.is_some());
    }
}
