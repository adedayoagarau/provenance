//! ONNX model inference for authorship attribution.
//!
//! Loads models trained by Python scripts (scikit-learn → ONNX) and runs
//! inference in Rust via the `ort` crate. Requires the `onnx` feature flag.
//!
//! # Usage
//!
//! ```ignore
//! // With the `onnx` feature enabled:
//! let registry = ModelRegistry::new("models/");
//! let model = registry.load("svm_v1.onnx")?;
//! let prediction = model.predict(&feature_vector)?;
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::identity::features::FeatureVector;
use crate::utils::errors::{ProvenanceError, Result};

/// Metadata for a registered ONNX model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model file name
    pub name: String,
    /// Model version string
    pub version: String,
    /// Model type (e.g., "svm", "gradient_boosting")
    pub model_type: String,
    /// Feature set used during training
    pub feature_set: String,
    /// Number of input features expected
    pub n_features: usize,
    /// Class labels in order
    pub class_labels: Vec<String>,
    /// Training accuracy (if known)
    pub training_accuracy: Option<f64>,
    /// Path to the ONNX file
    pub path: PathBuf,
}

/// Registry for managing versioned ONNX model files.
pub struct ModelRegistry {
    /// Base directory containing model files
    base_dir: PathBuf,
    /// Registered models (loaded from manifest)
    models: Vec<ModelInfo>,
}

impl ModelRegistry {
    /// Create a new registry scanning the given directory.
    pub fn new(base_dir: &Path) -> Result<Self> {
        let mut registry = Self {
            base_dir: base_dir.to_path_buf(),
            models: Vec::new(),
        };
        registry.scan()?;
        Ok(registry)
    }

    /// Scan the base directory for model manifest files.
    fn scan(&mut self) -> Result<()> {
        let manifest_path = self.base_dir.join("manifest.json");
        if manifest_path.exists() {
            let content = crate::utils::errors::read_file_string(&manifest_path)?;
            self.models = serde_json::from_str(&content).map_err(|e| {
                ProvenanceError::AnalysisError {
                    reason: format!("Failed to parse model manifest: {e}"),
                }
            })?;
        }
        Ok(())
    }

    /// List all registered models.
    pub fn list(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Find a model by name.
    pub fn find(&self, name: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Find the latest model of a given type.
    pub fn latest(&self, model_type: &str) -> Option<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.model_type == model_type)
            .next_back()
    }

    /// Register a new model.
    pub fn register(&mut self, info: ModelInfo) {
        self.models.push(info);
    }

    /// Save the manifest to disk.
    pub fn save_manifest(&self) -> Result<()> {
        let manifest_path = self.base_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(&self.models).map_err(|e| {
            ProvenanceError::AnalysisError {
                reason: format!("Failed to serialize manifest: {e}"),
            }
        })?;
        std::fs::write(&manifest_path, json).map_err(|e| ProvenanceError::IoWithPath {
            path: manifest_path.display().to_string(),
            source: e,
        })
    }
}

/// ONNX model wrapper for inference.
///
/// Requires the `onnx` feature flag to actually load and run models.
/// Without it, this module provides the types and registry but inference
/// methods return errors.
pub struct OnnxModel {
    info: ModelInfo,
    #[cfg(feature = "onnx")]
    session: ort::session::Session,
}

impl OnnxModel {
    /// Load an ONNX model from disk.
    #[cfg(feature = "onnx")]
    pub fn load(info: &ModelInfo) -> Result<Self> {
        let session = ort::session::Session::builder()
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("Failed to create session builder: {e}"),
            })?
            .commit_from_file(&info.path)
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("Failed to load ONNX model '{}': {e}", info.name),
            })?;

        Ok(Self {
            info: info.clone(),
            session,
        })
    }

    /// Load an ONNX model (stub when onnx feature is disabled).
    #[cfg(not(feature = "onnx"))]
    pub fn load(info: &ModelInfo) -> Result<Self> {
        Ok(Self {
            info: info.clone(),
        })
    }

    /// Run inference on a single feature vector.
    #[cfg(feature = "onnx")]
    pub fn predict(
        &self,
        features: &FeatureVector,
    ) -> Result<super::models::Prediction> {
        use ndarray::Array2;

        let input = Array2::from_shape_vec(
            (1, features.len()),
            features.values.iter().map(|&x| x as f32).collect(),
        )
        .map_err(|e| ProvenanceError::AnalysisError {
            reason: format!("Feature shape error: {e}"),
        })?;

        let input_value = ort::value::Value::from_array(input.view())
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("ONNX input error: {e}"),
            })?;

        let outputs = self.session.run(ort::inputs![input_value])
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("ONNX inference error: {e}"),
            })?;

        // Parse output (assumes sklearn ONNX format: label + probabilities)
        let label_idx = outputs[0]
            .try_extract_tensor::<i64>()
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("ONNX output parse error: {e}"),
            })?;

        let idx = label_idx.view()[[0]] as usize;
        let label = self
            .info
            .class_labels
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("class_{idx}"));

        // Try to extract probabilities
        let class_probabilities = if outputs.len() > 1 {
            if let Ok(probs) = outputs[1].try_extract_tensor::<f32>() {
                let probs_view: ndarray::ArrayViewD<'_, f32> = probs.view();
                self.info
                    .class_labels
                    .iter()
                    .enumerate()
                    .map(|(i, l)| (l.clone(), probs_view[[0, i]] as f64))
                    .collect()
            } else {
                vec![(label.clone(), 1.0)]
            }
        } else {
            vec![(label.clone(), 1.0)]
        };

        let confidence = class_probabilities
            .iter()
            .find(|(l, _)| l == &label)
            .map(|(_, p)| *p)
            .unwrap_or(1.0);

        Ok(super::models::Prediction {
            label,
            confidence,
            class_probabilities,
        })
    }

    /// Run inference (stub when onnx feature is disabled).
    #[cfg(not(feature = "onnx"))]
    pub fn predict(
        &self,
        _features: &FeatureVector,
    ) -> Result<super::models::Prediction> {
        Err(ProvenanceError::AnalysisError {
            reason: "ONNX inference requires the 'onnx' feature flag. \
                    Build with: cargo build --features onnx"
                .to_string(),
        })
    }

    /// Run batch inference on multiple feature vectors.
    pub fn predict_batch(
        &self,
        features: &[FeatureVector],
    ) -> Result<Vec<super::models::Prediction>> {
        features.iter().map(|f| self.predict(f)).collect()
    }

    /// Get model info.
    pub fn info(&self) -> &ModelInfo {
        &self.info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_model_registry_empty() {
        let dir = std::env::temp_dir().join("provenance_test_registry");
        let _ = fs::create_dir_all(&dir);

        let registry = ModelRegistry::new(&dir).unwrap();
        assert!(registry.list().is_empty());
        assert!(registry.find("nonexistent").is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_model_registry_manifest() {
        let dir = std::env::temp_dir().join("provenance_test_manifest");
        let _ = fs::create_dir_all(&dir);

        // Create a manifest
        let models = vec![ModelInfo {
            name: "svm_v1".into(),
            version: "1.0.0".into(),
            model_type: "svm".into(),
            feature_set: "standard".into(),
            n_features: 240,
            class_labels: vec!["alice".into(), "bob".into()],
            training_accuracy: Some(0.95),
            path: dir.join("svm_v1.onnx"),
        }];
        let json = serde_json::to_string_pretty(&models).unwrap();
        fs::write(dir.join("manifest.json"), json).unwrap();

        let registry = ModelRegistry::new(&dir).unwrap();
        assert_eq!(registry.list().len(), 1);
        assert!(registry.find("svm_v1").is_some());
        assert_eq!(registry.latest("svm").unwrap().name, "svm_v1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_onnx_model_stub() {
        let info = ModelInfo {
            name: "test".into(),
            version: "0.1".into(),
            model_type: "svm".into(),
            feature_set: "standard".into(),
            n_features: 2,
            class_labels: vec!["a".into(), "b".into()],
            training_accuracy: None,
            path: PathBuf::from("nonexistent.onnx"),
        };

        #[cfg(not(feature = "onnx"))]
        {
            let model = OnnxModel::load(&info).unwrap();
            assert_eq!(model.info().name, "test");

            let fv = FeatureVector {
                names: vec!["f1".into(), "f2".into()],
                values: vec![1.0, 2.0],
            };
            assert!(model.predict(&fv).is_err());
        }
    }
}
