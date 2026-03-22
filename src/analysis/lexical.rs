use anyhow::Result;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Lexical analysis profile for a text.
#[derive(Debug, Clone)]
pub struct LexicalProfile {
    pub total_words: usize,
    pub unique_words: usize,
    pub type_token_ratio: f64,
    pub hapax_legomena: usize,
    pub hapax_ratio: f64,
    pub avg_word_length: f64,
    pub word_frequency: HashMap<String, usize>,
}

/// Perform lexical analysis on text.
pub fn analyze(text: &str) -> Result<LexicalProfile> {
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
    let type_token_ratio = if total_words > 0 {
        unique_words as f64 / total_words as f64
    } else {
        0.0
    };

    let hapax_legomena = frequency.values().filter(|&&count| count == 1).count();
    let hapax_ratio = if total_words > 0 {
        hapax_legomena as f64 / total_words as f64
    } else {
        0.0
    };

    let total_chars: usize = words.iter().map(|w| w.len()).sum();
    let avg_word_length = if total_words > 0 {
        total_chars as f64 / total_words as f64
    } else {
        0.0
    };

    Ok(LexicalProfile {
        total_words,
        unique_words,
        type_token_ratio,
        hapax_legomena,
        hapax_ratio,
        avg_word_length,
        word_frequency: frequency,
    })
}
