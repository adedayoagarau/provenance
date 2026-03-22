use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Character and word n-gram frequency profiles.
///
/// Character n-grams (especially 2-5 grams) are proven highly effective for
/// authorship attribution. They capture subword patterns, punctuation habits,
/// spacing preferences, and morphological tendencies. They are language-independent
/// and resistant to topic confounds.
///
/// References:
/// - Stamatatos (2009, 2017): Character n-grams effectiveness
/// - Kestemont (2014): Character n-grams in cross-domain attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgramProfile {
    /// Character n-gram frequencies by n-value (2, 3, 4, 5)
    /// Each map: n-gram string → relative frequency
    pub char_bigrams: HashMap<String, f64>,
    pub char_trigrams: HashMap<String, f64>,
    pub char_fourgrams: HashMap<String, f64>,
    pub char_fivegrams: HashMap<String, f64>,

    /// Word bigram frequencies (top-K)
    pub word_bigrams: HashMap<String, f64>,

    /// Total character count analyzed
    pub total_chars: usize,
    /// Total words analyzed
    pub total_words: usize,
}

/// Maximum number of n-grams to keep per n-value.
const TOP_K: usize = 500;

/// Analyze character and word n-gram frequencies.
pub fn analyze(text: &str) -> NgramProfile {
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    let char_bigrams = compute_char_ngrams(&chars, 2);
    let char_trigrams = compute_char_ngrams(&chars, 3);
    let char_fourgrams = compute_char_ngrams(&chars, 4);
    let char_fivegrams = compute_char_ngrams(&chars, 5);

    let words: Vec<&str> = text.split_whitespace().collect();
    let total_words = words.len();
    let word_bigrams = compute_word_bigrams(&words);

    NgramProfile {
        char_bigrams,
        char_trigrams,
        char_fourgrams,
        char_fivegrams,
        word_bigrams,
        total_chars,
        total_words,
    }
}

/// Compute character n-gram frequencies, keeping top-K.
fn compute_char_ngrams(chars: &[char], n: usize) -> HashMap<String, f64> {
    if chars.len() < n {
        return HashMap::new();
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    let total = chars.len() - n + 1;

    for window in chars.windows(n) {
        let ngram: String = window.iter().collect();
        *counts.entry(ngram).or_insert(0) += 1;
    }

    top_k_normalized(counts, total, TOP_K)
}

/// Compute word bigram frequencies, keeping top-K.
fn compute_word_bigrams(words: &[&str]) -> HashMap<String, f64> {
    if words.len() < 2 {
        return HashMap::new();
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    let total = words.len() - 1;

    for pair in words.windows(2) {
        let bigram = format!("{} {}", pair[0].to_lowercase(), pair[1].to_lowercase());
        *counts.entry(bigram).or_insert(0) += 1;
    }

    top_k_normalized(counts, total, TOP_K)
}

/// Keep only top-K entries by count, normalize to relative frequency.
fn top_k_normalized(counts: HashMap<String, usize>, total: usize, k: usize) -> HashMap<String, f64> {
    if total == 0 {
        return HashMap::new();
    }

    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(k);

    sorted
        .into_iter()
        .map(|(ngram, count)| (ngram, count as f64 / total as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_bigrams() {
        let profile = analyze("hello world");
        assert!(!profile.char_bigrams.is_empty());
        // "he", "el", "ll", "lo", "o ", " w", "wo", "or", "rl", "ld"
        assert!(profile.char_bigrams.contains_key("he"));
        assert!(profile.char_bigrams.contains_key("ll"));
    }

    #[test]
    fn test_char_trigrams() {
        let profile = analyze("hello world");
        assert!(!profile.char_trigrams.is_empty());
        assert!(profile.char_trigrams.contains_key("hel"));
    }

    #[test]
    fn test_word_bigrams() {
        let text = "the quick brown fox the quick red fox";
        let profile = analyze(text);
        assert!(profile.word_bigrams.contains_key("the quick"));
        // "the quick" appears twice out of 7 bigrams
        let freq = profile.word_bigrams["the quick"];
        assert!((freq - 2.0 / 7.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_text() {
        let profile = analyze("");
        assert!(profile.char_bigrams.is_empty());
        assert_eq!(profile.total_chars, 0);
    }

    #[test]
    fn test_short_text() {
        let profile = analyze("a");
        assert!(profile.char_bigrams.is_empty()); // need at least 2 chars
        assert_eq!(profile.total_chars, 1);
    }
}
