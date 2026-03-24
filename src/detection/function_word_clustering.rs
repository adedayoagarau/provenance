//! Function word clustering uniformity for AI detection.
//!
//! Measures how uniformly function words are distributed across text windows.
//! Human writers show natural variation in function word usage patterns;
//! AI text tends to be more uniform.
//!
//! This feature survives prompt engineering attacks (25-40% evasion success)
//! because function word usage is largely unconscious and difficult to control.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use unicode_segmentation::UnicodeSegmentation;

/// Result of function word clustering analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionWordClusterResult {
    /// Coefficient of variation of function word ratios across windows.
    /// Lower CV = more uniform = more AI-like.
    pub ratio_cv: f64,
    /// Jensen-Shannon divergence between window function word distributions.
    /// Lower JS = more uniform = more AI-like.
    pub inter_window_divergence: f64,
    /// Number of windows analyzed.
    pub window_count: usize,
    /// Top function words by usage variance (highest variance = most human-varying).
    pub most_variable_words: Vec<(String, f64)>,
}

const WINDOW_SIZE: usize = 200;

const FUNCTION_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "to", "of", "in", "for",
    "on", "with", "at", "by", "from", "as", "into", "through", "during",
    "before", "after", "and", "but", "or", "nor", "not", "so", "yet",
    "both", "either", "neither", "each", "every", "all", "any", "some",
    "no", "only", "than", "too", "very", "just", "also",
    "i", "me", "my", "we", "us", "our", "you", "your", "he", "him",
    "his", "she", "her", "it", "its", "they", "them", "their",
    "this", "that", "these", "those", "which", "who", "whom",
];

/// Analyze function word clustering uniformity.
///
/// Returns `None` if text is too short for windowed analysis.
pub fn analyze(text: &str) -> Option<FunctionWordClusterResult> {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    if words.len() < WINDOW_SIZE * 3 {
        return None;
    }

    let fw_set: HashSet<&str> = FUNCTION_WORDS.iter().copied().collect();
    let num_windows = words.len() / WINDOW_SIZE;

    // Compute per-window function word ratios and distributions
    let mut window_ratios: Vec<f64> = Vec::with_capacity(num_windows);
    let mut window_distributions: Vec<HashMap<&str, f64>> = Vec::with_capacity(num_windows);

    for i in 0..num_windows {
        let start = i * WINDOW_SIZE;
        let end = (start + WINDOW_SIZE).min(words.len());
        let window = &words[start..end];
        let window_len = window.len() as f64;

        let mut fw_count = 0;
        let mut fw_freq: HashMap<&str, f64> = HashMap::new();

        for word in window {
            if fw_set.contains(word.as_str()) {
                fw_count += 1;
                // Find the matching function word (need to map back to static str)
                for &fw in FUNCTION_WORDS {
                    if fw == word.as_str() {
                        *fw_freq.entry(fw).or_insert(0.0) += 1.0 / window_len;
                        break;
                    }
                }
            }
        }

        window_ratios.push(fw_count as f64 / window_len);
        window_distributions.push(fw_freq);
    }

    // 1. CV of function word ratios across windows
    let mean_ratio = window_ratios.iter().sum::<f64>() / window_ratios.len() as f64;
    let variance = window_ratios.iter().map(|r| (r - mean_ratio).powi(2)).sum::<f64>()
        / (window_ratios.len() - 1) as f64;
    let ratio_cv = if mean_ratio > f64::EPSILON {
        variance.sqrt() / mean_ratio
    } else {
        0.0
    };

    // 2. Mean pairwise JS divergence between consecutive windows
    let mut js_sum = 0.0;
    let mut js_count = 0;

    for i in 0..window_distributions.len().saturating_sub(1) {
        let js = jensen_shannon(&window_distributions[i], &window_distributions[i + 1]);
        js_sum += js;
        js_count += 1;
    }

    let inter_window_divergence = if js_count > 0 {
        js_sum / js_count as f64
    } else {
        0.0
    };

    // 3. Per-word variance across windows
    let mut word_variances: Vec<(String, f64)> = Vec::new();
    for &fw in FUNCTION_WORDS {
        let freqs: Vec<f64> = window_distributions
            .iter()
            .map(|d| *d.get(fw).unwrap_or(&0.0))
            .collect();

        let fw_mean = freqs.iter().sum::<f64>() / freqs.len() as f64;
        if fw_mean > 0.001 {
            let fw_var = freqs.iter().map(|f| (f - fw_mean).powi(2)).sum::<f64>() / freqs.len() as f64;
            word_variances.push((fw.to_string(), fw_var));
        }
    }

    word_variances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let most_variable_words = word_variances.into_iter().take(10).collect();

    Some(FunctionWordClusterResult {
        ratio_cv,
        inter_window_divergence,
        window_count: num_windows,
        most_variable_words,
    })
}

/// Jensen-Shannon divergence between two frequency distributions.
fn jensen_shannon(p: &HashMap<&str, f64>, q: &HashMap<&str, f64>) -> f64 {
    // Collect all keys
    let all_keys: HashSet<&&str> = p.keys().chain(q.keys()).collect();

    let mut js = 0.0;
    for &&key in &all_keys {
        let p_val = *p.get(key).unwrap_or(&0.0);
        let q_val = *q.get(key).unwrap_or(&0.0);
        let m_val = (p_val + q_val) / 2.0;

        if m_val > f64::EPSILON {
            if p_val > f64::EPSILON {
                js += p_val * (p_val / m_val).ln();
            }
            if q_val > f64::EPSILON {
                js += q_val * (q_val / m_val).ln();
            }
        }
    }

    (js / 2.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_returns_none() {
        assert!(analyze("Too short.").is_none());
    }

    #[test]
    fn test_analyze_long_text() {
        let text = "The quick brown fox jumps over the lazy dog and the cat sat on the mat. \
                     A wonderful serenity has taken possession of my entire soul like these sweet \
                     mornings of spring which I enjoy with my whole heart. "
            .repeat(50);
        let result = analyze(&text);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.window_count >= 3);
        assert!(r.ratio_cv >= 0.0);
        assert!(r.inter_window_divergence >= 0.0);
    }

    #[test]
    fn test_jensen_shannon_identical() {
        let mut p = HashMap::new();
        p.insert("the", 0.1);
        p.insert("a", 0.05);
        let js = jensen_shannon(&p, &p);
        assert!(js < 1e-10, "Identical distributions should have JS ≈ 0: {js}");
    }
}
