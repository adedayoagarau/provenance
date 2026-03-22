use ndarray::Array1;

/// Cosine similarity between two vectors. Range: [-1, 1]. Higher = more similar.
pub fn cosine_similarity(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    let dot = a.dot(b);
    let norm_a = a.dot(a).sqrt();
    let norm_b = b.dot(b).sqrt();

    if norm_a < f64::EPSILON || norm_b < f64::EPSILON {
        return 0.0;
    }

    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

/// Cosine distance. Range: [0, 2]. Lower = more similar.
pub fn cosine_distance(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    1.0 - cosine_similarity(a, b)
}

/// Manhattan (L1) distance.
pub fn manhattan_distance(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    (a - b).mapv(f64::abs).sum()
}

/// Euclidean (L2) distance.
pub fn euclidean_distance(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    let diff = a - b;
    diff.dot(&diff).sqrt()
}

/// Kullback-Leibler divergence. KL(P || Q).
///
/// Input distributions must be positive and sum to ~1.
/// Returns f64::INFINITY if Q has zero where P is non-zero.
pub fn kl_divergence(p: &Array1<f64>, q: &Array1<f64>) -> f64 {
    let mut kl = 0.0;
    for i in 0..p.len() {
        if p[i] > 1e-10 {
            if q[i] < 1e-10 {
                return f64::INFINITY;
            }
            kl += p[i] * (p[i] / q[i]).ln();
        }
    }
    kl
}

/// Jensen-Shannon divergence (symmetric KL). Range: [0, ln(2)].
///
/// JSD(P, Q) = 0.5 * KL(P || M) + 0.5 * KL(Q || M) where M = 0.5*(P+Q)
pub fn jensen_shannon_divergence(p: &Array1<f64>, q: &Array1<f64>) -> f64 {
    let m = (p + q) * 0.5;
    0.5 * kl_divergence(p, &m) + 0.5 * kl_divergence(q, &m)
}

/// Chi-squared distance between two distributions.
///
/// χ²(P, Q) = Σ (p_i - q_i)² / (p_i + q_i) for all i where (p_i + q_i) > 0
pub fn chi_squared_distance(p: &Array1<f64>, q: &Array1<f64>) -> f64 {
    let mut chi2 = 0.0;
    for i in 0..p.len() {
        let sum = p[i] + q[i];
        if sum > 1e-10 {
            let diff = p[i] - q[i];
            chi2 += diff * diff / sum;
        }
    }
    chi2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical() {
        let a = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-10);
        assert!(cosine_distance(&a, &a).abs() < 1e-10);
    }

    #[test]
    fn test_manhattan() {
        let a = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array1::from_vec(vec![4.0, 5.0, 6.0]);
        assert!((manhattan_distance(&a, &b) - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_euclidean() {
        let a = Array1::from_vec(vec![0.0, 0.0]);
        let b = Array1::from_vec(vec![3.0, 4.0]);
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_jsd_identical() {
        let p = Array1::from_vec(vec![0.5, 0.3, 0.2]);
        assert!(jensen_shannon_divergence(&p, &p).abs() < 1e-10);
    }

    #[test]
    fn test_jsd_symmetric() {
        let p = Array1::from_vec(vec![0.5, 0.3, 0.2]);
        let q = Array1::from_vec(vec![0.1, 0.6, 0.3]);
        let d1 = jensen_shannon_divergence(&p, &q);
        let d2 = jensen_shannon_divergence(&q, &p);
        assert!((d1 - d2).abs() < 1e-10);
    }

    #[test]
    fn test_chi_squared_identical() {
        let p = Array1::from_vec(vec![0.5, 0.3, 0.2]);
        assert!(chi_squared_distance(&p, &p).abs() < 1e-10);
    }
}
