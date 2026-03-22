//! ML model abstractions and implementations for authorship attribution.
//!
//! Provides a unified `Classifier` trait with implementations:
//! - K-Nearest Neighbors (built-in, no external deps)
//! - SVM, Random Forest, Logistic Regression (via `linfa` feature)

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};

use crate::identity::distances;

/// Prediction result from a classifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    /// Predicted class label
    pub label: String,
    /// Confidence/probability for the predicted class (0.0–1.0)
    pub confidence: f64,
    /// Per-class probabilities (label → probability)
    pub class_probabilities: Vec<(String, f64)>,
}

/// Model type identifier for serialization and registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    KNearestNeighbors,
    SupportVectorMachine,
    RandomForest,
    LogisticRegression,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KNearestNeighbors => write!(f, "k-NN"),
            Self::SupportVectorMachine => write!(f, "SVM"),
            Self::RandomForest => write!(f, "Random Forest"),
            Self::LogisticRegression => write!(f, "Logistic Regression"),
        }
    }
}

/// Trait for authorship classifiers.
pub trait Classifier: Send + Sync {
    /// Train the model on labeled feature vectors.
    fn train(&mut self, features: &Array2<f64>, labels: &[String]) -> std::result::Result<(), String>;

    /// Predict the class for a single feature vector.
    fn predict(&self, features: &Array1<f64>) -> std::result::Result<Prediction, String>;

    /// Predict classes for a batch of feature vectors.
    fn predict_batch(&self, features: &Array2<f64>) -> std::result::Result<Vec<Prediction>, String> {
        let mut predictions = Vec::with_capacity(features.nrows());
        for row in features.rows() {
            predictions.push(self.predict(&row.to_owned())?);
        }
        Ok(predictions)
    }

    /// Model type identifier.
    fn model_type(&self) -> ModelType;

    /// Whether the model has been trained.
    fn is_trained(&self) -> bool;
}

// ─── K-Nearest Neighbors (built-in) ───────────────────────────────────────

/// K-Nearest Neighbors classifier using cosine distance.
///
/// Uses Vec<Vec<f64>> internally for serde compatibility (ndarray
/// doesn't implement Serialize/Deserialize without feature flags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnnClassifier {
    k: usize,
    train_features: Option<Vec<Vec<f64>>>,
    train_labels: Option<Vec<String>>,
}

impl KnnClassifier {
    pub fn new(k: usize) -> Self {
        Self {
            k: k.max(1),
            train_features: None,
            train_labels: None,
        }
    }
}

impl Classifier for KnnClassifier {
    fn train(&mut self, features: &Array2<f64>, labels: &[String]) -> std::result::Result<(), String> {
        if features.nrows() != labels.len() {
            return Err(format!(
                "Feature rows ({}) != label count ({})",
                features.nrows(),
                labels.len()
            ));
        }
        if features.nrows() == 0 {
            return Err("Cannot train on empty dataset".to_string());
        }
        // Store as Vec<Vec<f64>> for serde compatibility
        let rows: Vec<Vec<f64>> = features
            .rows()
            .into_iter()
            .map(|r| r.to_vec())
            .collect();
        self.train_features = Some(rows);
        self.train_labels = Some(labels.to_vec());
        Ok(())
    }

    fn predict(&self, features: &Array1<f64>) -> std::result::Result<Prediction, String> {
        let train_features = self
            .train_features
            .as_ref()
            .ok_or("Model not trained")?;
        let train_labels = self
            .train_labels
            .as_ref()
            .ok_or("Model not trained")?;

        let query = features.clone();

        // Compute Euclidean distances to all training points
        let mut dists: Vec<(usize, f64)> = train_features
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let row_arr = Array1::from_vec(row.clone());
                let d = distances::euclidean_distance(&row_arr, &query);
                (i, d)
            })
            .collect();

        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = self.k.min(dists.len());
        let neighbors = &dists[..k];

        // Vote by inverse distance weighting
        let mut votes: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
        for &(idx, dist) in neighbors {
            let weight = 1.0 / (dist + 1e-10);
            *votes.entry(&train_labels[idx]).or_default() += weight;
        }

        let total_weight: f64 = votes.values().sum();
        let mut class_probs: Vec<(String, f64)> = votes
            .iter()
            .map(|(&label, &weight)| (label.to_string(), weight / total_weight))
            .collect();
        class_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let best = &class_probs[0];
        Ok(Prediction {
            label: best.0.clone(),
            confidence: best.1,
            class_probabilities: class_probs,
        })
    }

    fn model_type(&self) -> ModelType {
        ModelType::KNearestNeighbors
    }

    fn is_trained(&self) -> bool {
        self.train_features.is_some()
    }
}

// ─── SVM (via linfa, feature-gated) ───────────────────────────────────────

/// SVM classifier using linfa-svm.
///
/// Requires the `linfa` feature flag. Uses RBF kernel with configurable
/// regularization parameter C and gamma.
#[cfg(feature = "linfa")]
pub struct SvmClassifier {
    c: f64,
    models: Option<Vec<(String, linfa_svm::Svm<f64, bool>)>>,
    labels: Vec<String>,
}

#[cfg(feature = "linfa")]
impl SvmClassifier {
    pub fn new(c: f64) -> Self {
        Self {
            c,
            models: None,
            labels: Vec::new(),
        }
    }
}

#[cfg(feature = "linfa")]
impl Classifier for SvmClassifier {
    fn train(&mut self, features: &Array2<f64>, labels: &[String]) -> std::result::Result<(), String> {
        use linfa::prelude::*;
        use linfa_svm::Svm;

        let unique_labels: Vec<String> = {
            let mut v: Vec<String> = labels.iter().cloned().collect();
            v.sort();
            v.dedup();
            v
        };

        // One-vs-rest: train a binary SVM for each class
        let mut models = Vec::new();
        for target_label in &unique_labels {
            let binary_labels: Array1<bool> = Array1::from_vec(
                labels.iter().map(|l| l == target_label).collect(),
            );

            let dataset = DatasetBase::new(features.clone(), binary_labels);

            let model = Svm::<_, bool>::params()
                .pos_neg_weights(self.c, self.c)
                .gaussian_kernel(1.0 / features.ncols() as f64)
                .fit(&dataset)
                .map_err(|e| format!("SVM training failed for class '{target_label}': {e}"))?;

            models.push((target_label.clone(), model));
        }

        self.labels = unique_labels;
        self.models = Some(models);
        Ok(())
    }

    fn predict(&self, features: &Array1<f64>) -> std::result::Result<Prediction, String> {
        use linfa::prelude::*;

        let models = self.models.as_ref().ok_or("Model not trained")?;

        let sample = features.clone().insert_axis(ndarray::Axis(0));
        let mut scores: Vec<(String, f64)> = Vec::new();

        for (label, model) in models {
            let pred = model.predict(&sample);
            let score = if pred[0] { 1.0 } else { 0.0 };
            scores.push((label.clone(), score));
        }

        // Normalize to probabilities
        let total: f64 = scores.iter().map(|(_, s)| s).sum::<f64>().max(1e-10);
        let mut class_probs: Vec<(String, f64)> = scores
            .iter()
            .map(|(l, s)| (l.clone(), s / total))
            .collect();
        class_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best = &class_probs[0];
        Ok(Prediction {
            label: best.0.clone(),
            confidence: best.1,
            class_probabilities: class_probs,
        })
    }

    fn model_type(&self) -> ModelType {
        ModelType::SupportVectorMachine
    }

    fn is_trained(&self) -> bool {
        self.models.is_some()
    }
}

// ─── Logistic Regression (via linfa, feature-gated) ──────────────────────

/// Logistic Regression classifier using linfa-logistic.
///
/// Produces calibrated probabilities, useful as a baseline and
/// for confidence estimation.
#[cfg(feature = "linfa")]
pub struct LogisticClassifier {
    max_iterations: u64,
    models: Option<Vec<(String, linfa_logistic::LogisticRegression<f64>)>>,
    labels: Vec<String>,
}

#[cfg(feature = "linfa")]
impl LogisticClassifier {
    pub fn new(max_iterations: u64) -> Self {
        Self {
            max_iterations,
            models: None,
            labels: Vec::new(),
        }
    }
}

// ─── Helper: build feature matrix from labeled samples ───────────────────

/// Convert labeled samples into an ndarray feature matrix and label vector.
pub fn samples_to_matrix(
    samples: &[super::features::LabeledSample],
) -> (Array2<f64>, Vec<String>) {
    if samples.is_empty() {
        return (Array2::zeros((0, 0)), Vec::new());
    }

    let n_features = samples[0].features.len();
    let n_samples = samples.len();

    let mut matrix = Array2::zeros((n_samples, n_features));
    let mut labels = Vec::with_capacity(n_samples);

    for (i, sample) in samples.iter().enumerate() {
        for (j, &val) in sample.features.values.iter().enumerate() {
            if j < n_features {
                matrix[[i, j]] = val;
            }
        }
        labels.push(sample.label.clone());
    }

    (matrix, labels)
}

/// Get unique class labels sorted alphabetically.
pub fn unique_labels(labels: &[String]) -> Vec<String> {
    let mut unique: Vec<String> = labels.to_vec();
    unique.sort();
    unique.dedup();
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knn_basic() {
        let mut knn = KnnClassifier::new(3);
        assert!(!knn.is_trained());

        // Simple 2D classification problem
        let features = Array2::from_shape_vec(
            (6, 2),
            vec![
                0.0, 0.0, // A
                0.1, 0.1, // A
                0.2, 0.0, // A
                1.0, 1.0, // B
                1.1, 1.1, // B
                1.0, 1.2, // B
            ],
        )
        .unwrap();

        let labels = vec![
            "A".into(), "A".into(), "A".into(),
            "B".into(), "B".into(), "B".into(),
        ];

        knn.train(&features, &labels).unwrap();
        assert!(knn.is_trained());

        // Point near cluster A
        let query = Array1::from_vec(vec![0.05, 0.05]);
        let pred = knn.predict(&query).unwrap();
        assert_eq!(pred.label, "A");
        assert!(pred.confidence > 0.5);

        // Point near cluster B
        let query = Array1::from_vec(vec![1.05, 1.05]);
        let pred = knn.predict(&query).unwrap();
        assert_eq!(pred.label, "B");
    }

    #[test]
    fn test_samples_to_matrix() {
        use crate::identity::features::FeatureVector;
        use super::super::features::LabeledSample;

        let samples = vec![
            LabeledSample {
                label: "alice".into(),
                source: "a.txt".into(),
                features: FeatureVector {
                    names: vec!["f1".into(), "f2".into()],
                    values: vec![1.0, 2.0],
                },
                word_count: 100,
            },
            LabeledSample {
                label: "bob".into(),
                source: "b.txt".into(),
                features: FeatureVector {
                    names: vec!["f1".into(), "f2".into()],
                    values: vec![3.0, 4.0],
                },
                word_count: 150,
            },
        ];

        let (matrix, labels) = samples_to_matrix(&samples);
        assert_eq!(matrix.nrows(), 2);
        assert_eq!(matrix.ncols(), 2);
        assert_eq!(labels, vec!["alice", "bob"]);
        assert_eq!(matrix[[0, 0]], 1.0);
        assert_eq!(matrix[[1, 1]], 4.0);
    }

    #[test]
    fn test_unique_labels() {
        let labels = vec![
            "bob".into(), "alice".into(), "bob".into(), "carol".into(), "alice".into(),
        ];
        let unique = unique_labels(&labels);
        assert_eq!(unique, vec!["alice", "bob", "carol"]);
    }
}
