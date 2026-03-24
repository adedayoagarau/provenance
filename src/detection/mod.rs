//! AI writing detection module.
//!
//! Baseline-free AI detection using statistical features that discriminate
//! between human and AI-generated text. No author profile needed.
//!
//! Features (by effect size):
//! - Burstiness coefficient (d ≈ 1.1)
//! - Zipf slope deviation (d ≈ 1.0)
//! - Hedge-to-intensifier ratio (d ≈ 0.76)
//! - Sentence length autocorrelation (d ≈ 0.65)
//! - POS trigram entropy (d ≈ 0.55)
//! - Lexical diversity × sentence length correlation
//! - Content word repetition × text position
//! - Function word clustering uniformity
//!
//! False positive countermeasures:
//! - NNES detection with score adjustment (-0.12 to -0.25)
//! - Template/boilerplate detection (score only non-template sections)
//! - Editing tool normalization (Grammarly/ProWritingAid, -0.15)
//! - Humanizer artifact detection (+0.15)
//! - Confidence thresholding (INCONCLUSIVE when <5 features fire)
//!
//! Pipeline: Text → Template Stripping → Register Classification → Feature Extraction →
//!           Register Normalization → Countermeasures → Weighted Sigmoid Composite →
//!           5-Tier Classification → Explanation

pub mod burstiness;
pub mod zipf;
pub mod hedge_ratio;
pub mod autocorrelation;
pub mod pos_entropy;
pub mod interaction;
pub mod function_word_clustering;
pub mod scoring;
pub mod heatmap;
pub mod nnes;
pub mod template;
pub mod editing_tools;
pub mod countermeasures;
pub mod explanation;
pub mod tier2_features;
pub mod advanced_features;
pub mod modes;

use serde::{Deserialize, Serialize};

use crate::analysis;
use crate::analysis::register::{self, RegisterClassification};
use crate::utils::errors::{ProvenanceError, Result};

pub use scoring::{DetectionFeatures, DetectionScore, DetectionTier};
pub use heatmap::HeatmapResult;
pub use countermeasures::CountermeasureReport;
pub use explanation::DetectionExplanation;
pub use modes::{AnalysisMode, MultiModeResult};

/// Minimum word count for AI detection analysis.
const MIN_WORDS: usize = 500;

/// Complete AI detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// The composite AI detection score.
    pub score: DetectionScore,
    /// Per-section heatmap (if text is long enough).
    pub heatmap: Option<HeatmapResult>,
    /// Raw feature values.
    pub features: DetectionFeatures,
    /// Register classification used for normalization.
    pub register: RegisterClassification,
    /// Word count of the analyzed text.
    pub word_count: usize,
    /// Countermeasure report (NNES, template, editing tools, humanizer).
    pub countermeasures: CountermeasureReport,
    /// Natural language explanation of the result.
    pub explanation: DetectionExplanation,
}

/// Run AI detection on raw text.
///
/// This is the main entry point for the detection module.
/// Returns an error if text has fewer than 500 words.
pub fn detect(text: &str) -> Result<DetectionResult> {
    let word_count = text.split_whitespace().count();
    if word_count < MIN_WORDS {
        return Err(ProvenanceError::InsufficientText {
            min_words: MIN_WORDS,
            actual_words: word_count,
        });
    }

    // Phase 1: Extract features on original text
    let features = extract_features(text);

    // Phase 2: Run countermeasures (may provide re-scored text)
    let (mut cm_report, rescore_text) =
        countermeasures::run_countermeasures(text, features.feature_count());

    // Phase 3: If template content was stripped, re-extract features on clean text
    let (final_features, scoring_text) = if let Some(ref clean_text) = rescore_text {
        let clean_word_count = clean_text.split_whitespace().count();
        if clean_word_count >= MIN_WORDS {
            (extract_features(clean_text), clean_text.as_str())
        } else {
            (features, text)
        }
    } else {
        (features, text)
    };

    // Phase 4: Register classification
    let analysis_result = analysis::analyze_text(scoring_text)?;
    let register_class = register::classify(&analysis_result);

    // Phase 5: Score with register normalization
    let mut score = scoring::score(&final_features, &register_class.primary);

    // Phase 6: Apply countermeasure adjustments
    let cm_adjustment = cm_report.nnes_adjustment
        + cm_report.editing_tool_adjustment
        + cm_report.humanizer_adjustment;
    score.adjusted_score = (score.adjusted_score + cm_adjustment).clamp(0.0, 1.0);

    // Override tier if inconclusive
    if cm_report.inconclusive && score.adjusted_score > 0.15 && score.adjusted_score < 0.85 {
        score.tier = DetectionTier::MixedUncertain;
        score.tier_label = "INCONCLUSIVE".to_string();
        score.action = "Insufficient features for reliable classification — human review required".to_string();
    } else {
        score.tier = DetectionTier::from_score(score.adjusted_score);
        score.tier_label = score.tier.label().to_string();
        score.action = score.tier.action().to_string();
    }

    // Track countermeasure adjustment in score for display
    if cm_adjustment.abs() > f64::EPSILON {
        cm_report.template_adjustment = if rescore_text.is_some() {
            // Template adjustment is implicit (re-scoring)
            0.0
        } else {
            0.0
        };
    }

    // Phase 7: Heatmap
    let heatmap = heatmap::analyze(scoring_text, &register_class.primary);

    // Phase 8: Explanation engine
    let explanation = explanation::explain(&score, &cm_report, word_count);

    Ok(DetectionResult {
        score,
        heatmap,
        features: final_features,
        register: register_class,
        word_count,
        countermeasures: cm_report,
        explanation,
    })
}

/// Run AI detection on a file path.
///
/// Extracts text from the file first, then runs detection.
pub fn detect_file(file_path: &str) -> Result<DetectionResult> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    let text = crate::extraction::extract_text(file_path)?;
    detect(&text)
}

/// Extract all detection features from text.
///
/// Used by both the main detection pipeline and the heatmap windowing.
pub(crate) fn extract_features(text: &str) -> DetectionFeatures {
    // Run all feature extractors in parallel via rayon
    let (burstiness, (zipf, hedge_ratio)) = rayon::join(
        || burstiness::analyze(text),
        || rayon::join(|| zipf::analyze(text), || hedge_ratio::analyze(text)),
    );

    let ((autocorrelation, (pos_entropy, interaction)), (tier2, advanced)) = rayon::join(
        || rayon::join(
            || autocorrelation::analyze(text),
            || rayon::join(|| pos_entropy::analyze(text), || interaction::analyze(text)),
        ),
        || rayon::join(
            || Some(tier2_features::analyze(text)),
            || Some(advanced_features::analyze(text)),
        ),
    );

    DetectionFeatures {
        burstiness,
        zipf,
        hedge_ratio,
        autocorrelation,
        pos_entropy,
        interaction,
        tier2,
        advanced,
    }
}

/// Format a detection result as plain text.
pub fn format_text(result: &DetectionResult) -> String {
    let mut out = String::new();
    let s = &result.score;

    out.push_str("═══════════════════════════════════════════════════════════\n");
    out.push_str("                 AI DETECTION ANALYSIS\n");
    out.push_str("═══════════════════════════════════════════════════════════\n\n");

    out.push_str(&format!("  Result:     {} ({})\n", s.tier_label, s.action));
    out.push_str(&format!("  Score:      {:.2} (adjusted: {:.2})\n", s.score, s.adjusted_score));
    out.push_str(&format!("  Confidence: {:.0}%\n", s.confidence * 100.0));
    out.push_str(&format!("  Register:   {} (adjustment: {:+.2})\n", s.register_label, s.register_adjustment));
    out.push_str(&format!("  Words:      {}\n", result.word_count));

    // Countermeasure adjustments
    let cm = &result.countermeasures;
    let total_cm = cm.total_adjustment();
    if total_cm.abs() > f64::EPSILON || cm.inconclusive || cm.template_rescored {
        out.push_str("\n─── Countermeasure Adjustments ──────────────────────────\n\n");
        if cm.nnes_adjustment.abs() > f64::EPSILON {
            out.push_str(&format!("  NNES detected:     {:+.2} (confidence: {:.0}%)\n",
                cm.nnes_adjustment, cm.nnes_confidence * 100.0));
        }
        if cm.editing_tool_adjustment.abs() > f64::EPSILON {
            out.push_str(&format!("  Editing tools:     {:+.2} (confidence: {:.0}%)\n",
                cm.editing_tool_adjustment, cm.editing_tool_confidence * 100.0));
        }
        if cm.humanizer_adjustment > f64::EPSILON {
            out.push_str(&format!("  Humanizer found:   {:+.2} (confidence: {:.0}%)\n",
                cm.humanizer_adjustment, cm.humanizer_confidence * 100.0));
        }
        if cm.template_rescored {
            out.push_str(&format!("  Template stripped:  {:.0}% boilerplate removed\n",
                cm.template_fraction * 100.0));
        }
        if cm.inconclusive {
            out.push_str(&format!("  INCONCLUSIVE:      {}\n",
                cm.inconclusive_reason.as_deref().unwrap_or("insufficient features")));
        }
    }

    out.push_str("\n─── Feature Breakdown ───────────────────────────────────\n\n");

    for f in &s.feature_scores {
        out.push_str(&format!("  {:50} {:.3} (weight: {:.2}, contribution: {:.3})\n",
            f.name, f.raw_value, f.weight, f.weighted_contribution));
        out.push_str(&format!("    {}\n\n", f.explanation));
    }

    if let Some(ref hm) = result.heatmap {
        out.push_str("─── Section Heatmap ─────────────────────────────────────\n\n");
        out.push_str(&format!("  Windows: {} ({}w, slide {}w)\n",
            hm.windows.len(), hm.window_size, hm.slide_step));
        out.push_str(&format!("  Range: {:.2} – {:.2} (mean: {:.2})\n\n",
            hm.min_score, hm.max_score, hm.mean_score));

        for w in &hm.windows {
            let bar = score_bar(w.score);
            out.push_str(&format!("  [{:3}] {:.2} {} {}\n",
                w.index, w.score, bar, truncate(&w.excerpt, 40)));
        }
        out.push('\n');
    }

    // Explanation
    out.push_str("─── Explanation ─────────────────────────────────────────\n\n");
    out.push_str(&format!("  {}\n", result.explanation.summary));

    if !result.explanation.caveats.is_empty() {
        out.push_str("\n  Caveats:\n");
        for caveat in &result.explanation.caveats {
            out.push_str(&format!("  - {}\n", caveat));
        }
    }

    if !result.explanation.adjustment_explanations.is_empty() {
        out.push_str("\n  Adjustments:\n");
        for adj in &result.explanation.adjustment_explanations {
            out.push_str(&format!("  - {}\n", adj));
        }
    }

    out.push('\n');
    out.push_str("═══════════════════════════════════════════════════════════\n");
    out.push_str("  NOTE: These results are statistical indicators, not\n");
    out.push_str("  definitive proof. Human review is always recommended.\n");
    out.push_str("═══════════════════════════════════════════════════════════\n");

    out
}

/// Format a detection result as JSON.
pub fn format_json(result: &DetectionResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
}

fn score_bar(score: f64) -> String {
    let filled = (score * 20.0).round() as usize;
    let empty = 20 - filled.min(20);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
