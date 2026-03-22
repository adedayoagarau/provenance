//! Training and prediction pipeline for authorship attribution.
//!
//! Orchestrates feature extraction → normalization → model training → prediction.

use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::identity::features::{CorpusStats, FeatureSet, FeatureVector};
use crate::utils::errors::{ProvenanceError, Result};

use super::features::{FeaturePipeline, LabeledSample};
use super::models::{self, Classifier, KnnClassifier, ModelType, Prediction};

/// Configuration for the ML pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Feature set to use
    pub feature_set: FeatureSet,
    /// Model type to train
    pub model_type: ModelType,
    /// K for k-NN (only used if model_type is KNearestNeighbors)
    pub knn_k: usize,
    /// Whether to z-score normalize features
    pub normalize: bool,
    /// Minimum word count to include a sample
    pub min_word_count: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            feature_set: FeatureSet::Standard,
            model_type: ModelType::KNearestNeighbors,
            knn_k: 5,
            normalize: true,
            min_word_count: 100,
        }
    }
}

/// Serializable feature set config for serde compatibility.
impl Serialize for FeatureSet {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            FeatureSet::Minimal => serializer.serialize_str("minimal"),
            FeatureSet::Standard => serializer.serialize_str("standard"),
            FeatureSet::Comprehensive => serializer.serialize_str("comprehensive"),
        }
    }
}

impl<'de> Deserialize<'de> for FeatureSet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "minimal" => Ok(FeatureSet::Minimal),
            "standard" => Ok(FeatureSet::Standard),
            "comprehensive" => Ok(FeatureSet::Comprehensive),
            _ => Err(serde::de::Error::custom(format!("unknown feature set: {s}"))),
        }
    }
}

/// A trained pipeline ready for prediction.
#[derive(Serialize, Deserialize)]
pub struct TrainedPipeline {
    /// Pipeline configuration
    pub config: PipelineConfig,
    /// Corpus statistics for normalization (if normalize=true)
    pub corpus_stats: Option<CorpusStats>,
    /// Training class labels (for reference)
    pub class_labels: Vec<String>,
    /// Number of training samples
    pub n_train_samples: usize,
    /// Number of features
    pub n_features: usize,
    /// The trained model (k-NN stores its data directly)
    #[serde(skip)]
    model: Option<Box<dyn Classifier>>,
    /// Serializable model data for k-NN
    knn_data: Option<KnnClassifier>,
}

impl TrainedPipeline {
    /// Predict the author of a feature vector.
    pub fn predict(&self, features: &FeatureVector) -> Result<Prediction> {
        let model = self.model.as_ref().ok_or(ProvenanceError::AnalysisError {
            reason: "Pipeline model not loaded".to_string(),
        })?;

        let input = if let Some(stats) = &self.corpus_stats {
            stats.normalize(features)
        } else {
            features.to_array()
        };

        model
            .predict(&input)
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("Prediction failed: {e}"),
            })
    }

    /// Predict the author of raw text.
    pub fn predict_text(&self, text: &str) -> Result<Prediction> {
        let pipeline = FeaturePipeline::new(self.config.feature_set);
        let (features, _word_count) = pipeline.extract_from_text(text)?;
        self.predict(&features)
    }

    /// Save the trained pipeline to a JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            ProvenanceError::AnalysisError {
                reason: format!("Serialization failed: {e}"),
            }
        })?;
        std::fs::write(path, json).map_err(|e| ProvenanceError::IoWithPath {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Load a trained pipeline from a JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let json = crate::utils::errors::read_file_string(path)?;
        let mut pipeline: TrainedPipeline =
            serde_json::from_str(&json).map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("Deserialization failed: {e}"),
            })?;

        // Restore the model from serialized data
        if let Some(knn) = &pipeline.knn_data {
            pipeline.model = Some(Box::new(knn.clone()));
        }

        Ok(pipeline)
    }
}

/// Train a pipeline from labeled samples.
pub fn train(samples: &[LabeledSample], config: PipelineConfig) -> Result<TrainedPipeline> {
    // Filter by minimum word count
    let filtered: Vec<&LabeledSample> = samples
        .iter()
        .filter(|s| s.word_count >= config.min_word_count)
        .collect();

    if filtered.len() < 2 {
        return Err(ProvenanceError::AnalysisError {
            reason: format!(
                "Need at least 2 samples with >= {} words, got {}",
                config.min_word_count,
                filtered.len()
            ),
        });
    }

    let class_labels = models::unique_labels(
        &filtered.iter().map(|s| s.label.clone()).collect::<Vec<_>>(),
    );

    if class_labels.len() < 2 {
        return Err(ProvenanceError::AnalysisError {
            reason: "Need at least 2 distinct author classes".to_string(),
        });
    }

    // Build feature matrix
    let owned_samples: Vec<LabeledSample> = filtered.into_iter().cloned().collect();
    let (raw_matrix, labels) = models::samples_to_matrix(&owned_samples);

    // Optionally normalize
    let (matrix, corpus_stats) = if config.normalize {
        let vectors: Vec<FeatureVector> = owned_samples
            .iter()
            .map(|s| s.features.clone())
            .collect();
        let stats = CorpusStats::from_vectors(&vectors).ok_or(ProvenanceError::AnalysisError {
            reason: "Failed to compute corpus statistics".to_string(),
        })?;

        let mut normalized = Array2::zeros(raw_matrix.dim());
        for (i, sample) in owned_samples.iter().enumerate() {
            let norm = stats.normalize(&sample.features);
            for (j, &val) in norm.iter().enumerate() {
                if j < normalized.ncols() {
                    normalized[[i, j]] = val;
                }
            }
        }

        (normalized, Some(stats))
    } else {
        (raw_matrix, None)
    };

    // Train model
    let n_features = matrix.ncols();
    let mut model: Box<dyn Classifier> = match config.model_type {
        ModelType::KNearestNeighbors => Box::new(KnnClassifier::new(config.knn_k)),
        #[cfg(feature = "linfa")]
        ModelType::SupportVectorMachine => {
            Box::new(models::SvmClassifier::new(1.0))
        }
        #[cfg(not(feature = "linfa"))]
        ModelType::SupportVectorMachine => {
            return Err(ProvenanceError::AnalysisError {
                reason: "SVM requires the 'linfa' feature flag".to_string(),
            });
        }
        #[cfg(not(feature = "linfa"))]
        ModelType::RandomForest | ModelType::LogisticRegression => {
            return Err(ProvenanceError::AnalysisError {
                reason: format!("{} requires the 'linfa' feature flag", config.model_type),
            });
        }
        #[cfg(feature = "linfa")]
        ModelType::LogisticRegression => {
            Box::new(models::LogisticClassifier::new(100))
        }
        #[cfg(feature = "linfa")]
        ModelType::RandomForest => {
            return Err(ProvenanceError::AnalysisError {
                reason: "Random Forest not yet implemented".to_string(),
            });
        }
    };

    model
        .train(&matrix, &labels)
        .map_err(|e| ProvenanceError::AnalysisError {
            reason: format!("Training failed: {e}"),
        })?;

    // Save k-NN data for serialization
    let knn_data = if config.model_type == ModelType::KNearestNeighbors {
        // Reconstruct KnnClassifier for serialization
        let mut knn = KnnClassifier::new(config.knn_k);
        knn.train(&matrix, &labels).ok();
        Some(knn)
    } else {
        None
    };

    Ok(TrainedPipeline {
        config,
        corpus_stats,
        class_labels,
        n_train_samples: owned_samples.len(),
        n_features,
        model: Some(model),
        knn_data,
    })
}

/// Train and immediately predict on a holdout set (convenience function).
pub fn train_predict(
    train_samples: &[LabeledSample],
    test_samples: &[LabeledSample],
    config: PipelineConfig,
) -> Result<Vec<(String, Prediction)>> {
    let pipeline = train(train_samples, config)?;

    let mut results = Vec::with_capacity(test_samples.len());
    for sample in test_samples {
        let pred = pipeline.predict(&sample.features)?;
        results.push((sample.label.clone(), pred));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::features::FeatureVector;

    fn make_sample(label: &str, values: Vec<f64>) -> LabeledSample {
        LabeledSample {
            label: label.to_string(),
            source: format!("{label}.txt"),
            features: FeatureVector {
                names: vec!["f1".into(), "f2".into()],
                values,
            },
            word_count: 500,
        }
    }

    #[test]
    fn test_train_predict_knn() {
        let train_data = vec![
            make_sample("alice", vec![0.0, 0.0]),
            make_sample("alice", vec![0.1, 0.1]),
            make_sample("bob", vec![1.0, 1.0]),
            make_sample("bob", vec![1.1, 1.1]),
        ];

        let config = PipelineConfig {
            model_type: ModelType::KNearestNeighbors,
            knn_k: 2,
            normalize: false,
            min_word_count: 0,
            ..Default::default()
        };

        let pipeline = train(&train_data, config).unwrap();
        assert_eq!(pipeline.class_labels, vec!["alice", "bob"]);
        assert_eq!(pipeline.n_train_samples, 4);

        let test_features = FeatureVector {
            names: vec!["f1".into(), "f2".into()],
            values: vec![0.05, 0.05],
        };
        let pred = pipeline.predict(&test_features).unwrap();
        assert_eq!(pred.label, "alice");
    }

    #[test]
    fn test_train_insufficient_samples() {
        let train_data = vec![make_sample("alice", vec![0.0, 0.0])];
        let config = PipelineConfig::default();
        let result = train(&train_data, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_train_single_class() {
        let train_data = vec![
            make_sample("alice", vec![0.0, 0.0]),
            make_sample("alice", vec![0.1, 0.1]),
        ];
        let config = PipelineConfig {
            min_word_count: 0,
            ..Default::default()
        };
        let result = train(&train_data, config);
        assert!(result.is_err()); // Need 2+ classes
    }
}
