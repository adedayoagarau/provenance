//! Multi-author segmentation for documents with mixed authorship.
//!
//! Uses change point detection to split a document into segments,
//! then attributes each segment to candidate author profiles.

use serde::{Deserialize, Serialize};

use super::anomaly::{self, WindowConfig, WindowAnalysis};
use super::changepoint::{self, ChangePointResult};
use super::features::{extract, FeatureVector};
use crate::analysis;

/// A segment of text attributed to a single author.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Segment index (0-based)
    pub index: usize,
    /// Start word position
    pub start_word: usize,
    /// End word position
    pub end_word: usize,
    /// Approximate word count
    pub word_count: usize,
    /// Feature vector for this segment
    pub features: FeatureVector,
    /// Best matching candidate (if candidates provided)
    pub attribution: Option<Attribution>,
}

/// Attribution result for a segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    /// Candidate name/label
    pub candidate: String,
    /// Confidence score (0.0–1.0)
    pub confidence: f64,
    /// Distance to each candidate (candidate → distance)
    pub distances: Vec<(String, f64)>,
}

/// Result of full document segmentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationResult {
    /// Identified segments
    pub segments: Vec<Segment>,
    /// Whether multi-authorship was detected
    pub is_multi_author: bool,
    /// Number of distinct authors detected
    pub estimated_authors: usize,
    /// Window analysis details
    pub window_analysis: WindowAnalysis,
    /// Change point detection details
    pub change_points: ChangePointResult,
}

/// Configuration for segmentation.
#[derive(Debug, Clone)]
pub struct SegmentationConfig {
    /// Window analysis settings
    pub window_config: WindowConfig,
    /// Minimum segment size in windows
    pub min_segment_windows: usize,
    /// Confidence threshold for multi-author detection
    pub multi_author_threshold: f64,
}

impl Default for SegmentationConfig {
    fn default() -> Self {
        Self {
            window_config: WindowConfig::default(),
            min_segment_windows: 3,
            multi_author_threshold: 0.5,
        }
    }
}

/// Segment a document into author-attributed sections.
pub fn segment_document(
    text: &str,
    candidates: &[(String, FeatureVector)],
    config: &SegmentationConfig,
) -> SegmentationResult {
    // Step 1: Sliding window analysis
    let window_analysis = anomaly::sliding_window_analysis(text, &config.window_config);

    // Step 2: Change point detection
    let change_points = changepoint::detect_change_points(
        &window_analysis.windows,
        &config.window_config,
        config.min_segment_windows,
    );

    // Step 3: Build segments from change points
    let words: Vec<&str> = text.split_whitespace().collect();
    let segments = build_segments(&words, &change_points, candidates, config);

    // Step 4: Determine if multi-authored
    let is_multi_author = change_points
        .change_points
        .iter()
        .any(|cp| cp.confidence > config.multi_author_threshold);

    let estimated_authors = if is_multi_author {
        estimate_author_count(&segments)
    } else {
        1
    };

    SegmentationResult {
        segments,
        is_multi_author,
        estimated_authors,
        window_analysis,
        change_points,
    }
}

/// Build segments from change points and attribute to candidates.
fn build_segments(
    words: &[&str],
    change_points: &ChangePointResult,
    candidates: &[(String, FeatureVector)],
    config: &SegmentationConfig,
) -> Vec<Segment> {
    // Collect segment boundaries
    let mut boundaries: Vec<usize> = vec![0];
    for cp in &change_points.change_points {
        boundaries.push(cp.position);
    }
    boundaries.push(words.len());
    boundaries.sort();
    boundaries.dedup();

    let mut segments = Vec::new();

    for i in 0..boundaries.len() - 1 {
        let start = boundaries[i];
        let end = boundaries[i + 1];

        if end <= start {
            continue;
        }

        let segment_text: String = words[start..end].join(" ");
        let word_count = end - start;

        // Extract features for this segment
        let features = if let Ok(analysis_result) = analysis::analyze_text(&segment_text) {
            extract(&analysis_result, config.window_config.feature_set)
        } else {
            FeatureVector {
                names: Vec::new(),
                values: Vec::new(),
            }
        };

        // Attribute to candidates
        let attribution = if !candidates.is_empty() && !features.is_empty() {
            Some(attribute_segment(&features, candidates))
        } else {
            None
        };

        segments.push(Segment {
            index: i,
            start_word: start,
            end_word: end,
            word_count,
            features,
            attribution,
        });
    }

    segments
}

/// Attribute a segment to the best-matching candidate.
fn attribute_segment(
    segment_features: &FeatureVector,
    candidates: &[(String, FeatureVector)],
) -> Attribution {
    let mut distances: Vec<(String, f64)> = candidates
        .iter()
        .map(|(name, candidate_fv)| {
            let dist = feature_distance(segment_features, candidate_fv);
            (name.clone(), dist)
        })
        .collect();

    distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let best = &distances[0];
    let confidence = if distances.len() >= 2 {
        // Confidence based on margin between best and second-best
        let margin = distances[1].1 - distances[0].1;
        let max_dist = distances.last().map(|(_, d)| *d).unwrap_or(1.0).max(1e-10);
        (margin / max_dist).min(1.0)
    } else {
        1.0
    };

    Attribution {
        candidate: best.0.clone(),
        confidence,
        distances,
    }
}

/// Compute feature-wise distance between two feature vectors.
fn feature_distance(a: &FeatureVector, b: &FeatureVector) -> f64 {
    // Build name→value map for b
    let b_map: std::collections::HashMap<&str, f64> = b
        .names
        .iter()
        .zip(b.values.iter())
        .map(|(n, &v)| (n.as_str(), v))
        .collect();

    let mut sum_sq = 0.0;
    let mut count = 0;

    for (i, name) in a.names.iter().enumerate() {
        if let Some(&b_val) = b_map.get(name.as_str()) {
            let diff = a.values[i] - b_val;
            sum_sq += diff * diff;
            count += 1;
        }
    }

    if count > 0 {
        (sum_sq / count as f64).sqrt()
    } else {
        f64::MAX
    }
}

/// Estimate the number of distinct authors from segment attributions.
fn estimate_author_count(segments: &[Segment]) -> usize {
    let mut authors: std::collections::HashSet<String> = std::collections::HashSet::new();

    for seg in segments {
        if let Some(ref attr) = seg.attribution {
            if attr.confidence > 0.3 {
                authors.insert(attr.candidate.clone());
            }
        }
    }

    authors.len().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_short_document() {
        let text = "This is a short document that should produce a single segment.";
        let config = SegmentationConfig::default();
        let result = segment_document(text, &[], &config);

        assert_eq!(result.segments.len(), 1);
        assert!(!result.is_multi_author);
        assert_eq!(result.estimated_authors, 1);
    }

    #[test]
    fn test_feature_distance() {
        let a = FeatureVector {
            names: vec!["f1".into(), "f2".into()],
            values: vec![1.0, 2.0],
        };
        let b = FeatureVector {
            names: vec!["f1".into(), "f2".into()],
            values: vec![1.0, 2.0],
        };
        assert!((feature_distance(&a, &b)).abs() < 1e-10);

        let c = FeatureVector {
            names: vec!["f1".into(), "f2".into()],
            values: vec![4.0, 6.0],
        };
        assert!(feature_distance(&a, &c) > 0.0);
    }

    #[test]
    fn test_attribution_with_candidates() {
        let segment = FeatureVector {
            names: vec!["f1".into(), "f2".into()],
            values: vec![1.0, 2.0],
        };

        let candidates = vec![
            ("alice".to_string(), FeatureVector {
                names: vec!["f1".into(), "f2".into()],
                values: vec![1.1, 2.1],
            }),
            ("bob".to_string(), FeatureVector {
                names: vec!["f1".into(), "f2".into()],
                values: vec![5.0, 6.0],
            }),
        ];

        let attr = attribute_segment(&segment, &candidates);
        assert_eq!(attr.candidate, "alice");
        assert!(attr.confidence > 0.0);
    }
}
