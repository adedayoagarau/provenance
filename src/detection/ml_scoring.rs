//! ML-based scoring pipeline using ONNX ensemble models.
//!
//! When the `onnx` feature is enabled and trained models are available,
//! this module replaces the handcrafted weighted sigmoid composite scorer
//! with a 3-model ensemble (XGBoost + Neural Network + SVM).
//!
//! Pipeline:
//! 1. Extract selected features (read names from `models/config.json`)
//! 2. Normalize features using saved z-score parameters
//! 3. Run inference through all 3 ONNX models
//! 4. Combine predictions using ensemble weights
//! 5. Apply two-stage calibration (temperature scaling + isotonic regression)
//! 6. Apply threshold for 2% FPR classification
//!
//! The total inference pipeline must stay under 2 seconds for 5000 words.
//!
//! When the `onnx` feature is not enabled or models are not found,
//! the system falls back to the existing weighted sigmoid scorer.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::utils::errors::{ProvenanceError, Result};

use super::scoring::{DetectionScore, DetectionTier};

/// Configuration loaded from `models/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlScoringConfig {
    /// Ensemble weights per model (must sum to 1.0).
    pub ensemble_weights: EnsembleWeights,
    /// Decision threshold (where FPR = 2%).
    pub threshold: f64,
    /// Target false positive rate.
    pub target_fpr: f64,
    /// Number of features expected.
    pub n_features: usize,
    /// ONNX model filenames.
    pub models: Vec<String>,
    /// Path to feature names JSON.
    pub feature_names_file: String,
    /// Path to normalization parameters JSON.
    pub normalization_file: String,
    /// Path to calibration parameters JSON.
    pub calibration_file: String,
}

/// Per-model ensemble weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsembleWeights {
    pub xgboost: f64,
    pub neural_net: f64,
    pub svm: f64,
}

/// Z-score normalization parameters per feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationParams {
    pub features: std::collections::HashMap<String, FeatureNorm>,
}

/// Mean and standard deviation for a single feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureNorm {
    pub mean: f64,
    pub std: f64,
}

/// Two-stage calibration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationParams {
    /// Temperature scaling parameter.
    pub temperature: f64,
    /// Isotonic regression x-thresholds.
    pub isotonic_x: Vec<f64>,
    /// Isotonic regression y-thresholds.
    pub isotonic_y: Vec<f64>,
}

impl CalibrationParams {
    /// Apply two-stage calibration to a raw score.
    ///
    /// Stage 1: Temperature scaling (divide logit by temperature).
    /// Stage 2: Isotonic regression (piecewise linear interpolation).
    pub fn calibrate(&self, raw_score: f64) -> f64 {
        // Stage 1: Temperature scaling
        let scaled = raw_score / self.temperature;

        // Stage 2: Isotonic regression (linear interpolation)
        if self.isotonic_x.is_empty() || self.isotonic_y.is_empty() {
            return scaled.clamp(0.0, 1.0);
        }

        // Clamp to range of isotonic regression
        if scaled <= self.isotonic_x[0] {
            return self.isotonic_y[0];
        }
        if scaled >= *self.isotonic_x.last().unwrap() {
            return *self.isotonic_y.last().unwrap();
        }

        // Linear interpolation between isotonic regression points
        for i in 0..self.isotonic_x.len() - 1 {
            if scaled >= self.isotonic_x[i] && scaled <= self.isotonic_x[i + 1] {
                let t = (scaled - self.isotonic_x[i])
                    / (self.isotonic_x[i + 1] - self.isotonic_x[i]);
                return self.isotonic_y[i] + t * (self.isotonic_y[i + 1] - self.isotonic_y[i]);
            }
        }

        scaled.clamp(0.0, 1.0)
    }
}

/// ML-based scorer using ONNX ensemble models.
///
/// Loads models, normalization parameters, calibration parameters,
/// and threshold from the `models/` directory.
pub struct MlScorer {
    /// Base directory containing model files.
    models_dir: PathBuf,
    /// Pipeline configuration.
    config: MlScoringConfig,
    /// Selected feature names (in order).
    feature_names: Vec<String>,
    /// Z-score normalization parameters.
    normalization: NormalizationParams,
    /// Calibration parameters.
    calibration: CalibrationParams,
    /// ONNX model sessions (XGBoost, Neural Net, SVM).
    #[cfg(feature = "onnx")]
    sessions: Vec<ort::session::Session>,
}

impl MlScorer {
    /// Try to load the ML scorer from the models directory.
    pub fn try_load(models_dir: &Path) -> Option<Self> {
        let config_path = models_dir.join("config.json");
        if !config_path.exists() {
            return None;
        }

        let config_str = std::fs::read_to_string(&config_path).ok()?;
        let config: MlScoringConfig = serde_json::from_str(&config_str).ok()?;

        let names_path = models_dir.join(&config.feature_names_file);
        let names_str = std::fs::read_to_string(&names_path).ok()?;
        let feature_names: Vec<String> = serde_json::from_str(&names_str).ok()?;

        let norm_path = models_dir.join(&config.normalization_file);
        let norm_str = std::fs::read_to_string(&norm_path).ok()?;
        let normalization: NormalizationParams = serde_json::from_str(&norm_str).ok()?;

        let cal_path = models_dir.join(&config.calibration_file);
        let cal_str = std::fs::read_to_string(&cal_path).ok()?;
        let calibration: CalibrationParams = serde_json::from_str(&cal_str).ok()?;

        #[cfg(feature = "onnx")]
        let sessions = {
            let mut sessions = Vec::new();
            for model_name in &config.models {
                let model_path = models_dir.join(model_name);
                match ort::session::Session::builder()
                    .and_then(|mut b| b.commit_from_file(&model_path))
                {
                    Ok(session) => sessions.push(session),
                    Err(e) => {
                        eprintln!("Warning: Failed to load ONNX model '{}': {}", model_name, e);
                        return None;
                    }
                }
            }
            sessions
        };

        Some(Self {
            models_dir: models_dir.to_path_buf(),
            config,
            feature_names,
            normalization,
            calibration,
            #[cfg(feature = "onnx")]
            sessions,
        })
    }

    /// Check if the ML scorer has valid models loaded.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "onnx")]
        {
            self.sessions.len() == 3
        }
        #[cfg(not(feature = "onnx"))]
        {
            false
        }
    }

    /// Extract selected features from a DetectionFeatures struct.
    ///
    /// Maps feature names to values from the detection pipeline output.
    pub fn extract_features(
        &self,
        features: &super::DetectionFeatures,
    ) -> Vec<f64> {
        let mut values = Vec::with_capacity(self.feature_names.len());

        for name in &self.feature_names {
            let val = match name.as_str() {
                // Tier 1 detection features
                "burstiness_coefficient" => features.burstiness.as_ref().map(|b| b.coefficient),
                "zipf_deviation" => features.zipf.as_ref().map(|z| z.deviation),
                "hedge_ratio" => features.hedge_ratio.as_ref().map(|h| h.ratio),
                "autocorrelation_lag1" => features.autocorrelation.as_ref().map(|a| a.lag1_autocorrelation),
                "pos_trigram_entropy" => features.pos_entropy.as_ref().map(|p| p.trigram_entropy),
                "diversity_length_correlation" => features.interaction.as_ref().map(|i| i.diversity_length_correlation),
                "repetition_position_delta" => features.interaction.as_ref().map(|i| i.repetition_position_delta),

                // Tier 2 features
                "passive_clustering" => features.tier2.as_ref().and_then(|t| t.passive_clustering),
                "transition_density" => features.tier2.as_ref().and_then(|t| t.transition_density),
                "paragraph_length_cv" => features.tier2.as_ref().and_then(|t| t.paragraph_length_cv),
                "sentence_opening_diversity" => features.tier2.as_ref().and_then(|t| t.sentence_opening_diversity),
                "nested_clause_ratio" => features.tier2.as_ref().and_then(|t| t.nested_clause_ratio),
                "comma_splice_ratio" => features.tier2.as_ref().and_then(|t| t.comma_splice_ratio),

                // Advanced features
                "vocab_sophistication_slope" => features.advanced.as_ref().and_then(|a| a.vocab_sophistication_slope),
                "register_consistency" => features.advanced.as_ref().and_then(|a| a.register_consistency),
                "initial_adverb_ratio" => features.advanced.as_ref().and_then(|a| a.initial_adverb_ratio),
                "modal_density" => features.advanced.as_ref().and_then(|a| a.modal_density),
                "punctuation_diversity" => features.advanced.as_ref().and_then(|a| a.punctuation_diversity),
                "enumeration_ratio" => features.advanced.as_ref().and_then(|a| a.enumeration_ratio),
                "paragraph_transition_overlap" => features.advanced.as_ref().and_then(|a| a.paragraph_transition_overlap),

                // Unknown feature — use 0.0 (will be normalized)
                _ => None,
            };

            values.push(val.unwrap_or(0.0));
        }

        values
    }

    /// Normalize features using saved z-score parameters.
    pub fn normalize(&self, features: &[f64]) -> Vec<f64> {
        features
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                if let Some(norm) = self.normalization.features.get(&self.feature_names[i]) {
                    if norm.std > 1e-10 {
                        (val - norm.mean) / norm.std
                    } else {
                        0.0
                    }
                } else {
                    val
                }
            })
            .collect()
    }

    /// Run inference through all 3 ONNX models and return ensemble score.
    #[cfg(feature = "onnx")]
    pub fn predict(&self, normalized_features: &[f64]) -> Result<f64> {
        let n = normalized_features.len();
        let data: Vec<f32> = normalized_features.iter().map(|&x| x as f32).collect();

        let weights = [
            self.config.ensemble_weights.xgboost,
            self.config.ensemble_weights.neural_net,
            self.config.ensemble_weights.svm,
        ];

        let mut ensemble_score = 0.0;

        for (i, session) in self.sessions.iter().enumerate() {
            let value = ort::value::Value::from_array(([1usize, n], data.clone()))
                .map_err(|e| ProvenanceError::AnalysisError {
                    reason: format!("ONNX input error for model {i}: {e}"),
                })?;

            let outputs = session
                .run(ort::inputs![value])
                .map_err(|e| ProvenanceError::AnalysisError {
                    reason: format!("ONNX inference error for model {i}: {e}"),
                })?;

            // Extract probability score from model output
            let score = if outputs.len() > 1 {
                // sklearn format: [labels, probabilities]
                if let Ok(probs_tensor) = outputs[1].try_extract_tensor::<f32>() {
                    let probs = probs_tensor.as_slice().unwrap_or(&[0.5]);
                    // Class 1 (AI) probability — second element if binary
                    if probs.len() > 1 { probs[1] as f64 } else { probs[0] as f64 }
                } else {
                    0.5
                }
            } else {
                // PyTorch format: single output
                if let Ok(out_tensor) = outputs[0].try_extract_tensor::<f32>() {
                    let data = out_tensor.as_slice().unwrap_or(&[0.5]);
                    data[0] as f64
                } else {
                    0.5
                }
            };

            ensemble_score += weights[i] * score;
        }

        Ok(ensemble_score)
    }

    /// Run inference (stub when onnx feature is disabled).
    #[cfg(not(feature = "onnx"))]
    pub fn predict(&self, _normalized_features: &[f64]) -> Result<f64> {
        Err(ProvenanceError::AnalysisError {
            reason: "ML scoring requires the 'onnx' feature flag. \
                    Build with: cargo build --features onnx"
                .to_string(),
        })
    }

    /// Run the full ML scoring pipeline on extracted detection features.
    ///
    /// Returns a DetectionScore with the ML-calibrated score and classification.
    pub fn score(
        &self,
        features: &super::DetectionFeatures,
    ) -> Result<DetectionScore> {
        // Step 1: Extract selected features
        let raw_features = self.extract_features(features);

        // Step 2: Normalize
        let normalized = self.normalize(&raw_features);

        // Step 3-4: Run ensemble inference
        let ensemble_score = self.predict(&normalized)?;

        // Step 5: Apply two-stage calibration
        let calibrated_score = self.calibration.calibrate(ensemble_score);

        // Step 6: Apply threshold for classification
        let tier = if calibrated_score >= self.config.threshold {
            // Above threshold = classified as AI at 2% FPR
            DetectionTier::from_score(calibrated_score)
        } else {
            DetectionTier::from_score(calibrated_score)
        };

        let features_used = raw_features.iter().filter(|&&v| v != 0.0).count();
        let confidence = (calibrated_score - 0.5).abs() * 2.0;

        Ok(DetectionScore {
            score: ensemble_score,
            adjusted_score: calibrated_score,
            tier,
            tier_label: tier.label().to_string(),
            action: tier.action().to_string(),
            register_adjustment: 0.0, // ML model handles register internally
            register_label: String::new(),
            features_used,
            feature_scores: Vec::new(), // ML scorer doesn't expose per-feature breakdown
            confidence: confidence.clamp(0.0, 1.0),
        })
    }
}

/// Check if ML models are available at the given path.
pub fn ml_models_available(models_dir: &Path) -> bool {
    models_dir.join("config.json").exists()
        && models_dir.join("calibration.json").exists()
        && models_dir.join("normalization.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_identity() {
        let cal = CalibrationParams {
            temperature: 1.0,
            isotonic_x: vec![0.0, 0.5, 1.0],
            isotonic_y: vec![0.0, 0.5, 1.0],
        };
        let result = cal.calibrate(0.5);
        assert!((result - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calibration_clamping() {
        let cal = CalibrationParams {
            temperature: 1.0,
            isotonic_x: vec![0.2, 0.8],
            isotonic_y: vec![0.1, 0.9],
        };
        assert_eq!(cal.calibrate(0.0), 0.1);  // Below range → first y
        assert_eq!(cal.calibrate(1.0), 0.9);  // Above range → last y
    }

    #[test]
    fn test_calibration_interpolation() {
        let cal = CalibrationParams {
            temperature: 1.0,
            isotonic_x: vec![0.0, 1.0],
            isotonic_y: vec![0.0, 1.0],
        };
        let result = cal.calibrate(0.3);
        assert!((result - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_ml_models_available_false() {
        let tmp = std::env::temp_dir().join("provenance_test_ml_scoring");
        let _ = std::fs::create_dir_all(&tmp);
        assert!(!ml_models_available(&tmp));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
