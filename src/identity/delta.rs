use ndarray::Array1;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::features::{CorpusStats, FeatureVector};

/// Burrows' Delta result comparing a query text against a candidate author.
///
/// Lower delta = more similar = more likely same author.
///
/// References:
/// - Burrows (2002): "Delta: A Measure of Stylistic Difference"
/// - Evert et al. (2017): "Understanding and explaining Delta measures"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaResult {
    /// Classic Burrows' Delta (Manhattan distance on z-scores)
    pub classic_delta: f64,
    /// Cosine Delta (Wurzburg variant) — often outperforms classic
    pub cosine_delta: f64,
    /// Euclidean Delta (Linear Delta)
    pub euclidean_delta: f64,
    /// Number of features used in computation
    pub feature_count: usize,
}

/// Compute all Delta variants between two z-scored feature vectors.
///
/// Both vectors must be z-score normalized against the same corpus statistics.
pub fn compute_delta(query_z: &Array1<f64>, candidate_z: &Array1<f64>) -> DeltaResult {
    let n = query_z.len();
    assert_eq!(n, candidate_z.len(), "Feature vectors must have same length");

    let nf = n as f64;

    // Classic Delta: mean absolute difference of z-scores
    // Δ_B(A,B) = (1/n) * Σ|z_A(i) - z_B(i)|
    let classic_delta = (query_z - candidate_z).mapv(f64::abs).sum() / nf;

    // Cosine Delta: 1 - cosine_similarity(z_A, z_B)
    // Cosine Delta is more robust and often outperforms Classic Delta
    // (Evert et al., 2017)
    let cosine_delta = 1.0 - cosine_similarity(query_z, candidate_z);

    // Euclidean Delta: Euclidean distance normalized by sqrt(n)
    let diff = query_z - candidate_z;
    let euclidean_delta = diff.mapv(|x| x * x).sum().sqrt() / nf.sqrt();

    DeltaResult {
        classic_delta,
        cosine_delta,
        euclidean_delta,
        feature_count: n,
    }
}

/// Compute Burrows' Delta between a query and a candidate using raw feature vectors.
///
/// Computes corpus statistics from the two vectors combined (simple 2-document scenario),
/// then z-normalizes and computes Delta.
///
/// For identical vectors, returns zero delta (perfect match).
pub fn delta_from_vectors(query: &FeatureVector, candidate: &FeatureVector) -> DeltaResult {
    // Fast path: identical vectors
    if query.values == candidate.values {
        return DeltaResult {
            classic_delta: 0.0,
            cosine_delta: 0.0,
            euclidean_delta: 0.0,
            feature_count: query.len(),
        };
    }

    let stats = CorpusStats::from_vectors(&[query.clone(), candidate.clone()])
        .expect("Non-empty vectors");

    let query_z = stats.normalize(query);
    let candidate_z = stats.normalize(candidate);

    compute_delta(&query_z, &candidate_z)
}

/// Compute Burrows' Delta between a query and a candidate using pre-computed
/// corpus statistics for z-normalization.
///
/// This is the preferred method when comparing against multiple candidates,
/// as the same corpus statistics should be used for all comparisons.
pub fn delta_with_corpus(
    query: &FeatureVector,
    candidate: &FeatureVector,
    corpus: &CorpusStats,
) -> DeltaResult {
    let query_z = corpus.normalize(query);
    let candidate_z = corpus.normalize(candidate);

    compute_delta(&query_z, &candidate_z)
}

/// Rank multiple candidates by their Delta distance to a query.
///
/// Returns candidates sorted by Cosine Delta (ascending = most similar first).
pub fn rank_candidates(
    query: &FeatureVector,
    candidates: &[(String, FeatureVector)],
    corpus: &CorpusStats,
) -> Vec<(String, DeltaResult)> {
    let query_z = corpus.normalize(query);

    let mut results: Vec<(String, DeltaResult)> = candidates
        .par_iter()
        .map(|(name, candidate)| {
            let candidate_z = corpus.normalize(candidate);
            let delta = compute_delta(&query_z, &candidate_z);
            (name.clone(), delta)
        })
        .collect();

    // Sort by cosine delta (ascending — lower = more similar)
    results.sort_by(|a, b| {
        a.1.cosine_delta.partial_cmp(&b.1.cosine_delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    let dot = a.dot(b);
    let norm_a = a.dot(a).sqrt();
    let norm_b = b.dot(b).sqrt();

    if norm_a < f64::EPSILON || norm_b < f64::EPSILON {
        return 0.0;
    }

    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors_zero_delta() {
        let v = FeatureVector {
            names: vec!["a".into(), "b".into(), "c".into()],
            values: vec![1.0, 2.0, 3.0],
        };
        let result = delta_from_vectors(&v, &v);

        assert!(result.classic_delta.abs() < 1e-10);
        assert!(result.cosine_delta.abs() < 1e-10);
        assert!(result.euclidean_delta.abs() < 1e-10);
    }

    #[test]
    fn test_different_vectors_positive_delta() {
        let v1 = FeatureVector {
            names: vec!["a".into(), "b".into(), "c".into()],
            values: vec![1.0, 2.0, 3.0],
        };
        let v2 = FeatureVector {
            names: vec!["a".into(), "b".into(), "c".into()],
            values: vec![4.0, 5.0, 6.0],
        };
        let result = delta_from_vectors(&v1, &v2);

        assert!(result.classic_delta > 0.0);
        assert!(result.cosine_delta > 0.0);
        assert!(result.euclidean_delta > 0.0);
    }

    #[test]
    fn test_delta_is_symmetric() {
        let v1 = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![1.0, 5.0],
        };
        let v2 = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![3.0, 1.0],
        };

        let d1 = delta_from_vectors(&v1, &v2);
        let d2 = delta_from_vectors(&v2, &v1);

        assert!((d1.classic_delta - d2.classic_delta).abs() < 1e-10);
        assert!((d1.cosine_delta - d2.cosine_delta).abs() < 1e-10);
    }

    #[test]
    fn test_rank_candidates() {
        let query = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![1.0, 2.0],
        };
        let close = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![1.1, 2.1],
        };
        let far = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![10.0, 20.0],
        };

        let candidates = vec![
            ("far_author".to_string(), far),
            ("close_author".to_string(), close),
        ];

        let all_vecs: Vec<FeatureVector> = std::iter::once(query.clone())
            .chain(candidates.iter().map(|(_, v)| v.clone()))
            .collect();
        let stats = CorpusStats::from_vectors(&all_vecs).unwrap();

        let ranked = rank_candidates(&query, &candidates, &stats);

        assert_eq!(ranked[0].0, "close_author");
        assert_eq!(ranked[1].0, "far_author");
        assert!(ranked[0].1.cosine_delta < ranked[1].1.cosine_delta);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = Array1::from_vec(vec![1.0, 0.0]);
        let b = Array1::from_vec(vec![0.0, 1.0]);
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }
}
