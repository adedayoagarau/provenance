//! Machine learning pipeline for authorship attribution.
//!
//! Provides classical ML models (SVM, Random Forest, Logistic Regression),
//! a feature engineering pipeline with caching, evaluation with cross-validation,
//! and optional ONNX model inference.

pub mod features;
pub mod models;
pub mod pipeline;
pub mod evaluation;
pub mod inference;
