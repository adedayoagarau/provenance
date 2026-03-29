//! ONNX model inference for authorship attribution.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::identity::features::FeatureVector;
use crate::utils::errors::{ProvenanceError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub model_type: String,
    pub feature_set: String,
    pub n_features: usize,
    pub class_labels: Vec<String>,
    pub training_accuracy: Option<f64>,
    pub path: PathBuf,
}

pub struct ModelRegistry {
    base_dir: PathBuf,
    models: Vec<ModelInfo>,
}

impl ModelRegistry {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let mut registry = Self { base_dir: base_dir.to_path_buf(), models: Vec::new() };
        registry.scan()?;
        Ok(registry)
    }

    fn scan(&mut self) -> Result<()> {
        let manifest_path = self.base_dir.join("manifest.json");
        if manifest_path.exists() {
            let content = crate::utils::errors::read_file_string(&manifest_path)?;
            self.models = serde_json::from_str(&content).map_err(|e| {
                ProvenanceError::AnalysisError { reason: format!("Failed to parse model manifest: {e}") }
            })?;
        }
        Ok(())
    }

    pub fn list(&self) -> &[ModelInfo] { &self.models }
    pub fn find(&self, name: &str) -> Option<&ModelInfo> { self.models.iter().find(|m| m.name == name) }
    pub fn latest(&self, model_type: &str) -> Option<&ModelInfo> {
        self.models.iter().filter(|m| m.model_type == model_type).next_back()
    }
    pub fn register(&mut self, info: ModelInfo) { self.models.push(info); }

    pub fn save_manifest(&self) -> Result<()> {
        let manifest_path = self.base_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(&self.models).map_err(|e| {
            ProvenanceError::AnalysisError { reason: format!("Failed to serialize manifest: {e}") }
        })?;
        std::fs::write(&manifest_path, json).map_err(|e| ProvenanceError::IoWithPath {
            path: manifest_path.display().to_string(), source: e,
        })
    }
}

pub struct OnnxModel {
    info: ModelInfo,
    #[cfg(feature = "onnx")]
    session: ort::session::Session,
}

impl OnnxModel {
    #[cfg(feature = "onnx")]
    pub fn load(info: &ModelInfo) -> Result<Self> {
        let session = ort::session::Session::builder()
            .and_then(|mut b| b.commit_from_file(&info.path))
            .map_err(|e| ProvenanceError::AnalysisError {
                reason: format!("Failed to load ONNX model '{}': {e}", info.name),
            })?;
        Ok(Self { info: info.clone(), session })
    }

    #[cfg(not(feature = "onnx"))]
    pub fn load(info: &ModelInfo) -> Result<Self> {
        Ok(Self { info: info.clone() })
    }

    #[cfg(feature = "onnx")]
    pub fn predict(&mut self, features: &FeatureVector) -> Result<super::models::Prediction> {
        let n = features.len();
        let data: Vec<f32> = features.values.iter().map(|&x| x as f32).collect();

        let value = ort::value::Value::from_array(([1usize, n], data))
            .map_err(|e| ProvenanceError::AnalysisError { reason: format!("ONNX input error: {e}") })?;

        let outputs = self.session.run(ort::inputs![value])
            .map_err(|e| ProvenanceError::AnalysisError { reason: format!("ONNX inference error: {e}") })?;

        // Parse sklearn ONNX output: [labels, probabilities]
        let label_tensor = outputs[0].try_extract_tensor::<i64>()
            .map_err(|e| ProvenanceError::AnalysisError { reason: format!("Output parse error: {e}") })?;
        let idx = label_tensor.1[0] as usize;
        let label = self.info.class_labels.get(idx).cloned().unwrap_or_else(|| format!("class_{idx}"));

        let class_probabilities = if outputs.len() > 1 {
            if let Ok(probs_tensor) = outputs[1].try_extract_tensor::<f32>() {
                let probs_slice = probs_tensor.1;
                let _n_classes = self.info.class_labels.len();
                self.info.class_labels.iter().enumerate()
                    .map(|(i, l)| (l.clone(), if i < probs_slice.len() { probs_slice[i] as f64 } else { 0.0 }))
                    .collect()
            } else {
                vec![(label.clone(), 1.0)]
            }
        } else {
            vec![(label.clone(), 1.0)]
        };

        let confidence = class_probabilities.iter().find(|(l, _)| l == &label).map(|(_, p)| *p).unwrap_or(1.0);
        Ok(super::models::Prediction { label, confidence, class_probabilities })
    }

    #[cfg(not(feature = "onnx"))]
    pub fn predict(&self, _features: &FeatureVector) -> Result<super::models::Prediction> {
        Err(ProvenanceError::AnalysisError {
            reason: "ONNX inference requires the 'onnx' feature flag.".to_string(),
        })
    }

    pub fn predict_batch(&mut self, features: &[FeatureVector]) -> Result<Vec<super::models::Prediction>> {
        features.iter().map(|f| self.predict(f)).collect()
    }

    pub fn info(&self) -> &ModelInfo { &self.info }
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
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_model_registry_manifest() {
        let dir = std::env::temp_dir().join("provenance_test_manifest");
        let _ = fs::create_dir_all(&dir);
        let models = vec![ModelInfo {
            name: "svm_v1".into(), version: "1.0.0".into(), model_type: "svm".into(),
            feature_set: "standard".into(), n_features: 240,
            class_labels: vec!["alice".into(), "bob".into()],
            training_accuracy: Some(0.95), path: dir.join("svm_v1.onnx"),
        }];
        fs::write(dir.join("manifest.json"), serde_json::to_string_pretty(&models).unwrap()).unwrap();
        let registry = ModelRegistry::new(&dir).unwrap();
        assert_eq!(registry.list().len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
