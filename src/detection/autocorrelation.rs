//! Sentence length autocorrelation for AI detection.
//!
//! Measures the Pearson correlation between consecutive sentence lengths.
//! Human writers vary sentence length more randomly (near-zero autocorrelation),
//! while AI tends to produce more rhythmically consistent sentence lengths.
//!
//! Human range: -0.05 to +0.05 | AI range: 0.05 to 0.15 | Cohen's d ≈ 0.65

use serde::{Deserialize, Serialize};

/// Result of sentence length autocorrelation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocorrelationResult {
    /// Lag-1 autocorrelation of sentence lengths (word counts).
    /// Near zero = human-like; positive = AI-like.
    pub lag1_autocorrelation: f64,
    /// Lag-2 autocorrelation for additional signal.
    pub lag2_autocorrelation: f64,
    /// Number of sentences analyzed.
    pub sentence_count: usize,
    /// Mean sentence length (in words).
    pub mean_sentence_length: f64,
    /// Standard deviation of sentence lengths.
    pub sentence_length_std: f64,
}

/// Compute sentence length autocorrelation.
///
/// Returns `None` if there are fewer than 5 sentences.
pub fn analyze(text: &str) -> Option<AutocorrelationResult> {
    let sentences = split_sentences(text);
    let sentence_count = sentences.len();

    if sentence_count < 5 {
        return None;
    }

    let lengths: Vec<f64> = sentences
        .iter()
        .map(|s| s.split_whitespace().count() as f64)
        .collect();

    let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
    let variance = lengths.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
        / (lengths.len() - 1) as f64;
    let std_dev = variance.sqrt();

    let lag1 = pearson_autocorrelation(&lengths, 1);
    let lag2 = if sentence_count >= 6 {
        pearson_autocorrelation(&lengths, 2)
    } else {
        0.0
    };

    Some(AutocorrelationResult {
        lag1_autocorrelation: lag1,
        lag2_autocorrelation: lag2,
        sentence_count,
        mean_sentence_length: mean,
        sentence_length_std: std_dev,
    })
}

/// Compute Pearson autocorrelation at a given lag.
fn pearson_autocorrelation(series: &[f64], lag: usize) -> f64 {
    let n = series.len();
    if n <= lag + 1 {
        return 0.0;
    }

    let mean = series.iter().sum::<f64>() / n as f64;

    let mut numerator = 0.0;
    let mut denom = 0.0;

    for i in 0..n {
        denom += (series[i] - mean).powi(2);
        if i + lag < n {
            numerator += (series[i] - mean) * (series[i + lag] - mean);
        }
    }

    if denom < f64::EPSILON {
        return 0.0;
    }

    numerator / denom
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pearson_autocorrelation_constant() {
        // Constant series → autocorrelation = 0 (no variance)
        let constant = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let r = pearson_autocorrelation(&constant, 1);
        assert!((r - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pearson_autocorrelation_alternating() {
        // Alternating series → negative autocorrelation
        let alternating = vec![1.0, 10.0, 1.0, 10.0, 1.0, 10.0, 1.0, 10.0];
        let r = pearson_autocorrelation(&alternating, 1);
        assert!(r < 0.0, "Alternating series should have negative autocorrelation: {r}");
    }

    #[test]
    fn test_short_text_returns_none() {
        let text = "Short. Text. Here.";
        assert!(analyze(text).is_none());
    }

    #[test]
    fn test_analyze_multi_sentence() {
        let text = "The first sentence is quite long with many words in it. \
                     Short one. Another sentence of moderate length here. \
                     Yet another thing to say about the topic at hand. \
                     The final thought brings everything together nicely. \
                     And one more for good measure.";
        let result = analyze(text);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.sentence_count >= 5);
        assert!(r.lag1_autocorrelation.is_finite());
    }
}
