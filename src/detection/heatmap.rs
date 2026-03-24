//! Per-section heatmap for AI detection.
//!
//! Sliding window (300 words, slide 75) producing per-window AI likelihood scores.
//! Each window includes: score, confidence, top 3 contributing features.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::analysis::register::Register;

use super::scoring;

/// A single heatmap window with its AI score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapWindow {
    /// Window index (0-based).
    pub index: usize,
    /// Starting word position in the original text.
    pub word_start: usize,
    /// Ending word position (exclusive).
    pub word_end: usize,
    /// AI likelihood score for this window (0.0–1.0).
    pub score: f64,
    /// Confidence in this window's score.
    pub confidence: f64,
    /// Top contributing features for this window.
    pub top_features: Vec<HeatmapFeature>,
    /// Text excerpt (first 80 chars of the window).
    pub excerpt: String,
}

/// A contributing feature in a heatmap window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapFeature {
    pub name: String,
    pub contribution: f64,
}

/// Complete heatmap result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapResult {
    /// Per-window scores.
    pub windows: Vec<HeatmapWindow>,
    /// Window size in words.
    pub window_size: usize,
    /// Slide step in words.
    pub slide_step: usize,
    /// Overall min/max/mean across windows.
    pub min_score: f64,
    pub max_score: f64,
    pub mean_score: f64,
}

const WINDOW_SIZE: usize = 300;
const SLIDE_STEP: usize = 75;

/// Generate a per-section heatmap for the text.
///
/// Returns `None` if text is shorter than one window.
pub fn analyze(text: &str, register: &Register) -> Option<HeatmapResult> {
    let all_words: Vec<&str> = text.unicode_words().collect();

    if all_words.len() < WINDOW_SIZE {
        return None;
    }

    let mut windows: Vec<HeatmapWindow> = Vec::new();
    let mut pos = 0;

    while pos + WINDOW_SIZE <= all_words.len() {
        let window_words = &all_words[pos..pos + WINDOW_SIZE];
        let window_text = window_words.join(" ");

        // Extract features for this window
        let features = super::extract_features(&window_text);
        let score_result = scoring::score(&features, register);

        // Top 3 features by weighted contribution
        let mut sorted_features = score_result.feature_scores.clone();
        sorted_features.sort_by(|a, b| {
            b.weighted_contribution
                .abs()
                .partial_cmp(&a.weighted_contribution.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_features: Vec<HeatmapFeature> = sorted_features
            .iter()
            .take(3)
            .map(|f| HeatmapFeature {
                name: f.name.clone(),
                contribution: f.weighted_contribution,
            })
            .collect();

        // Excerpt: first 80 chars
        let excerpt: String = window_text.chars().take(80).collect();

        windows.push(HeatmapWindow {
            index: windows.len(),
            word_start: pos,
            word_end: pos + WINDOW_SIZE,
            score: score_result.adjusted_score,
            confidence: score_result.confidence,
            top_features,
            excerpt,
        });

        pos += SLIDE_STEP;
    }

    if windows.is_empty() {
        return None;
    }

    let scores: Vec<f64> = windows.iter().map(|w| w.score).collect();
    let min_score = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean_score = scores.iter().sum::<f64>() / scores.len() as f64;

    Some(HeatmapResult {
        windows,
        window_size: WINDOW_SIZE,
        slide_step: SLIDE_STEP,
        min_score,
        max_score,
        mean_score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::register::CasualSubtype;

    #[test]
    fn test_short_text_returns_none() {
        let register = Register::Casual(CasualSubtype::BlogPost);
        assert!(analyze("Too short.", &register).is_none());
    }

    #[test]
    fn test_heatmap_generation() {
        let text = "The quick brown fox jumps over the lazy dog and returns home. \
                     A wonderful serenity has taken possession of my entire soul. \
                     I am alone and feel the charm of existence in this spot. "
            .repeat(40);
        let register = Register::Casual(CasualSubtype::BlogPost);
        let result = analyze(&text, &register);
        assert!(result.is_some());
        let heatmap = result.unwrap();
        assert!(!heatmap.windows.is_empty());
        assert!(heatmap.min_score <= heatmap.max_score);
    }
}
