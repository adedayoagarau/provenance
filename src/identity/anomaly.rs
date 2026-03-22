//! Sliding window anomaly detection for authorship analysis.
//!
//! Splits text into overlapping windows, extracts features per window,
//! and identifies windows that deviate significantly from the overall style.
//! This is the foundation for multi-author detection.

use serde::{Deserialize, Serialize};

use rayon::prelude::*;

use crate::analysis::{self, AnalysisResult};
use crate::identity::features::{extract, FeatureSet, FeatureVector};

/// Configuration for sliding window analysis.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window size in words
    pub window_size: usize,
    /// Slide step in words
    pub slide_step: usize,
    /// Feature set to use
    pub feature_set: FeatureSet,
    /// Z-score threshold for anomaly detection
    pub anomaly_threshold: f64,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            window_size: 500,
            slide_step: 100,
            feature_set: FeatureSet::Standard,
            anomaly_threshold: 2.0,
        }
    }
}

/// Result of sliding window analysis for a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAnalysis {
    /// Per-window results
    pub windows: Vec<WindowResult>,
    /// Overall anomaly score (0.0 = uniform style, 1.0 = highly varied)
    pub overall_anomaly_score: f64,
    /// Indices of windows flagged as anomalous
    pub anomalous_windows: Vec<usize>,
    /// Number of windows analyzed
    pub window_count: usize,
}

/// Analysis result for a single text window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowResult {
    /// Window index (0-based)
    pub index: usize,
    /// Start word position
    pub start_word: usize,
    /// End word position
    pub end_word: usize,
    /// Anomaly score for this window (higher = more anomalous)
    pub anomaly_score: f64,
    /// Feature vector for this window
    pub features: FeatureVector,
    /// Whether this window is flagged as anomalous
    pub is_anomalous: bool,
    /// Key deviating features (feature name, z-score)
    pub deviations: Vec<(String, f64)>,
}

/// An anomaly detected in a document section (backward-compatible type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub section: String,
    pub deviation_score: f64,
    pub description: String,
}

/// Run sliding window analysis on text.
pub fn sliding_window_analysis(text: &str, config: &WindowConfig) -> WindowAnalysis {
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.len() < config.window_size {
        // Text too short for windowing — analyze as a single window
        return single_window_analysis(text, &words);
    }

    // Collect window ranges
    let mut all_ranges = Vec::new();
    let mut pos = 0;
    while pos + config.window_size <= words.len() {
        all_ranges.push((pos, pos + config.window_size));
        pos += config.slide_step;
    }

    // Extract features for all windows in parallel
    let feature_set = config.feature_set;
    let window_results: Vec<_> = all_ranges
        .par_iter()
        .filter_map(|&(start, end)| {
            let window_text: String = words[start..end].join(" ");
            analysis::analyze_text(&window_text).ok().map(|a| {
                (extract(&a, feature_set), (start, end))
            })
        })
        .collect();

    let (window_features, window_ranges): (Vec<_>, Vec<_>) =
        window_results.into_iter().unzip();

    if window_features.is_empty() {
        return WindowAnalysis {
            windows: Vec::new(),
            overall_anomaly_score: 0.0,
            anomalous_windows: Vec::new(),
            window_count: 0,
        };
    }

    // Compute mean and std for each feature across all windows
    let n_features = window_features[0].len();
    let n_windows = window_features.len();

    let mut means = vec![0.0f64; n_features];
    let mut counts = vec![0usize; n_features];

    for wf in &window_features {
        for (i, &val) in wf.values.iter().enumerate() {
            if i < n_features {
                means[i] += val;
                counts[i] += 1;
            }
        }
    }
    for i in 0..n_features {
        if counts[i] > 0 {
            means[i] /= counts[i] as f64;
        }
    }

    let mut stds = vec![0.0f64; n_features];
    for wf in &window_features {
        for (i, &val) in wf.values.iter().enumerate() {
            if i < n_features {
                let diff = val - means[i];
                stds[i] += diff * diff;
            }
        }
    }
    for i in 0..n_features {
        if counts[i] > 0 {
            stds[i] = (stds[i] / counts[i] as f64).sqrt();
        }
        if stds[i] < 1e-10 {
            stds[i] = 1e-10;
        }
    }

    // Score each window
    let mut windows = Vec::with_capacity(n_windows);
    let mut anomalous_indices = Vec::new();

    for (idx, features) in window_features.into_iter().enumerate() {
        let (start, end) = window_ranges[idx];

        // Compute z-scores and anomaly score
        let mut z_scores = Vec::new();
        let mut total_z_sq = 0.0;
        let mut deviations = Vec::new();

        for (i, &val) in features.values.iter().enumerate() {
            if i < n_features {
                let z = (val - means[i]) / stds[i];
                z_scores.push(z);
                total_z_sq += z * z;

                if z.abs() > config.anomaly_threshold {
                    deviations.push((features.names[i].clone(), z));
                }
            }
        }

        // RMS z-score as anomaly measure
        let anomaly_score = if !z_scores.is_empty() {
            (total_z_sq / z_scores.len() as f64).sqrt()
        } else {
            0.0
        };

        let is_anomalous = anomaly_score > config.anomaly_threshold;
        if is_anomalous {
            anomalous_indices.push(idx);
        }

        // Keep only top 5 deviations
        deviations.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        deviations.truncate(5);

        windows.push(WindowResult {
            index: idx,
            start_word: start,
            end_word: end,
            anomaly_score,
            features,
            is_anomalous,
            deviations,
        });
    }

    // Overall anomaly = variance of per-window anomaly scores
    let mean_anomaly: f64 = windows.iter().map(|w| w.anomaly_score).sum::<f64>() / windows.len() as f64;
    let anomaly_variance: f64 = windows
        .iter()
        .map(|w| (w.anomaly_score - mean_anomaly).powi(2))
        .sum::<f64>()
        / windows.len() as f64;
    let overall = anomaly_variance.sqrt().min(1.0);

    WindowAnalysis {
        window_count: windows.len(),
        windows,
        overall_anomaly_score: overall,
        anomalous_windows: anomalous_indices,
    }
}

/// Analyze text too short for windowing.
fn single_window_analysis(text: &str, words: &[&str]) -> WindowAnalysis {
    if let Ok(analysis) = analysis::analyze_text(text) {
        let features = extract(&analysis, FeatureSet::Standard);
        let window = WindowResult {
            index: 0,
            start_word: 0,
            end_word: words.len(),
            anomaly_score: 0.0,
            features,
            is_anomalous: false,
            deviations: Vec::new(),
        };
        WindowAnalysis {
            windows: vec![window],
            overall_anomaly_score: 0.0,
            anomalous_windows: Vec::new(),
            window_count: 1,
        }
    } else {
        WindowAnalysis {
            windows: Vec::new(),
            overall_anomaly_score: 0.0,
            anomalous_windows: Vec::new(),
            window_count: 0,
        }
    }
}

/// Detect anomalous sections (backward-compatible interface).
pub fn detect(analysis: &AnalysisResult, profile: &super::profile::AuthorProfile) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    let ttr_diff = (analysis.lexical.type_token_ratio - profile.avg_type_token_ratio).abs();
    if ttr_diff > 0.15 {
        anomalies.push(Anomaly {
            section: "vocabulary".to_string(),
            deviation_score: ttr_diff,
            description: format!(
                "Type-token ratio ({:.3}) deviates significantly from profile ({:.3})",
                analysis.lexical.type_token_ratio, profile.avg_type_token_ratio
            ),
        });
    }

    let wl_diff = (analysis.lexical.avg_word_length - profile.avg_word_length).abs();
    if wl_diff > 1.0 {
        anomalies.push(Anomaly {
            section: "word_choice".to_string(),
            deviation_score: wl_diff,
            description: format!(
                "Average word length ({:.2}) deviates from profile ({:.2})",
                analysis.lexical.avg_word_length, profile.avg_word_length
            ),
        });
    }

    anomalies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_single_window() {
        let text = "This is a short text that has very few words.";
        let config = WindowConfig::default(); // 500 word window
        let result = sliding_window_analysis(text, &config);

        assert_eq!(result.window_count, 1);
        assert_eq!(result.overall_anomaly_score, 0.0);
        assert!(result.anomalous_windows.is_empty());
    }

    #[test]
    fn test_uniform_text_windows() {
        // Create a text long enough for multiple windows (500+ words)
        let sentence = "The quick brown fox jumps over the lazy dog near the river bank. ";
        let text = sentence.repeat(60); // ~780 words

        let config = WindowConfig {
            window_size: 200,
            slide_step: 50,
            anomaly_threshold: 2.0,
            feature_set: FeatureSet::Minimal,
        };

        let result = sliding_window_analysis(&text, &config);
        assert!(result.window_count > 1);
        // Uniform text should have low anomaly
        assert!(
            result.overall_anomaly_score < 0.5,
            "Uniform text anomaly score ({}) should be low",
            result.overall_anomaly_score
        );
    }

    #[test]
    fn test_window_config_defaults() {
        let config = WindowConfig::default();
        assert_eq!(config.window_size, 500);
        assert_eq!(config.slide_step, 100);
        assert!((config.anomaly_threshold - 2.0).abs() < f64::EPSILON);
    }
}
