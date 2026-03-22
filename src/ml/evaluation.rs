//! Evaluation framework for authorship attribution models.
//!
//! K-fold cross-validation, classification metrics (accuracy, precision,
//! recall, F1, ROC-AUC approximation, EER), confusion matrices,
//! and per-author performance breakdown.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::features::LabeledSample;
use super::models::{self, Prediction};
use super::pipeline::{self, PipelineConfig};
use crate::utils::errors::{ProvenanceError, Result};

/// Complete evaluation report from cross-validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationReport {
    /// Overall metrics averaged across folds
    pub overall: ClassificationMetrics,
    /// Per-author metrics
    pub per_author: HashMap<String, ClassificationMetrics>,
    /// Confusion matrix (actual → predicted → count)
    pub confusion_matrix: ConfusionMatrix,
    /// Number of folds used
    pub n_folds: usize,
    /// Total samples evaluated
    pub n_samples: usize,
    /// Per-fold accuracy (for variance estimation)
    pub fold_accuracies: Vec<f64>,
}

/// Classification metrics for binary or multi-class problems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetrics {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    /// Equal Error Rate (where FPR = FNR)
    pub eer: f64,
    /// Number of correct predictions
    pub correct: usize,
    /// Total predictions
    pub total: usize,
}

/// Confusion matrix tracking actual vs predicted labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfusionMatrix {
    /// Ordered class labels
    pub labels: Vec<String>,
    /// Matrix[i][j] = count where actual=labels[i], predicted=labels[j]
    pub matrix: Vec<Vec<usize>>,
}

impl ConfusionMatrix {
    /// Create an empty confusion matrix for the given labels.
    pub fn new(labels: Vec<String>) -> Self {
        let n = labels.len();
        Self {
            labels,
            matrix: vec![vec![0; n]; n],
        }
    }

    /// Record a prediction.
    pub fn record(&mut self, actual: &str, predicted: &str) {
        if let (Some(i), Some(j)) = (
            self.labels.iter().position(|l| l == actual),
            self.labels.iter().position(|l| l == predicted),
        ) {
            self.matrix[i][j] += 1;
        }
    }

    /// Get true positives for a class.
    pub fn true_positives(&self, class_idx: usize) -> usize {
        self.matrix[class_idx][class_idx]
    }

    /// Get false positives for a class (other classes predicted as this class).
    pub fn false_positives(&self, class_idx: usize) -> usize {
        let mut fp = 0;
        for i in 0..self.labels.len() {
            if i != class_idx {
                fp += self.matrix[i][class_idx];
            }
        }
        fp
    }

    /// Get false negatives for a class (this class predicted as other classes).
    pub fn false_negatives(&self, class_idx: usize) -> usize {
        let mut fn_count = 0;
        for j in 0..self.labels.len() {
            if j != class_idx {
                fn_count += self.matrix[class_idx][j];
            }
        }
        fn_count
    }

    /// Total correct predictions (trace of matrix).
    pub fn total_correct(&self) -> usize {
        (0..self.labels.len()).map(|i| self.matrix[i][i]).sum()
    }

    /// Total predictions.
    pub fn total(&self) -> usize {
        self.matrix.iter().flat_map(|row| row.iter()).sum()
    }
}

impl std::fmt::Display for ConfusionMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Header
        write!(f, "{:>12}", "Actual\\Pred")?;
        for label in &self.labels {
            write!(f, " {:>10}", label)?;
        }
        writeln!(f)?;

        // Rows
        for (i, label) in self.labels.iter().enumerate() {
            write!(f, "{:>12}", label)?;
            for j in 0..self.labels.len() {
                write!(f, " {:>10}", self.matrix[i][j])?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

/// Run k-fold cross-validation on labeled samples.
///
/// Splits data into `k` folds, trains on k-1, tests on 1, rotates.
/// Returns an evaluation report with aggregated metrics.
pub fn cross_validate(
    samples: &[LabeledSample],
    config: &PipelineConfig,
    k: usize,
) -> Result<EvaluationReport> {
    let k = k.max(2).min(samples.len());

    if samples.len() < k {
        return Err(ProvenanceError::AnalysisError {
            reason: format!("Need at least {k} samples for {k}-fold CV, got {}", samples.len()),
        });
    }

    let labels = models::unique_labels(
        &samples.iter().map(|s| s.label.clone()).collect::<Vec<_>>(),
    );

    if labels.len() < 2 {
        return Err(ProvenanceError::AnalysisError {
            reason: "Need at least 2 classes for cross-validation".to_string(),
        });
    }

    // Stratified fold assignment (round-robin within each class)
    let fold_indices = stratified_folds(samples, k);

    let mut all_predictions: Vec<(String, String)> = Vec::new(); // (actual, predicted)
    let mut fold_accuracies = Vec::with_capacity(k);

    for fold in 0..k {
        let mut train_set = Vec::new();
        let mut test_set = Vec::new();

        for (i, sample) in samples.iter().enumerate() {
            if fold_indices[i] == fold {
                test_set.push(sample.clone());
            } else {
                train_set.push(sample.clone());
            }
        }

        if test_set.is_empty() || train_set.is_empty() {
            continue;
        }

        // Check that training set has 2+ classes
        let train_classes = models::unique_labels(
            &train_set.iter().map(|s| s.label.clone()).collect::<Vec<_>>(),
        );
        if train_classes.len() < 2 {
            continue;
        }

        let fold_config = PipelineConfig {
            min_word_count: 0, // Already filtered by caller
            ..config.clone()
        };

        match pipeline::train_predict(&train_set, &test_set, fold_config) {
            Ok(results) => {
                let mut correct = 0;
                for (actual, pred) in &results {
                    all_predictions.push((actual.clone(), pred.label.clone()));
                    if actual == &pred.label {
                        correct += 1;
                    }
                }
                fold_accuracies.push(correct as f64 / results.len() as f64);
            }
            Err(_) => {
                // Skip failed folds
                continue;
            }
        }
    }

    if all_predictions.is_empty() {
        return Err(ProvenanceError::AnalysisError {
            reason: "All folds failed during cross-validation".to_string(),
        });
    }

    // Build confusion matrix
    let mut cm = ConfusionMatrix::new(labels.clone());
    for (actual, predicted) in &all_predictions {
        cm.record(actual, predicted);
    }

    // Compute overall metrics
    let overall = compute_overall_metrics(&cm);

    // Compute per-author metrics
    let mut per_author = HashMap::new();
    for (idx, label) in labels.iter().enumerate() {
        per_author.insert(label.clone(), compute_class_metrics(&cm, idx));
    }

    Ok(EvaluationReport {
        overall,
        per_author,
        confusion_matrix: cm,
        n_folds: fold_accuracies.len(),
        n_samples: all_predictions.len(),
        fold_accuracies,
    })
}

/// Evaluate predictions against ground truth (no cross-validation).
pub fn evaluate(
    predictions: &[(String, Prediction)],
) -> (ClassificationMetrics, ConfusionMatrix) {
    let labels = models::unique_labels(
        &predictions
            .iter()
            .map(|(actual, _)| actual.clone())
            .collect::<Vec<_>>(),
    );

    let mut cm = ConfusionMatrix::new(labels);
    for (actual, pred) in predictions {
        cm.record(actual, &pred.label);
    }

    let metrics = compute_overall_metrics(&cm);
    (metrics, cm)
}

/// Assign stratified fold indices (round-robin within each class).
fn stratified_folds(samples: &[LabeledSample], k: usize) -> Vec<usize> {
    let mut fold_indices = vec![0usize; samples.len()];

    // Group indices by class
    let mut class_indices: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, sample) in samples.iter().enumerate() {
        class_indices
            .entry(&sample.label)
            .or_default()
            .push(i);
    }

    // Round-robin assign folds within each class
    for indices in class_indices.values() {
        for (j, &idx) in indices.iter().enumerate() {
            fold_indices[idx] = j % k;
        }
    }

    fold_indices
}

/// Compute overall metrics from a confusion matrix (macro-averaged).
fn compute_overall_metrics(cm: &ConfusionMatrix) -> ClassificationMetrics {
    let total = cm.total();
    let correct = cm.total_correct();
    let accuracy = if total > 0 { correct as f64 / total as f64 } else { 0.0 };

    // Macro-averaged precision, recall, F1
    let n_classes = cm.labels.len();
    let mut total_precision = 0.0;
    let mut total_recall = 0.0;
    let mut valid_classes = 0;

    for idx in 0..n_classes {
        let tp = cm.true_positives(idx) as f64;
        let fp = cm.false_positives(idx) as f64;
        let fn_count = cm.false_negatives(idx) as f64;

        if tp + fp > 0.0 {
            total_precision += tp / (tp + fp);
            valid_classes += 1;
        }
        if tp + fn_count > 0.0 {
            total_recall += tp / (tp + fn_count);
        }
    }

    let precision = if valid_classes > 0 {
        total_precision / valid_classes as f64
    } else {
        0.0
    };
    let recall = if n_classes > 0 {
        total_recall / n_classes as f64
    } else {
        0.0
    };
    let f1_score = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    // EER approximation: where FPR ≈ FNR
    let eer = approximate_eer(cm);

    ClassificationMetrics {
        accuracy,
        precision,
        recall,
        f1_score,
        eer,
        correct,
        total,
    }
}

/// Compute per-class metrics from a confusion matrix.
fn compute_class_metrics(cm: &ConfusionMatrix, class_idx: usize) -> ClassificationMetrics {
    let tp = cm.true_positives(class_idx) as f64;
    let fp = cm.false_positives(class_idx) as f64;
    let fn_count = cm.false_negatives(class_idx) as f64;

    let precision = if tp + fp > 0.0 { tp / (tp + fp) } else { 0.0 };
    let recall = if tp + fn_count > 0.0 { tp / (tp + fn_count) } else { 0.0 };
    let f1_score = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    let total_for_class = (tp + fn_count) as usize;
    let correct = tp as usize;

    ClassificationMetrics {
        accuracy: if total_for_class > 0 { tp / (tp + fn_count) } else { 0.0 },
        precision,
        recall,
        f1_score,
        eer: 0.0, // EER is only meaningful at the overall level
        correct,
        total: total_for_class,
    }
}

/// Approximate Equal Error Rate from confusion matrix.
///
/// For multi-class, averages per-class EER approximations.
fn approximate_eer(cm: &ConfusionMatrix) -> f64 {
    let n_classes = cm.labels.len();
    if n_classes == 0 {
        return 0.0;
    }

    let total = cm.total() as f64;
    if total == 0.0 {
        return 0.0;
    }

    let mut eer_sum = 0.0;
    for idx in 0..n_classes {
        let tp = cm.true_positives(idx) as f64;
        let fp = cm.false_positives(idx) as f64;
        let fn_count = cm.false_negatives(idx) as f64;
        let tn = total - tp - fp - fn_count;

        let fpr = if fp + tn > 0.0 { fp / (fp + tn) } else { 0.0 };
        let fnr = if fn_count + tp > 0.0 { fn_count / (fn_count + tp) } else { 0.0 };

        // EER approximation: average of FPR and FNR at the operating point
        eer_sum += (fpr + fnr) / 2.0;
    }

    eer_sum / n_classes as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confusion_matrix() {
        let mut cm = ConfusionMatrix::new(vec!["A".into(), "B".into(), "C".into()]);

        // Perfect predictions for A, some confusion for B/C
        cm.record("A", "A"); // TP for A
        cm.record("A", "A");
        cm.record("B", "B"); // TP for B
        cm.record("B", "C"); // FN for B, FP for C
        cm.record("C", "C"); // TP for C
        cm.record("C", "B"); // FN for C, FP for B

        assert_eq!(cm.total(), 6);
        assert_eq!(cm.total_correct(), 4);
        assert_eq!(cm.true_positives(0), 2); // A: 2 TP
        assert_eq!(cm.false_positives(0), 0); // A: 0 FP
        assert_eq!(cm.false_negatives(0), 0); // A: 0 FN
        assert_eq!(cm.true_positives(1), 1); // B: 1 TP
        assert_eq!(cm.false_positives(1), 1); // B: 1 FP (C→B)
        assert_eq!(cm.false_negatives(1), 1); // B: 1 FN (B→C)
    }

    #[test]
    fn test_overall_metrics_perfect() {
        let mut cm = ConfusionMatrix::new(vec!["A".into(), "B".into()]);
        cm.record("A", "A");
        cm.record("A", "A");
        cm.record("B", "B");
        cm.record("B", "B");

        let metrics = compute_overall_metrics(&cm);
        assert!((metrics.accuracy - 1.0).abs() < 1e-10);
        assert!((metrics.precision - 1.0).abs() < 1e-10);
        assert!((metrics.recall - 1.0).abs() < 1e-10);
        assert!((metrics.f1_score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_overall_metrics_random() {
        let mut cm = ConfusionMatrix::new(vec!["A".into(), "B".into()]);
        // All predicted as A
        cm.record("A", "A");
        cm.record("A", "A");
        cm.record("B", "A");
        cm.record("B", "A");

        let metrics = compute_overall_metrics(&cm);
        assert!((metrics.accuracy - 0.5).abs() < 1e-10);
        // Precision for A = 2/4 = 0.5, B = 0/0 → only 1 valid class
        assert!((metrics.precision - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_stratified_folds() {
        use crate::identity::features::FeatureVector;

        let samples: Vec<LabeledSample> = vec![
            LabeledSample { label: "A".into(), source: "".into(), features: FeatureVector { names: vec![], values: vec![] }, word_count: 100 },
            LabeledSample { label: "A".into(), source: "".into(), features: FeatureVector { names: vec![], values: vec![] }, word_count: 100 },
            LabeledSample { label: "B".into(), source: "".into(), features: FeatureVector { names: vec![], values: vec![] }, word_count: 100 },
            LabeledSample { label: "B".into(), source: "".into(), features: FeatureVector { names: vec![], values: vec![] }, word_count: 100 },
        ];

        let folds = stratified_folds(&samples, 2);
        // Each class should have samples in both folds
        assert_eq!(folds[0], 0); // A-first → fold 0
        assert_eq!(folds[1], 1); // A-second → fold 1
        assert_eq!(folds[2], 0); // B-first → fold 0
        assert_eq!(folds[3], 1); // B-second → fold 1
    }

    #[test]
    fn test_evaluate_predictions() {
        use super::super::models::Prediction;

        let predictions = vec![
            ("A".to_string(), Prediction { label: "A".into(), confidence: 0.9, class_probabilities: vec![] }),
            ("A".to_string(), Prediction { label: "B".into(), confidence: 0.6, class_probabilities: vec![] }),
            ("B".to_string(), Prediction { label: "B".into(), confidence: 0.8, class_probabilities: vec![] }),
            ("B".to_string(), Prediction { label: "B".into(), confidence: 0.7, class_probabilities: vec![] }),
        ];

        let (metrics, cm) = evaluate(&predictions);
        assert_eq!(metrics.correct, 3);
        assert_eq!(metrics.total, 4);
        assert!((metrics.accuracy - 0.75).abs() < 1e-10);
        assert_eq!(cm.total_correct(), 3);
    }
}
