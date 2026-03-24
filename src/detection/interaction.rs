//! Interaction features for AI detection.
//!
//! Two cross-feature interactions that capture AI writing patterns:
//!
//! 1. **Lexical diversity × sentence length correlation**: Pearson r between
//!    TTR and mean sentence length across 200-word windows. Human ≈ 0;
//!    AI shows positive correlation (0.2–0.4).
//!
//! 2. **Content word repetition × text position**: Difference in content word
//!    reuse rate between first and second half. Human writing shows high variation;
//!    AI shows ~40% less variation (more uniform repetition across the text).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

/// Combined interaction feature results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionResult {
    /// Pearson r(TTR, mean_sentence_length) across 200-word windows.
    /// Near zero = human-like; 0.2–0.4 = AI-like.
    pub diversity_length_correlation: f64,
    /// Number of windows used for the diversity-length correlation.
    pub diversity_window_count: usize,
    /// Absolute difference in content word reuse rate between first and second half.
    /// Higher = more human-like variation.
    pub repetition_position_delta: f64,
    /// Content word reuse rate in first half.
    pub first_half_reuse_rate: f64,
    /// Content word reuse rate in second half.
    pub second_half_reuse_rate: f64,
}

const WINDOW_SIZE: usize = 200;

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

/// Compute interaction features.
///
/// Returns `None` if text is too short for meaningful windowed analysis.
pub fn analyze(text: &str) -> Option<InteractionResult> {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    if words.len() < WINDOW_SIZE * 2 {
        return None;
    }

    let diversity_length_correlation = compute_diversity_length_correlation(text, &words);
    let diversity_window_count = words.len() / WINDOW_SIZE;

    let (repetition_position_delta, first_half_reuse_rate, second_half_reuse_rate) =
        compute_repetition_position_delta(&words);

    Some(InteractionResult {
        diversity_length_correlation,
        diversity_window_count,
        repetition_position_delta,
        first_half_reuse_rate,
        second_half_reuse_rate,
    })
}

/// Feature 1: Pearson r(TTR, mean_sentence_length) across 200-word windows.
fn compute_diversity_length_correlation(text: &str, words: &[String]) -> f64 {
    let num_windows = words.len() / WINDOW_SIZE;
    if num_windows < 3 {
        return 0.0;
    }

    // We need sentence boundaries that fall within each window.
    // Approximate by splitting text into proportional chunks.
    let sentences = split_sentences(text);
    if sentences.len() < num_windows {
        return 0.0;
    }

    let mut ttrs: Vec<f64> = Vec::with_capacity(num_windows);
    let mut mean_sent_lens: Vec<f64> = Vec::with_capacity(num_windows);

    for i in 0..num_windows {
        let start = i * WINDOW_SIZE;
        let end = (start + WINDOW_SIZE).min(words.len());
        let window = &words[start..end];

        // TTR for this window
        let unique: HashSet<&String> = window.iter().collect();
        let ttr = unique.len() as f64 / window.len() as f64;
        ttrs.push(ttr);

        // Mean sentence length: find sentences whose words fall mostly in this window
        // Approximation: use the fraction of total sentences proportional to window position
        let frac_start = start as f64 / words.len() as f64;
        let frac_end = end as f64 / words.len() as f64;
        let sent_start = (frac_start * sentences.len() as f64) as usize;
        let sent_end = ((frac_end * sentences.len() as f64) as usize).min(sentences.len());

        if sent_end > sent_start {
            let window_sents = &sentences[sent_start..sent_end];
            let total_words_in_sents: usize = window_sents
                .iter()
                .map(|s| s.split_whitespace().count())
                .sum();
            let mean_len = total_words_in_sents as f64 / window_sents.len() as f64;
            mean_sent_lens.push(mean_len);
        } else {
            mean_sent_lens.push(0.0);
        }
    }

    pearson_correlation(&ttrs, &mean_sent_lens)
}

/// Feature 2: Content word repetition delta between halves.
fn compute_repetition_position_delta(words: &[String]) -> (f64, f64, f64) {
    let func_set: HashSet<&str> = FUNCTION_WORDS.iter().copied().collect();

    let content_words: Vec<&str> = words
        .iter()
        .filter(|w| !func_set.contains(w.as_str()) && w.len() > 2)
        .map(|w| w.as_str())
        .collect();

    if content_words.len() < 20 {
        return (0.0, 0.0, 0.0);
    }

    let mid = content_words.len() / 2;
    let first_half = &content_words[..mid];
    let second_half = &content_words[mid..];

    let first_reuse = compute_reuse_rate(first_half);
    let second_reuse = compute_reuse_rate(second_half);

    let delta = (first_reuse - second_reuse).abs();

    (delta, first_reuse, second_reuse)
}

/// Compute the reuse rate: fraction of words that appeared earlier in the slice.
fn compute_reuse_rate(words: &[&str]) -> f64 {
    if words.is_empty() {
        return 0.0;
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut reuse_count = 0;

    for &word in words {
        if seen.contains(word) {
            reuse_count += 1;
        } else {
            seen.insert(word);
        }
    }

    reuse_count as f64 / words.len() as f64
}

/// Split text into sentences (mirrors syntactic.rs logic).
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for i in 0..len {
        current.push(chars[i]);
        let is_terminal = chars[i] == '.' || chars[i] == '!' || chars[i] == '?';
        if is_terminal {
            let next_is_boundary = i + 1 >= len
                || chars[i + 1].is_whitespace()
                || chars[i + 1] == '"'
                || chars[i + 1] == '\''
                || chars[i + 1] == ')';
            if next_is_boundary {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() && trimmed.split_whitespace().count() > 0 {
                    sentences.push(trimmed);
                }
                current.clear();
            }
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() && trimmed.split_whitespace().count() > 1 {
        sentences.push(trimmed);
    }
    sentences
}

/// Pearson correlation coefficient between two series.
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 3 {
        return 0.0;
    }

    let x = &x[..n];
    let y = &y[..n];

    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < f64::EPSILON {
        return 0.0;
    }

    cov / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reuse_rate() {
        let words = vec!["hello", "world", "hello", "foo", "world"];
        let rate = compute_reuse_rate(&words);
        // 2 reuses out of 5 words = 0.4
        assert!((rate - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_short_text_returns_none() {
        assert!(analyze("Too short.").is_none());
    }

    #[test]
    fn test_analyze_long_text() {
        let text = "The quick brown fox jumps over the lazy dog. \
                     A wonderful serenity has taken possession of my entire soul. \
                     I am alone and feel the charm of existence in this spot. "
            .repeat(30);
        let result = analyze(&text);
        assert!(result.is_some());
    }
}
