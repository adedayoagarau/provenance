//! Feature engineering pipeline for ML-based authorship attribution.
//!
//! Provides automated text → feature vector extraction with caching,
//! feature metadata, and batch processing utilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::analysis::{self, AnalysisResult};
use crate::extraction;
use crate::identity::features::{extract, FeatureSet, FeatureVector};
use crate::utils::errors::{ProvenanceError, Result};

/// Metadata describing a single feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMetadata {
    /// Feature name (e.g., "fw:the", "lex:mattr")
    pub name: String,
    /// Category (function_word, lexical, syntactic, stylometric, semantic, ngram)
    pub category: FeatureCategory,
    /// Relative computational cost (1 = cheap, 3 = expensive)
    pub cost: u8,
    /// Reliability score based on research literature (0.0–1.0)
    pub reliability: f64,
    /// Human-readable description
    pub description: String,
}

/// Feature categories matching the analysis modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureCategory {
    FunctionWord,
    Lexical,
    Syntactic,
    Stylometric,
    Semantic,
    Ngram,
}

impl std::fmt::Display for FeatureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FunctionWord => write!(f, "function_word"),
            Self::Lexical => write!(f, "lexical"),
            Self::Syntactic => write!(f, "syntactic"),
            Self::Stylometric => write!(f, "stylometric"),
            Self::Semantic => write!(f, "semantic"),
            Self::Ngram => write!(f, "ngram"),
        }
    }
}

/// A labeled feature vector for supervised learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledSample {
    /// Author label (class)
    pub label: String,
    /// Source file path
    pub source: String,
    /// Extracted feature vector
    pub features: FeatureVector,
    /// Word count of the source text
    pub word_count: usize,
}

/// Feature extraction pipeline with caching support.
pub struct FeaturePipeline {
    /// Which feature set to extract
    feature_set: FeatureSet,
    /// In-memory cache: file path → extracted features
    cache: HashMap<PathBuf, CachedExtraction>,
}

#[derive(Debug, Clone)]
struct CachedExtraction {
    features: FeatureVector,
    #[allow(dead_code)]
    analysis: AnalysisResult,
    word_count: usize,
}

impl FeaturePipeline {
    /// Create a new pipeline with the given feature set.
    pub fn new(feature_set: FeatureSet) -> Self {
        Self {
            feature_set,
            cache: HashMap::new(),
        }
    }

    /// Extract features from a single text string.
    pub fn extract_from_text(&self, text: &str) -> Result<(FeatureVector, usize)> {
        let analysis = analysis::analyze_text(text)?;
        let word_count = analysis.lexical.total_words;
        let features = extract(&analysis, self.feature_set);
        Ok((features, word_count))
    }

    /// Extract features from a file, using cache if available.
    pub fn extract_from_file(&mut self, path: &Path) -> Result<(FeatureVector, usize)> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if let Some(cached) = self.cache.get(&canonical) {
            return Ok((cached.features.clone(), cached.word_count));
        }

        let text = extraction::extract_text(path.to_str().unwrap_or(""))?;
        let analysis = analysis::analyze_text(&text)?;
        let word_count = analysis.lexical.total_words;
        let features = extract(&analysis, self.feature_set);

        self.cache.insert(
            canonical,
            CachedExtraction {
                features: features.clone(),
                analysis,
                word_count,
            },
        );

        Ok((features, word_count))
    }

    /// Extract labeled samples from a corpus directory.
    ///
    /// Expected structure: `corpus_dir/<author_name>/<text_files>`
    pub fn extract_corpus(&mut self, corpus_dir: &Path) -> Result<Vec<LabeledSample>> {
        let mut samples = Vec::new();

        let entries = std::fs::read_dir(corpus_dir).map_err(|e| ProvenanceError::IoWithPath {
            path: corpus_dir.display().to_string(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry?;
            let author_dir = entry.path();
            if !author_dir.is_dir() {
                continue;
            }

            let author_name = author_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let files =
                std::fs::read_dir(&author_dir).map_err(|e| ProvenanceError::IoWithPath {
                    path: author_dir.display().to_string(),
                    source: e,
                })?;

            for file_entry in files {
                let file_entry = file_entry?;
                let file_path = file_entry.path();
                if !file_path.is_file() {
                    continue;
                }

                match self.extract_from_file(&file_path) {
                    Ok((features, word_count)) => {
                        samples.push(LabeledSample {
                            label: author_name.clone(),
                            source: file_path.display().to_string(),
                            features,
                            word_count,
                        });
                    }
                    Err(_) => {
                        // Skip files that fail extraction (unsupported formats, etc.)
                        continue;
                    }
                }
            }
        }

        Ok(samples)
    }

    /// Export labeled samples to CSV for Python training scripts.
    pub fn export_csv(&self, samples: &[LabeledSample], output_path: &Path) -> Result<()> {
        if samples.is_empty() {
            return Err(ProvenanceError::AnalysisError {
                reason: "No samples to export".to_string(),
            });
        }

        let mut wtr = csv::Writer::from_path(output_path).map_err(|e| {
            ProvenanceError::IoWithPath {
                path: output_path.display().to_string(),
                source: std::io::Error::other(e.to_string()),
            }
        })?;

        // Header: label, source, word_count, feature1, feature2, ...
        let feature_names = &samples[0].features.names;
        let mut header = vec!["label".to_string(), "source".to_string(), "word_count".to_string()];
        header.extend(feature_names.iter().cloned());
        wtr.write_record(&header).map_err(csv_to_io_error)?;

        // Data rows
        for sample in samples {
            let mut row = vec![
                sample.label.clone(),
                sample.source.clone(),
                sample.word_count.to_string(),
            ];
            for &val in &sample.features.values {
                row.push(format!("{val:.8}"));
            }
            wtr.write_record(&row).map_err(csv_to_io_error)?;
        }

        wtr.flush().map_err(|e| ProvenanceError::IoWithPath {
            path: output_path.display().to_string(),
            source: std::io::Error::other(e.to_string()),
        })?;

        Ok(())
    }

    /// Get the feature set used by this pipeline.
    pub fn feature_set(&self) -> FeatureSet {
        self.feature_set
    }

    /// Clear the extraction cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Number of cached extractions.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

fn csv_to_io_error(e: csv::Error) -> ProvenanceError {
    ProvenanceError::AnalysisError {
        reason: format!("CSV write error: {e}"),
    }
}

/// Generate metadata for all features in a given feature set.
pub fn feature_metadata(set: FeatureSet) -> Vec<FeatureMetadata> {
    let mut meta = Vec::new();

    // Function words — always included, highest reliability per research
    meta.push(FeatureMetadata {
        name: "fw:*".to_string(),
        category: FeatureCategory::FunctionWord,
        cost: 1,
        reliability: 0.95,
        description: "Function word frequencies (200+ words), normalized per 1000 words. \
                      Most reliable authorship signal per Burrows (2002), Koppel et al. (2009)."
            .to_string(),
    });

    if set == FeatureSet::Minimal {
        return meta;
    }

    // Lexical metrics
    for (name, desc, reliability) in [
        ("lex:mattr", "Moving Average Type-Token Ratio (window=500)", 0.85),
        ("lex:yules_k", "Yule's K vocabulary richness", 0.80),
        ("lex:honores_r", "Honoré's R hapax-based richness", 0.75),
        ("lex:brunets_w", "Brunet's W vocabulary measure", 0.75),
        ("lex:hapax_ratio", "Hapax legomena ratio", 0.70),
        ("lex:hapax_dis_ratio", "Hapax/dis-legomena ratio", 0.65),
        ("lex:avg_word_length", "Average word length in characters", 0.80),
    ] {
        meta.push(FeatureMetadata {
            name: name.to_string(),
            category: FeatureCategory::Lexical,
            cost: 1,
            reliability,
            description: desc.to_string(),
        });
    }

    // Syntactic metrics
    for (name, desc, reliability) in [
        ("syn:avg_words_per_sentence", "Average sentence length", 0.80),
        ("syn:sentence_length_variance", "Sentence length variance", 0.70),
        ("syn:interrogative_ratio", "Interrogative sentence ratio", 0.60),
        ("syn:exclamatory_ratio", "Exclamatory sentence ratio", 0.55),
        ("syn:passive_voice_ratio", "Passive voice ratio", 0.65),
    ] {
        meta.push(FeatureMetadata {
            name: name.to_string(),
            category: FeatureCategory::Syntactic,
            cost: 1,
            reliability,
            description: desc.to_string(),
        });
    }

    // Stylometric metrics
    for (name, desc, reliability) in [
        ("sty:comma_ratio", "Comma usage ratio", 0.70),
        ("sty:semicolon_ratio", "Semicolon usage ratio", 0.75),
        ("sty:colon_ratio", "Colon usage ratio", 0.65),
        ("sty:exclamation_ratio", "Exclamation mark ratio", 0.60),
        ("sty:question_ratio", "Question mark ratio", 0.60),
        ("sty:contraction_ratio", "Contraction usage ratio", 0.80),
        ("sty:hedge_word_ratio", "Hedge word frequency", 0.65),
        ("sty:intensifier_ratio", "Intensifier frequency", 0.60),
        ("sty:short_paragraph_ratio", "Short paragraph ratio", 0.50),
    ] {
        meta.push(FeatureMetadata {
            name: name.to_string(),
            category: FeatureCategory::Stylometric,
            cost: 1,
            reliability,
            description: desc.to_string(),
        });
    }

    // Semantic/readability metrics
    for (name, desc, reliability) in [
        ("sem:discourse_marker_ratio", "Discourse marker frequency", 0.60),
        ("sem:flesch_kincaid_grade", "Flesch-Kincaid grade level", 0.65),
        ("sem:gunning_fog_index", "Gunning Fog readability index", 0.60),
        ("sem:avg_syllables_per_word", "Average syllables per word", 0.70),
    ] {
        meta.push(FeatureMetadata {
            name: name.to_string(),
            category: FeatureCategory::Semantic,
            cost: 1,
            reliability,
            description: desc.to_string(),
        });
    }

    if set == FeatureSet::Standard {
        return meta;
    }

    // N-gram features (comprehensive only)
    for (prefix, desc, reliability) in [
        ("c2:*", "Character bigram frequencies (top 100)", 0.90),
        ("c3:*", "Character trigram frequencies (top 100)", 0.92),
        ("c4:*", "Character 4-gram frequencies (top 100)", 0.88),
        ("c5:*", "Character 5-gram frequencies (top 100)", 0.85),
        ("wb:*", "Word bigram frequencies (top 100)", 0.80),
    ] {
        meta.push(FeatureMetadata {
            name: prefix.to_string(),
            category: FeatureCategory::Ngram,
            cost: 2,
            reliability,
            description: desc.to_string(),
        });
    }

    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_extract_text() {
        let pipeline = FeaturePipeline::new(FeatureSet::Standard);
        let text = "The quick brown fox jumps over the lazy dog. \
                    She was running through the forest and he was chasing after her. \
                    They eventually stopped near a river and rested for a while.";
        let (features, word_count) = pipeline.extract_from_text(text).unwrap();
        assert!(!features.is_empty());
        assert!(word_count > 0);
    }

    #[test]
    fn test_pipeline_caching() {
        let mut pipeline = FeaturePipeline::new(FeatureSet::Minimal);
        assert_eq!(pipeline.cache_size(), 0);
        // Cache operations are tested via extract_from_file with real files
        pipeline.clear_cache();
        assert_eq!(pipeline.cache_size(), 0);
    }

    #[test]
    fn test_feature_metadata() {
        let minimal = feature_metadata(FeatureSet::Minimal);
        let standard = feature_metadata(FeatureSet::Standard);
        let comprehensive = feature_metadata(FeatureSet::Comprehensive);

        assert_eq!(minimal.len(), 1); // Just fw:* wildcard
        assert!(standard.len() > minimal.len());
        assert!(comprehensive.len() > standard.len());

        // Check all have valid reliability scores
        for m in &comprehensive {
            assert!(m.reliability > 0.0 && m.reliability <= 1.0);
        }
    }
}
