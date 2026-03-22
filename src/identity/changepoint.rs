//! Style change point detection using binary segmentation.
//!
//! Identifies positions in a text where the writing style changes
//! significantly, suggesting a different author or writing session.
//! Uses a simplified binary segmentation approach on stylometric features.

use serde::{Deserialize, Serialize};

use super::anomaly::{WindowConfig, WindowResult};

/// A detected style change point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePoint {
    /// Word position where the change occurs
    pub position: usize,
    /// Confidence score (0.0–1.0)
    pub confidence: f64,
    /// The type of style shift detected
    pub shift_type: ShiftType,
    /// Features that changed most at this point
    pub key_features: Vec<(String, f64)>,
}

/// Type of style shift at a change point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShiftType {
    /// Gradual transition
    Gradual,
    /// Abrupt change
    Abrupt,
}

/// Result of change point detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePointResult {
    /// Detected change points, sorted by position
    pub change_points: Vec<ChangePoint>,
    /// Number of segments identified
    pub n_segments: usize,
    /// Overall style consistency (0.0 = many changes, 1.0 = very consistent)
    pub consistency_score: f64,
}

/// Detect style change points from window analysis results.
///
/// Uses binary segmentation: finds the split point that maximizes
/// the stylometric distance between the two halves, then recurses
/// on each half.
pub fn detect_change_points(
    windows: &[WindowResult],
    config: &WindowConfig,
    min_segment_size: usize,
) -> ChangePointResult {
    if windows.len() < 2 {
        return ChangePointResult {
            change_points: Vec::new(),
            n_segments: 1,
            consistency_score: 1.0,
        };
    }

    let mut change_points = Vec::new();
    let min_seg = min_segment_size.max(2);

    binary_segment(windows, 0, windows.len(), min_seg, config.anomaly_threshold, &mut change_points);

    change_points.sort_by_key(|cp| cp.position);

    let n_segments = change_points.len() + 1;
    let consistency = if change_points.is_empty() {
        1.0
    } else {
        let max_confidence = change_points
            .iter()
            .map(|cp| cp.confidence)
            .fold(0.0f64, f64::max);
        (1.0 - max_confidence).max(0.0)
    };

    ChangePointResult {
        change_points,
        n_segments,
        consistency_score: consistency,
    }
}

/// Recursive binary segmentation.
fn binary_segment(
    windows: &[WindowResult],
    global_offset: usize,
    len: usize,
    min_segment: usize,
    threshold: f64,
    results: &mut Vec<ChangePoint>,
) {
    if len < min_segment * 2 {
        return;
    }

    let segment = &windows[global_offset..global_offset + len];

    // Find the split point that maximizes between-group distance
    let (best_split, best_score, best_features) = find_best_split(segment, min_segment);

    // Normalize score to 0-1 confidence
    let confidence = sigmoid(best_score, threshold);

    if confidence < 0.3 {
        return; // Not significant enough
    }

    let split_window = &segment[best_split];
    let shift_type = if best_score > threshold * 1.5 {
        ShiftType::Abrupt
    } else {
        ShiftType::Gradual
    };

    results.push(ChangePoint {
        position: split_window.start_word,
        confidence,
        shift_type,
        key_features: best_features,
    });

    // Recurse on each half
    binary_segment(windows, global_offset, best_split, min_segment, threshold, results);
    binary_segment(
        windows,
        global_offset + best_split,
        len - best_split,
        min_segment,
        threshold,
        results,
    );
}

/// Find the split point that maximizes the distance between two halves.
///
/// Returns (split_index, score, key_features).
fn find_best_split(
    windows: &[WindowResult],
    min_segment: usize,
) -> (usize, f64, Vec<(String, f64)>) {
    let n = windows.len();
    let n_features = if windows.is_empty() || windows[0].features.values.is_empty() {
        0
    } else {
        windows[0].features.len()
    };

    if n_features == 0 || n < min_segment * 2 {
        return (n / 2, 0.0, Vec::new());
    }

    let mut best_split = n / 2;
    let mut best_score = 0.0;
    let mut best_features = Vec::new();

    for split in min_segment..=(n - min_segment) {
        let (score, features) = compute_split_score(windows, split, n_features);
        if score > best_score {
            best_score = score;
            best_split = split;
            best_features = features;
        }
    }

    (best_split, best_score, best_features)
}

/// Compute the distance score for a split at the given position.
fn compute_split_score(
    windows: &[WindowResult],
    split: usize,
    n_features: usize,
) -> (f64, Vec<(String, f64)>) {
    let left = &windows[..split];
    let right = &windows[split..];

    if left.is_empty() || right.is_empty() {
        return (0.0, Vec::new());
    }

    // Compute mean of each feature in each half
    let left_means = compute_means(left, n_features);
    let right_means = compute_means(right, n_features);
    let left_stds = compute_stds(left, &left_means, n_features);

    // Compute feature-wise differences
    let feature_names = &windows[0].features.names;
    let mut diffs: Vec<(String, f64)> = Vec::new();
    let mut total_diff = 0.0;

    for i in 0..n_features.min(left_means.len()).min(right_means.len()) {
        let std = if i < left_stds.len() { left_stds[i].max(1e-10) } else { 1e-10 };
        let z = (left_means[i] - right_means[i]).abs() / std;
        total_diff += z * z;

        if z > 1.0 && i < feature_names.len() {
            diffs.push((feature_names[i].clone(), z));
        }
    }

    let score = (total_diff / n_features as f64).sqrt();

    // Top 5 most different features
    diffs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    diffs.truncate(5);

    (score, diffs)
}

/// Compute feature means across windows.
fn compute_means(windows: &[WindowResult], n_features: usize) -> Vec<f64> {
    let mut means = vec![0.0; n_features];
    let mut counts = vec![0usize; n_features];

    for w in windows {
        for (i, &val) in w.features.values.iter().enumerate() {
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

    means
}

/// Compute feature standard deviations across windows.
fn compute_stds(windows: &[WindowResult], means: &[f64], n_features: usize) -> Vec<f64> {
    let mut stds = vec![0.0; n_features];
    let mut counts = vec![0usize; n_features];

    for w in windows {
        for (i, &val) in w.features.values.iter().enumerate() {
            if i < n_features && i < means.len() {
                let diff = val - means[i];
                stds[i] += diff * diff;
                counts[i] += 1;
            }
        }
    }

    for i in 0..n_features {
        if counts[i] > 0 {
            stds[i] = (stds[i] / counts[i] as f64).sqrt();
        }
    }

    stds
}

/// Sigmoid function for normalizing scores to 0-1.
fn sigmoid(x: f64, center: f64) -> f64 {
    1.0 / (1.0 + (-5.0 * (x - center)).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::features::FeatureVector;

    fn make_window(idx: usize, values: Vec<f64>) -> WindowResult {
        WindowResult {
            index: idx,
            start_word: idx * 100,
            end_word: idx * 100 + 500,
            anomaly_score: 0.0,
            features: FeatureVector {
                names: vec!["f1".into(), "f2".into(), "f3".into()],
                values,
            },
            is_anomalous: false,
            deviations: Vec::new(),
        }
    }

    #[test]
    fn test_no_change_points_uniform() {
        // All windows have similar features
        let windows: Vec<WindowResult> = (0..10)
            .map(|i| make_window(i, vec![1.0, 2.0, 3.0]))
            .collect();

        let config = WindowConfig::default();
        let result = detect_change_points(&windows, &config, 3);

        assert!(result.change_points.is_empty());
        assert_eq!(result.n_segments, 1);
        assert!((result.consistency_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_change_point_detection() {
        // Two distinct clusters of windows
        let mut windows: Vec<WindowResult> = Vec::new();
        for i in 0..5 {
            windows.push(make_window(i, vec![1.0, 2.0, 3.0]));
        }
        for i in 5..10 {
            windows.push(make_window(i, vec![10.0, 20.0, 30.0])); // Very different
        }

        let config = WindowConfig {
            anomaly_threshold: 1.0,
            ..Default::default()
        };
        let result = detect_change_points(&windows, &config, 2);

        assert!(!result.change_points.is_empty(), "Should detect a change point");
        assert!(result.n_segments >= 2);
        assert!(result.consistency_score < 1.0);
    }

    #[test]
    fn test_too_few_windows() {
        let windows = vec![make_window(0, vec![1.0])];
        let config = WindowConfig::default();
        let result = detect_change_points(&windows, &config, 2);

        assert!(result.change_points.is_empty());
        assert_eq!(result.n_segments, 1);
    }

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0, 0.0) - 0.5).abs() < 0.01);
        assert!(sigmoid(5.0, 0.0) > 0.99);
        assert!(sigmoid(-5.0, 0.0) < 0.01);
    }
}
