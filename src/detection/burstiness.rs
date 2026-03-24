//! Burstiness coefficient for AI detection.
//!
//! Measures how "bursty" word usage is across 100-word windows.
//! Human writing has high burstiness (words cluster in specific sections),
//! while AI writing distributes words more uniformly.
//!
//! B = (σ² - μ) / (σ² + μ) where σ² and μ are computed over per-window
//! word frequency counts.
//!
//! Human range: 0.35–0.55 | AI range: 0.15–0.25 | Cohen's d ≈ 1.1

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Result of burstiness analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstinessResult {
    /// Composite burstiness coefficient (mean across content words).
    /// Higher = more bursty (more human-like).
    pub coefficient: f64,
    /// Number of windows analyzed.
    pub window_count: usize,
    /// Number of content words tracked.
    pub tracked_words: usize,
    /// Per-word burstiness values for the top contributors.
    pub top_bursty_words: Vec<(String, f64)>,
}

const WINDOW_SIZE: usize = 100;

/// Function words to exclude — we only measure burstiness on content words.
const FUNCTION_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
    "been", "being", "have", "has", "had", "do", "does", "did", "will",
    "would", "could", "should", "may", "might", "shall", "can", "need",
    "this", "that", "these", "those", "it", "its", "he", "she", "they",
    "we", "you", "i", "me", "him", "her", "us", "them", "my", "your",
    "his", "our", "their", "not", "no", "if", "then", "than", "so",
    "very", "just", "about", "up", "out", "into", "over", "after",
    "also", "which", "who", "what", "when", "where", "how", "all",
    "each", "every", "both", "few", "more", "most", "other", "some",
    "such", "only", "own", "same", "there", "here",
];

/// Compute the burstiness coefficient for the given text.
///
/// Returns `None` if the text has fewer than 2 windows worth of words.
pub fn analyze(text: &str) -> Option<BurstinessResult> {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() > 1)
        .collect();

    if words.len() < WINDOW_SIZE * 2 {
        return None;
    }

    let function_set: std::collections::HashSet<&str> =
        FUNCTION_WORDS.iter().copied().collect();

    // Build windows of WINDOW_SIZE words
    let num_windows = words.len() / WINDOW_SIZE;
    let windows: Vec<&[String]> = (0..num_windows)
        .map(|i| &words[i * WINDOW_SIZE..(i + 1) * WINDOW_SIZE])
        .collect();

    // Count content word frequency in each window
    // word -> vec of per-window counts
    let mut global_freq: HashMap<&str, usize> = HashMap::new();
    for word in &words {
        if !function_set.contains(word.as_str()) {
            *global_freq.entry(word.as_str()).or_insert(0) += 1;
        }
    }

    // Only track words that appear at least 3 times total (need variance signal)
    let tracked: Vec<&str> = global_freq
        .iter()
        .filter(|(_, &count)| count >= 3)
        .map(|(&word, _)| word)
        .collect();

    if tracked.is_empty() {
        return Some(BurstinessResult {
            coefficient: 0.0,
            window_count: num_windows,
            tracked_words: 0,
            top_bursty_words: Vec::new(),
        });
    }

    // Compute per-word burstiness
    let mut word_burstiness: Vec<(&str, f64)> = Vec::with_capacity(tracked.len());

    for &word in &tracked {
        let counts: Vec<f64> = windows
            .iter()
            .map(|window| {
                window.iter().filter(|w| w.as_str() == word).count() as f64
            })
            .collect();

        let b = compute_burstiness(&counts);
        if b.is_finite() {
            word_burstiness.push((word, b));
        }
    }

    if word_burstiness.is_empty() {
        return Some(BurstinessResult {
            coefficient: 0.0,
            window_count: num_windows,
            tracked_words: 0,
            top_bursty_words: Vec::new(),
        });
    }

    // Composite: mean burstiness across all tracked words
    let sum: f64 = word_burstiness.iter().map(|(_, b)| *b).sum();
    let coefficient = sum / word_burstiness.len() as f64;

    // Top 10 most bursty words
    word_burstiness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_bursty_words: Vec<(String, f64)> = word_burstiness
        .iter()
        .take(10)
        .map(|(w, b)| (w.to_string(), *b))
        .collect();

    Some(BurstinessResult {
        coefficient,
        window_count: num_windows,
        tracked_words: word_burstiness.len(),
        top_bursty_words,
    })
}

/// Compute burstiness B = (σ² - μ) / (σ² + μ) for a series of counts.
fn compute_burstiness(counts: &[f64]) -> f64 {
    let n = counts.len() as f64;
    if n < 2.0 {
        return 0.0;
    }

    let mean = counts.iter().sum::<f64>() / n;
    if mean < f64::EPSILON {
        return 0.0;
    }

    let variance = counts.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);

    let denominator = variance + mean;
    if denominator < f64::EPSILON {
        return 0.0;
    }

    (variance - mean) / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_burstiness_formula() {
        // Bursty: word appears in clusters
        let bursty = vec![5.0, 0.0, 0.0, 5.0, 0.0, 0.0];
        let b = compute_burstiness(&bursty);
        assert!(b > 0.0, "Bursty distribution should have positive B: {b}");

        // Uniform: word appears evenly
        let uniform = vec![2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
        let b_uniform = compute_burstiness(&uniform);
        assert!(b_uniform < b, "Uniform should be less bursty than clustered");
    }

    #[test]
    fn test_short_text_returns_none() {
        let short = "This is too short.";
        assert!(analyze(short).is_none());
    }

    #[test]
    fn test_analyze_returns_result_for_long_text() {
        // Generate a text with 300+ words
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(50);
        let result = analyze(&text);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.window_count >= 2);
    }
}
