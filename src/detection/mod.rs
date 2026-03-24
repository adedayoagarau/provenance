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
//!
//! Pipeline: Text → Register Classification → Feature Extraction →
//!           Register Normalization → Weighted Sigmoid Composite → 5-Tier Classification

pub mod burstiness;
pub mod zipf;
pub mod hedge_ratio;
pub mod autocorrelation;
pub mod pos_entropy;
pub mod interaction;
pub mod scoring;
pub mod heatmap;

use serde::{Deserialize, Serialize};

use crate::analysis;
use crate::analysis::register::{self, RegisterClassification};
use crate::utils::errors::{ProvenanceError, Result};

pub use scoring::{DetectionFeatures, DetectionScore, DetectionTier};
pub use heatmap::HeatmapResult;

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

    // Run the standard analysis pipeline to get register classification
    let analysis_result = analysis::analyze_text(text)?;
    let register_class = register::classify(&analysis_result);

    // Extract detection-specific features
    let features = extract_features(text);

    // Score
    let score = scoring::score(&features, &register_class.primary);

    // Heatmap (only for texts long enough)
    let heatmap = heatmap::analyze(text, &register_class.primary);

    Ok(DetectionResult {
        score,
        heatmap,
        features,
        register: register_class,
        word_count,
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

    let (autocorrelation, (pos_entropy, interaction)) = rayon::join(
        || autocorrelation::analyze(text),
        || rayon::join(|| pos_entropy::analyze(text), || interaction::analyze(text)),
    );

    DetectionFeatures {
        burstiness,
        zipf,
        hedge_ratio,
        autocorrelation,
        pos_entropy,
        interaction,
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
