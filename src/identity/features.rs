use ndarray::Array1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::analysis::AnalysisResult;

/// A named feature vector extracted from text analysis.
///
/// This is the unified representation used for all distance computations
/// including Burrows' Delta. Features are ordered and named so that
/// vectors from different texts are directly comparable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    /// Feature names in order
    pub names: Vec<String>,
    /// Feature values (same length as names)
    pub values: Vec<f64>,
}

/// Feature set configurations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeatureSet {
    /// Function words only (~200 features). Fast baseline.
    Minimal,
    /// Function words + scalar metrics (~240 features). Good balance.
    Standard,
    /// Function words + scalar metrics + top character n-grams (~740 features).
    Comprehensive,
}

impl FeatureVector {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Convert to ndarray for numerical operations.
    pub fn to_array(&self) -> Array1<f64> {
        Array1::from_vec(self.values.clone())
    }

    /// Get a feature value by name.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.names.iter().position(|n| n == name).map(|i| self.values[i])
    }
}

/// Extract a feature vector from an analysis result.
///
/// The feature set determines which features are included. All vectors
/// produced with the same feature set have identical dimensionality and
/// feature ordering, making them directly comparable.
pub fn extract(analysis: &AnalysisResult, set: FeatureSet) -> FeatureVector {
    let mut names = Vec::new();
    let mut values = Vec::new();

    // Always include function word frequencies (sorted by word for consistency)
    let mut fw_words: Vec<(&String, &f64)> = analysis.function_words.frequencies.iter().collect();
    fw_words.sort_by_key(|(word, _)| word.to_string());
    for (word, &freq) in &fw_words {
        names.push(format!("fw:{word}"));
        values.push(freq);
    }

    if set == FeatureSet::Minimal {
        return FeatureVector { names, values };
    }

    // Standard: add scalar metrics
    // Lexical
    names.push("lex:mattr".to_string()); values.push(analysis.lexical.mattr);
    names.push("lex:yules_k".to_string()); values.push(analysis.lexical.yules_k);
    names.push("lex:honores_r".to_string()); values.push(analysis.lexical.honores_r);
    names.push("lex:brunets_w".to_string()); values.push(analysis.lexical.brunets_w);
    names.push("lex:hapax_ratio".to_string()); values.push(analysis.lexical.hapax_ratio);
    names.push("lex:hapax_dis_ratio".to_string()); values.push(analysis.lexical.hapax_dis_ratio);
    names.push("lex:avg_word_length".to_string()); values.push(analysis.lexical.avg_word_length);

    // Syntactic
    names.push("syn:avg_words_per_sentence".to_string()); values.push(analysis.syntactic.avg_words_per_sentence);
    names.push("syn:sentence_length_variance".to_string()); values.push(analysis.syntactic.sentence_length_variance);
    names.push("syn:interrogative_ratio".to_string()); values.push(analysis.syntactic.interrogative_ratio);
    names.push("syn:exclamatory_ratio".to_string()); values.push(analysis.syntactic.exclamatory_ratio);
    names.push("syn:passive_voice_ratio".to_string()); values.push(analysis.syntactic.passive_voice_ratio);

    // Stylometric
    names.push("sty:comma_ratio".to_string()); values.push(analysis.stylometric.comma_ratio);
    names.push("sty:semicolon_ratio".to_string()); values.push(analysis.stylometric.semicolon_ratio);
    names.push("sty:colon_ratio".to_string()); values.push(analysis.stylometric.colon_ratio);
    names.push("sty:exclamation_ratio".to_string()); values.push(analysis.stylometric.exclamation_ratio);
    names.push("sty:question_ratio".to_string()); values.push(analysis.stylometric.question_ratio);
    names.push("sty:contraction_ratio".to_string()); values.push(analysis.stylometric.contraction_ratio);
    names.push("sty:hedge_word_ratio".to_string()); values.push(analysis.stylometric.hedge_word_ratio);
    names.push("sty:intensifier_ratio".to_string()); values.push(analysis.stylometric.intensifier_ratio);
    names.push("sty:short_paragraph_ratio".to_string()); values.push(analysis.stylometric.short_paragraph_ratio);

    // Discourse & readability
    names.push("sem:discourse_marker_ratio".to_string()); values.push(analysis.semantic.discourse_marker_ratio);
    names.push("sem:flesch_kincaid_grade".to_string()); values.push(analysis.semantic.flesch_kincaid_grade);
    names.push("sem:gunning_fog_index".to_string()); values.push(analysis.semantic.gunning_fog_index);
    names.push("sem:avg_syllables_per_word".to_string()); values.push(analysis.semantic.avg_syllables_per_word);

    // Function word aggregate
    names.push("fw:ratio".to_string()); values.push(analysis.function_words.function_word_ratio);
    names.push("fw:diversity".to_string()); values.push(analysis.function_words.function_word_diversity as f64);

    if set == FeatureSet::Standard {
        return FeatureVector { names, values };
    }

    // Comprehensive: add top character n-grams (sorted for consistency)
    add_sorted_ngrams(&mut names, &mut values, "c2", &analysis.ngrams.char_bigrams, 100);
    add_sorted_ngrams(&mut names, &mut values, "c3", &analysis.ngrams.char_trigrams, 100);
    add_sorted_ngrams(&mut names, &mut values, "c4", &analysis.ngrams.char_fourgrams, 100);
    add_sorted_ngrams(&mut names, &mut values, "c5", &analysis.ngrams.char_fivegrams, 100);
    add_sorted_ngrams(&mut names, &mut values, "wb", &analysis.ngrams.word_bigrams, 100);

    FeatureVector { names, values }
}

/// Add top-N n-grams from a frequency map, sorted by key for consistency.
fn add_sorted_ngrams(
    names: &mut Vec<String>,
    values: &mut Vec<f64>,
    prefix: &str,
    freqs: &HashMap<String, f64>,
    top_n: usize,
) {
    let mut sorted: Vec<(&String, &f64)> = freqs.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(top_n);
    // Re-sort by key for deterministic ordering
    sorted.sort_by_key(|(key, _)| key.to_string());

    for (ngram, &freq) in &sorted {
        names.push(format!("{prefix}:{ngram}"));
        values.push(freq);
    }
}

/// Corpus statistics for z-score normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusStats {
    pub feature_names: Vec<String>,
    pub means: Vec<f64>,
    pub stds: Vec<f64>,
    pub sample_count: usize,
}

impl CorpusStats {
    /// Compute corpus statistics from a set of feature vectors.
    ///
    /// All vectors must have the same dimensionality and feature ordering.
    pub fn from_vectors(vectors: &[FeatureVector]) -> Option<Self> {
        if vectors.is_empty() {
            return None;
        }

        let dim = vectors[0].len();
        let n = vectors.len() as f64;
        let names = vectors[0].names.clone();

        // Compute means
        let mut means = vec![0.0; dim];
        for v in vectors {
            for (i, &val) in v.values.iter().enumerate() {
                means[i] += val;
            }
        }
        for m in &mut means {
            *m /= n;
        }

        // Compute standard deviations
        let mut stds = vec![0.0; dim];
        for v in vectors {
            for (i, &val) in v.values.iter().enumerate() {
                let diff = val - means[i];
                stds[i] += diff * diff;
            }
        }
        for s in &mut stds {
            *s = (*s / n).sqrt();
            // Avoid division by zero: set minimum std
            if *s < 1e-10 {
                *s = 1e-10;
            }
        }

        Some(CorpusStats {
            feature_names: names,
            means,
            stds,
            sample_count: vectors.len(),
        })
    }

    /// Z-score normalize a feature vector using these corpus statistics.
    pub fn normalize(&self, vector: &FeatureVector) -> Array1<f64> {
        let mut result = vec![0.0; self.means.len()];

        // Match features by name (handles different-length vectors)
        let name_to_idx: HashMap<&str, usize> = self.feature_names.iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();

        for (i, name) in vector.names.iter().enumerate() {
            if let Some(&idx) = name_to_idx.get(name.as_str()) {
                result[idx] = (vector.values[i] - self.means[idx]) / self.stds[idx];
            }
        }

        Array1::from_vec(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpus_stats() {
        let v1 = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![1.0, 4.0],
        };
        let v2 = FeatureVector {
            names: vec!["a".into(), "b".into()],
            values: vec![3.0, 6.0],
        };

        let stats = CorpusStats::from_vectors(&[v1.clone(), v2.clone()]).unwrap();
        assert_eq!(stats.means[0], 2.0);
        assert_eq!(stats.means[1], 5.0);

        // Normalize v1: a=(1-2)/1 = -1, b=(4-5)/1 = -1
        let norm = stats.normalize(&v1);
        assert!((norm[0] - (-1.0)).abs() < 0.01);
        assert!((norm[1] - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn test_feature_set_sizes() {
        let text = "The quick brown fox jumps over the lazy dog. The cat sat on the mat and the dog barked loudly at the passing cars.";
        let analysis = crate::analysis::analyze_text(text).unwrap();

        let minimal = extract(&analysis, FeatureSet::Minimal);
        let standard = extract(&analysis, FeatureSet::Standard);
        let comprehensive = extract(&analysis, FeatureSet::Comprehensive);

        // Minimal should be smallest (just function words)
        assert!(minimal.len() < standard.len());
        // Standard adds ~27 scalar metrics
        assert!(standard.len() > minimal.len() + 20);
        // Comprehensive adds n-grams
        assert!(comprehensive.len() > standard.len());
    }
}
