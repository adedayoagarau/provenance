use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stylometric analysis profile for a text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylometricProfile {
    pub punctuation_frequency: HashMap<char, usize>,
    pub avg_paragraph_length: f64,
    pub exclamation_ratio: f64,
    pub question_ratio: f64,
    pub comma_ratio: f64,
    pub semicolon_ratio: f64,
}

/// Perform stylometric analysis on text.
pub fn analyze(text: &str) -> StylometricProfile {
    let total_chars = text.len() as f64;

    let punctuation_marks = ['.', ',', ';', ':', '!', '?', '-', '(', ')', '"', '\''];
    let mut punctuation_frequency: HashMap<char, usize> = HashMap::new();

    for ch in text.chars() {
        if punctuation_marks.contains(&ch) {
            *punctuation_frequency.entry(ch).or_insert(0) += 1;
        }
    }

    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let avg_paragraph_length = if !paragraphs.is_empty() {
        paragraphs.iter().map(|p| p.len()).sum::<usize>() as f64 / paragraphs.len() as f64
    } else {
        0.0
    };

    let get_count = |ch: char| -> f64 {
        *punctuation_frequency.get(&ch).unwrap_or(&0) as f64
    };
    let ratio = |ch: char| -> f64 {
        if total_chars > 0.0 { get_count(ch) / total_chars } else { 0.0 }
    };

    let exclamation_ratio = ratio('!');
    let question_ratio = ratio('?');
    let comma_ratio = ratio(',');
    let semicolon_ratio = ratio(';');

    StylometricProfile {
        punctuation_frequency,
        avg_paragraph_length,
        exclamation_ratio,
        question_ratio,
        comma_ratio,
        semicolon_ratio,
    }
}
