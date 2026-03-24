//! Hedge-to-intensifier ratio for AI detection.
//!
//! Measures the balance between hedging language (uncertainty markers) and
//! intensifiers (emphasis markers). Human writers hedge more relative to
//! intensifying; AI tends to over-intensify.
//!
//! Human range: 1.2–1.8 | AI range: 0.6–0.9 | Cohen's d ≈ 0.76

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

/// Result of hedge-to-intensifier ratio analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgeRatioResult {
    /// The ratio: hedge_count / intensifier_count.
    /// Higher = more hedging relative to intensifying (more human-like).
    pub ratio: f64,
    /// Raw hedge word count.
    pub hedge_count: usize,
    /// Raw intensifier count.
    pub intensifier_count: usize,
    /// Hedge density (hedges per 1000 words).
    pub hedge_density: f64,
    /// Intensifier density (intensifiers per 1000 words).
    pub intensifier_density: f64,
    /// Total words analyzed.
    pub total_words: usize,
}

/// Hedge words: markers of uncertainty, tentativeness, or qualification.
/// Wired from the existing stylometric.rs list, extended with additional markers.
const HEDGE_WORDS: &[&str] = &[
    "perhaps", "maybe", "possibly", "probably", "presumably",
    "apparently", "seemingly", "supposedly", "arguably",
    "somewhat", "rather", "fairly", "quite", "relatively",
    "roughly", "approximately", "generally", "typically",
    "usually", "often", "sometimes", "occasionally",
    "might", "could", "may", "seem", "seems", "seemed",
    "appear", "appears", "appeared", "suggest", "suggests",
    "tend", "tends", "tended",
    "likely", "unlikely", "possible", "impossible",
    "certain", "uncertain", "unclear",
    "almost", "nearly", "virtually", "essentially",
    "basically", "fundamentally",
];

/// Intensifiers: emphasis markers.
/// Wired from the existing stylometric.rs list.
const INTENSIFIERS: &[&str] = &[
    "very", "extremely", "incredibly", "remarkably", "exceptionally",
    "absolutely", "completely", "totally", "entirely", "utterly",
    "thoroughly", "perfectly", "genuinely", "truly", "really",
    "deeply", "highly", "greatly", "strongly", "firmly",
    "particularly", "especially", "specifically",
    "definitely", "certainly", "surely", "clearly", "obviously",
    "undoubtedly", "unquestionably",
    "so", "too", "quite", "awfully", "terribly",
];

/// Compute the hedge-to-intensifier ratio for the given text.
///
/// Returns `None` if the text has no hedges AND no intensifiers (nothing to measure).
pub fn analyze(text: &str) -> Option<HedgeRatioResult> {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    let total_words = words.len();
    if total_words == 0 {
        return None;
    }

    let hedge_set: HashSet<&str> = HEDGE_WORDS.iter().copied().collect();
    let intensifier_set: HashSet<&str> = INTENSIFIERS.iter().copied().collect();

    let hedge_count = words.iter().filter(|w| hedge_set.contains(w.as_str())).count();
    let intensifier_count = words.iter().filter(|w| intensifier_set.contains(w.as_str())).count();

    if hedge_count == 0 && intensifier_count == 0 {
        return None;
    }

    // Ratio: hedge_count / intensifier_count
    // If no intensifiers, use a high sentinel (all hedges, no intensifiers = very human)
    let ratio = if intensifier_count == 0 {
        hedge_count as f64 * 2.0 // Cap at 2× hedge count to avoid infinity
    } else {
        hedge_count as f64 / intensifier_count as f64
    };

    let hedge_density = hedge_count as f64 / total_words as f64 * 1000.0;
    let intensifier_density = intensifier_count as f64 / total_words as f64 * 1000.0;

    Some(HedgeRatioResult {
        ratio,
        hedge_count,
        intensifier_count,
        hedge_density,
        intensifier_density,
        total_words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_like_text() {
        // Human-like: lots of hedging, few intensifiers
        let text = "Perhaps the results suggest that the approach might possibly \
                     work, though it seems somewhat unlikely. The data appears to \
                     generally indicate a trend, but probably requires further study. \
                     The findings could arguably be interpreted differently. \
                     This is very important and truly remarkable.";
        let result = analyze(text).unwrap();
        assert!(result.ratio > 1.0, "Human text should have high hedge ratio: {}", result.ratio);
    }

    #[test]
    fn test_ai_like_text() {
        // AI-like: heavy intensifiers, few hedges
        let text = "This is absolutely incredible and truly remarkable. \
                     The results are extremely impressive and completely \
                     groundbreaking. It is undoubtedly the very best approach, \
                     perfectly designed and thoroughly tested. The outcome is \
                     definitely outstanding and clearly superior in every way.";
        let result = analyze(text).unwrap();
        assert!(result.ratio < 1.0, "AI text should have low hedge ratio: {}", result.ratio);
    }

    #[test]
    fn test_empty_text() {
        assert!(analyze("").is_none());
    }

    #[test]
    fn test_no_markers() {
        let text = "The cat sat on the mat. Dogs run in the park.";
        assert!(analyze(text).is_none());
    }
}
