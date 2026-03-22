use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Lexical analysis profile for a text.
///
/// Includes both traditional metrics (TTR, hapax) and length-independent
/// vocabulary richness measures validated by stylometry research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalProfile {
    pub total_words: usize,
    pub unique_words: usize,

    /// Simple Type-Token Ratio. WARNING: length-dependent — use MATTR instead
    /// for cross-document comparison.
    pub type_token_ratio: f64,

    pub hapax_legomena: usize,
    pub dis_legomena: usize,
    pub hapax_ratio: f64,
    pub avg_word_length: f64,

    /// Word length distribution: index = word length, value = count
    pub word_length_distribution: Vec<usize>,

    /// Moving Average Type-Token Ratio (window=500 words).
    /// Length-independent vocabulary richness measure.
    pub mattr: f64,

    /// Yule's K: vocabulary richness independent of text length.
    /// Based on frequency spectrum. Higher = less diverse vocabulary.
    pub yules_k: f64,

    /// Honoré's R: based on hapax legomena proportion.
    /// Higher = richer vocabulary. Length-partially-independent.
    pub honores_r: f64,

    /// Brunet's W: W = N^(V^-0.172). Lower = richer vocabulary.
    pub brunets_w: f64,

    /// Hapax/Dis-legomena ratio (words appearing once / words appearing twice).
    pub hapax_dis_ratio: f64,

    pub word_frequency: HashMap<String, usize>,
}

/// Perform lexical analysis on text.
pub fn analyze(text: &str) -> LexicalProfile {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    let total_words = words.len();

    let mut frequency: HashMap<String, usize> = HashMap::new();
    for word in &words {
        *frequency.entry(word.clone()).or_insert(0) += 1;
    }

    let unique_words = frequency.len();
    let n = total_words as f64;
    let v = unique_words as f64;

    let type_token_ratio = if total_words > 0 { v / n } else { 0.0 };

    let hapax_legomena = frequency.values().filter(|&&c| c == 1).count();
    let dis_legomena = frequency.values().filter(|&&c| c == 2).count();
    let hapax_ratio = if total_words > 0 { hapax_legomena as f64 / n } else { 0.0 };
    let hapax_dis_ratio = if dis_legomena > 0 {
        hapax_legomena as f64 / dis_legomena as f64
    } else {
        0.0
    };

    let total_chars: usize = words.iter().map(|w| w.len()).sum();
    let avg_word_length = if total_words > 0 { total_chars as f64 / n } else { 0.0 };

    // Word length distribution
    let max_len = words.iter().map(|w| w.len()).max().unwrap_or(0);
    let mut word_length_distribution = vec![0usize; max_len + 1];
    for word in &words {
        word_length_distribution[word.len()] += 1;
    }

    let mattr = compute_mattr(&words, 500);
    let yules_k = compute_yules_k(&frequency, total_words);
    let honores_r = compute_honores_r(total_words, unique_words, hapax_legomena);
    let brunets_w = compute_brunets_w(total_words, unique_words);

    LexicalProfile {
        total_words,
        unique_words,
        type_token_ratio,
        hapax_legomena,
        dis_legomena,
        hapax_ratio,
        avg_word_length,
        word_length_distribution,
        mattr,
        yules_k,
        honores_r,
        brunets_w,
        hapax_dis_ratio,
        word_frequency: frequency,
    }
}

/// Moving Average Type-Token Ratio.
///
/// Computes TTR over a sliding window of `window_size` words and averages.
/// This is length-independent because each window is the same size.
fn compute_mattr(words: &[String], window_size: usize) -> f64 {
    if words.len() < window_size || window_size == 0 {
        // Fall back to simple TTR for short texts
        if words.is_empty() { return 0.0; }
        let unique: std::collections::HashSet<&String> = words.iter().collect();
        return unique.len() as f64 / words.len() as f64;
    }

    let mut sum_ttr = 0.0;
    let num_windows = words.len() - window_size + 1;

    for i in 0..num_windows {
        let window = &words[i..i + window_size];
        let unique: std::collections::HashSet<&String> = window.iter().collect();
        sum_ttr += unique.len() as f64 / window_size as f64;
    }

    sum_ttr / num_windows as f64
}

/// Yule's K characteristic.
///
/// K = 10^4 * (sum(i^2 * V(i,N)) - N) / N^2
/// where V(i,N) = number of word types occurring exactly i times.
/// Higher K = less diverse (more repetitive) vocabulary.
fn compute_yules_k(frequency: &HashMap<String, usize>, total_words: usize) -> f64 {
    if total_words == 0 {
        return 0.0;
    }

    let n = total_words as f64;

    // Build frequency spectrum: how many types occur exactly i times
    let mut spectrum: HashMap<usize, usize> = HashMap::new();
    for &count in frequency.values() {
        *spectrum.entry(count).or_insert(0) += 1;
    }

    let sum_i2_vi: f64 = spectrum
        .iter()
        .map(|(&i, &vi)| (i as f64).powi(2) * vi as f64)
        .sum();

    let k = 10_000.0 * (sum_i2_vi - n) / (n * n);
    k.max(0.0)
}

/// Honoré's R statistic.
///
/// R = 100 * log(N) / (1 - V1/V)
/// where N = total tokens, V = vocabulary size, V1 = hapax legomena count.
/// Higher R = richer vocabulary.
fn compute_honores_r(total_words: usize, unique_words: usize, hapax: usize) -> f64 {
    if total_words == 0 || unique_words == 0 {
        return 0.0;
    }

    let n = total_words as f64;
    let v = unique_words as f64;
    let v1 = hapax as f64;

    let denominator = 1.0 - v1 / v;
    if denominator.abs() < f64::EPSILON {
        return 0.0; // All words are hapax (very short text)
    }

    100.0 * n.ln() / denominator
}

/// Brunet's W index.
///
/// W = N^(V^-0.172)
/// Lower W = richer vocabulary.
fn compute_brunets_w(total_words: usize, unique_words: usize) -> f64 {
    if total_words == 0 || unique_words == 0 {
        return 0.0;
    }

    let n = total_words as f64;
    let v = unique_words as f64;

    n.powf(v.powf(-0.172))
}
