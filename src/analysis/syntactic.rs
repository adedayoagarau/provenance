use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Syntactic analysis profile for a text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntacticProfile {
    pub total_sentences: usize,
    pub avg_sentence_length: f64,
    pub sentence_length_variance: f64,
    pub avg_words_per_sentence: f64,

    /// Distribution of sentence lengths (word count → frequency)
    pub sentence_length_distribution: HashMap<usize, usize>,

    /// Sentence type counts
    pub declarative_count: usize,
    pub interrogative_count: usize,
    pub exclamatory_count: usize,

    /// Sentence type ratios (out of total sentences)
    pub interrogative_ratio: f64,
    pub exclamatory_ratio: f64,

    /// Most common sentence-opening words (first word → count)
    pub sentence_openings: HashMap<String, usize>,

    /// Estimated passive voice ratio (heuristic)
    pub passive_voice_ratio: f64,
}

/// Perform syntactic analysis on text.
pub fn analyze(text: &str) -> SyntacticProfile {
    let sentences = split_sentences(text);
    let total_sentences = sentences.len();

    if total_sentences == 0 {
        return SyntacticProfile {
            total_sentences: 0,
            avg_sentence_length: 0.0,
            sentence_length_variance: 0.0,
            avg_words_per_sentence: 0.0,
            sentence_length_distribution: HashMap::new(),
            declarative_count: 0,
            interrogative_count: 0,
            exclamatory_count: 0,
            interrogative_ratio: 0.0,
            exclamatory_ratio: 0.0,
            sentence_openings: HashMap::new(),
            passive_voice_ratio: 0.0,
        };
    }

    let sentence_word_counts: Vec<usize> = sentences
        .iter()
        .map(|s| s.split_whitespace().count())
        .collect();

    let total_words: usize = sentence_word_counts.iter().sum();
    let avg_words_per_sentence = total_words as f64 / total_sentences as f64;

    let sentence_lengths: Vec<usize> = sentences.iter().map(|s| s.len()).collect();
    let avg_sentence_length = sentence_lengths.iter().sum::<usize>() as f64 / total_sentences as f64;

    let sentence_length_variance = if total_sentences > 1 {
        let mean = avg_words_per_sentence;
        let sum_sq: f64 = sentence_word_counts
            .iter()
            .map(|&len| {
                let diff = len as f64 - mean;
                diff * diff
            })
            .sum();
        sum_sq / (total_sentences - 1) as f64
    } else {
        0.0
    };

    // Sentence length distribution (by word count)
    let mut sentence_length_distribution: HashMap<usize, usize> = HashMap::new();
    for &wc in &sentence_word_counts {
        *sentence_length_distribution.entry(wc).or_insert(0) += 1;
    }

    // Sentence type classification
    let mut interrogative_count = 0;
    let mut exclamatory_count = 0;
    let mut passive_count = 0;
    let mut sentence_openings: HashMap<String, usize> = HashMap::new();

    for sentence in &sentences {
        let trimmed = sentence.trim();
        if trimmed.ends_with('?') {
            interrogative_count += 1;
        } else if trimmed.ends_with('!') {
            exclamatory_count += 1;
        }

        // Sentence opening word
        if let Some(first_word) = trimmed.split_whitespace().next() {
            let word = first_word.to_lowercase()
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>();
            if !word.is_empty() {
                *sentence_openings.entry(word).or_insert(0) += 1;
            }
        }

        // Passive voice heuristic: "was/were/is/are/been/being + past participle"
        if detect_passive(trimmed) {
            passive_count += 1;
        }
    }

    let declarative_count = total_sentences - interrogative_count - exclamatory_count;
    let n = total_sentences as f64;

    SyntacticProfile {
        total_sentences,
        avg_sentence_length,
        sentence_length_variance,
        avg_words_per_sentence,
        sentence_length_distribution,
        declarative_count,
        interrogative_count,
        exclamatory_count,
        interrogative_ratio: interrogative_count as f64 / n,
        exclamatory_ratio: exclamatory_count as f64 / n,
        sentence_openings,
        passive_voice_ratio: passive_count as f64 / n,
    }
}

/// Split text into sentences (simple heuristic).
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for i in 0..len {
        current.push(chars[i]);

        let is_terminal = chars[i] == '.' || chars[i] == '!' || chars[i] == '?';
        if is_terminal {
            // Check it's not an abbreviation (e.g., "Dr.", "Mr.", "U.S.")
            let next_is_space_or_end = i + 1 >= len
                || chars[i + 1].is_whitespace()
                || chars[i + 1] == '"'
                || chars[i + 1] == '\''
                || chars[i + 1] == ')';

            if next_is_space_or_end {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() && trimmed.split_whitespace().count() > 0 {
                    sentences.push(trimmed);
                }
                current.clear();
            }
        }
    }

    // Remaining text
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() && trimmed.split_whitespace().count() > 1 {
        sentences.push(trimmed);
    }

    sentences
}

/// Simple heuristic for passive voice detection.
/// Looks for: be-form + word ending in -ed/-en
fn detect_passive(sentence: &str) -> bool {
    let words: Vec<&str> = sentence.split_whitespace().collect();
    let be_forms = ["was", "were", "is", "are", "been", "being", "be"];

    for i in 0..words.len().saturating_sub(1) {
        let word = words[i].to_lowercase();
        let clean: String = word.chars().filter(|c| c.is_alphabetic()).collect();

        if be_forms.contains(&clean.as_str()) {
            // Check if next word looks like past participle
            if let Some(next) = words.get(i + 1) {
                let next_clean: String = next.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .collect();
                if next_clean.ends_with("ed") || next_clean.ends_with("en")
                    || next_clean.ends_with("wn") || next_clean.ends_with("nt")
                {
                    return true;
                }
            }
        }
    }
    false
}
