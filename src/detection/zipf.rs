//! Zipf's law slope deviation for AI detection.
//!
//! Natural language follows Zipf's law: word frequency ∝ 1/rank^s, where s ≈ 1.0.
//! In log-log space this is a linear relationship with slope ≈ -1.0.
//! AI text adheres more tightly to this law (deviation ±0.05),
//! while human text deviates more (±0.15).
//!
//! Human range: ±0.15 from -1.0 | AI range: ±0.05 from -1.0 | Cohen's d ≈ 1.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Result of Zipf slope analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipfResult {
    /// The fitted slope in log-log space (expected ≈ -1.0 for natural language).
    pub slope: f64,
    /// Absolute deviation from the ideal -1.0 slope.
    /// Smaller deviation = more AI-like.
    pub deviation: f64,
    /// R² goodness of fit. Higher R² with low deviation = more AI-like.
    pub r_squared: f64,
    /// Number of distinct words used in the fit.
    pub vocabulary_size: usize,
    /// Residual variance around the fitted line.
    pub residual_variance: f64,
}

/// Minimum vocabulary size for a meaningful Zipf fit.
const MIN_VOCAB: usize = 50;

/// Compute Zipf slope deviation for the given text.
///
/// Returns `None` if vocabulary is too small for a meaningful fit.
pub fn analyze(text: &str) -> Option<ZipfResult> {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .filter(|w| w.chars().any(|c| c.is_alphabetic()))
        .collect();

    let mut frequency: HashMap<String, usize> = HashMap::new();
    for word in &words {
        *frequency.entry(word.clone()).or_insert(0) += 1;
    }

    if frequency.len() < MIN_VOCAB {
        return None;
    }

    // Sort by frequency descending
    let mut freq_vec: Vec<usize> = frequency.values().copied().collect();
    freq_vec.sort_unstable_by(|a, b| b.cmp(a));

    // Log-log transform: log(rank) vs log(frequency)
    let mut log_ranks: Vec<f64> = Vec::with_capacity(freq_vec.len());
    let mut log_freqs: Vec<f64> = Vec::with_capacity(freq_vec.len());

    for (i, &freq) in freq_vec.iter().enumerate() {
        if freq == 0 {
            break;
        }
        log_ranks.push(((i + 1) as f64).ln());
        log_freqs.push((freq as f64).ln());
    }

    let n = log_ranks.len();
    if n < 10 {
        return None;
    }

    // Ordinary least squares: log_freq = slope * log_rank + intercept
    let (slope, _intercept, r_squared, residual_variance) =
        linear_regression(&log_ranks, &log_freqs);

    let deviation = (slope - (-1.0)).abs();

    Some(ZipfResult {
        slope,
        deviation,
        r_squared,
        vocabulary_size: frequency.len(),
        residual_variance,
    })
}

/// Simple OLS linear regression.
/// Returns (slope, intercept, r_squared, residual_variance).
fn linear_regression(x: &[f64], y: &[f64]) -> (f64, f64, f64, f64) {
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    let denom = sum_x2 - sum_x * sum_x / n;
    if denom.abs() < f64::EPSILON {
        return (0.0, mean_y, 0.0, 0.0);
    }

    let slope = (sum_xy - sum_x * sum_y / n) / denom;
    let intercept = mean_y - slope * mean_x;

    // R² = 1 - SS_res / SS_tot
    let ss_tot: f64 = y.iter().map(|yi| (yi - mean_y).powi(2)).sum();
    let ss_res: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| {
            let predicted = slope * xi + intercept;
            (yi - predicted).powi(2)
        })
        .sum();

    let r_squared = if ss_tot > f64::EPSILON {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    let residual_variance = if n > 2.0 { ss_res / (n - 2.0) } else { 0.0 };

    (slope, intercept, r_squared, residual_variance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression_perfect_fit() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let (slope, intercept, r2, _) = linear_regression(&x, &y);
        assert!((slope - 2.0).abs() < 1e-10);
        assert!((intercept - 0.0).abs() < 1e-10);
        assert!((r2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_short_text_returns_none() {
        let short = "hello world";
        assert!(analyze(short).is_none());
    }

    #[test]
    fn test_analyze_returns_result() {
        // Build text with enough vocabulary
        let words: Vec<&str> = vec![
            "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
            "and", "cat", "sat", "on", "mat", "while", "bird", "flew",
            "high", "above", "the", "tall", "green", "trees", "near",
            "river", "bank", "where", "fish", "swim", "deep", "cold",
            "water", "flows", "through", "ancient", "forest", "paths",
            "lead", "toward", "mountain", "peak", "covered", "snow",
            "white", "clouds", "drift", "across", "blue", "sky",
            "gentle", "breeze", "carries", "sweet", "scent", "flowers",
        ];
        let text = (0..200)
            .map(|i| words[i % words.len()])
            .collect::<Vec<_>>()
            .join(" ");
        let result = analyze(&text);
        assert!(result.is_some());
        let r = result.unwrap();
        // Slope should be negative (Zipf's law)
        assert!(r.slope < 0.0, "Zipf slope should be negative: {}", r.slope);
    }
}
