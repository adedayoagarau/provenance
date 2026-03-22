use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Stylometric analysis profile for a text.
///
/// Captures punctuation habits, formatting preferences, and word-choice patterns
/// that serve as authorship indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylometricProfile {
    pub punctuation_frequency: HashMap<char, usize>,
    pub avg_paragraph_length: f64,

    // Basic punctuation ratios (per total characters)
    pub exclamation_ratio: f64,
    pub question_ratio: f64,
    pub comma_ratio: f64,
    pub semicolon_ratio: f64,
    pub colon_ratio: f64,

    // Advanced punctuation patterns
    pub em_dash_count: usize,
    pub en_dash_count: usize,
    pub hyphen_count: usize,
    pub ellipsis_count: usize,
    pub parenthetical_count: usize,
    pub single_quote_count: usize,
    pub double_quote_count: usize,

    // Contraction usage
    pub contraction_count: usize,
    pub contraction_ratio: f64,

    // Hedge words (uncertainty markers)
    pub hedge_word_count: usize,
    pub hedge_word_ratio: f64,

    // Intensifiers (emphasis markers)
    pub intensifier_count: usize,
    pub intensifier_ratio: f64,

    // Paragraph structure
    pub paragraph_count: usize,
    pub paragraph_length_variance: f64,
    pub short_paragraph_ratio: f64,
}

/// Perform stylometric analysis on text.
pub fn analyze(text: &str) -> StylometricProfile {
    let total_chars = text.len() as f64;
    let words: Vec<String> = text.unicode_words().map(|w| w.to_lowercase()).collect();
    let total_words = words.len();

    // Punctuation frequency
    let punctuation_marks = ['.', ',', ';', ':', '!', '?', '-', '(', ')', '"', '\'',
                             '\u{2014}', '\u{2013}', '\u{2026}', '\u{2018}', '\u{2019}',
                             '\u{201C}', '\u{201D}'];
    let mut punctuation_frequency: HashMap<char, usize> = HashMap::new();
    for ch in text.chars() {
        if punctuation_marks.contains(&ch) {
            *punctuation_frequency.entry(ch).or_insert(0) += 1;
        }
    }

    // Paragraphs
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let paragraph_count = paragraphs.len();

    let avg_paragraph_length = if paragraph_count > 0 {
        paragraphs.iter().map(|p| p.len()).sum::<usize>() as f64 / paragraph_count as f64
    } else {
        0.0
    };

    let paragraph_length_variance = if paragraph_count > 1 {
        let mean = avg_paragraph_length;
        let sum_sq: f64 = paragraphs.iter()
            .map(|p| { let diff = p.len() as f64 - mean; diff * diff })
            .sum();
        sum_sq / (paragraph_count - 1) as f64
    } else {
        0.0
    };

    // Short paragraphs (< 50 chars)
    let short_paragraphs = paragraphs.iter().filter(|p| p.len() < 50).count();
    let short_paragraph_ratio = if paragraph_count > 0 {
        short_paragraphs as f64 / paragraph_count as f64
    } else {
        0.0
    };

    // Punctuation ratios
    let get_count = |ch: char| -> f64 { *punctuation_frequency.get(&ch).unwrap_or(&0) as f64 };
    let char_ratio = |ch: char| -> f64 {
        if total_chars > 0.0 { get_count(ch) / total_chars } else { 0.0 }
    };

    let exclamation_ratio = char_ratio('!');
    let question_ratio = char_ratio('?');
    let comma_ratio = char_ratio(',');
    let semicolon_ratio = char_ratio(';');
    let colon_ratio = char_ratio(':');

    // Advanced punctuation
    let em_dash_count = count_pattern(text, "\u{2014}") + count_pattern(text, "---") + count_pattern(text, " -- ");
    let en_dash_count = count_char(text, '\u{2013}');
    let hyphen_count = text.chars().filter(|&c| c == '-').count()
        .saturating_sub(em_dash_count * 3); // subtract dashes used in em-dash patterns
    let ellipsis_count = count_char(text, '\u{2026}') + count_pattern(text, "...");
    let parenthetical_count = count_char(text, '(');
    let single_quote_count = count_char(text, '\u{2018}') + count_char(text, '\u{2019}');
    let double_quote_count = count_char(text, '\u{201C}') + count_char(text, '\u{201D}')
        + count_char(text, '"');

    // Contractions
    let contraction_count = count_contractions(&words);
    let contraction_ratio = if total_words > 0 {
        contraction_count as f64 / total_words as f64
    } else {
        0.0
    };

    // Hedge words
    let hedge_word_count = count_word_list(&words, &HEDGE_WORDS);
    let hedge_word_ratio = if total_words > 0 {
        hedge_word_count as f64 / total_words as f64
    } else {
        0.0
    };

    // Intensifiers
    let intensifier_count = count_word_list(&words, &INTENSIFIERS);
    let intensifier_ratio = if total_words > 0 {
        intensifier_count as f64 / total_words as f64
    } else {
        0.0
    };

    StylometricProfile {
        punctuation_frequency,
        avg_paragraph_length,
        exclamation_ratio,
        question_ratio,
        comma_ratio,
        semicolon_ratio,
        colon_ratio,
        em_dash_count,
        en_dash_count,
        hyphen_count,
        ellipsis_count,
        parenthetical_count,
        single_quote_count,
        double_quote_count,
        contraction_count,
        contraction_ratio,
        hedge_word_count,
        hedge_word_ratio,
        intensifier_count,
        intensifier_ratio,
        paragraph_count,
        paragraph_length_variance,
        short_paragraph_ratio,
    }
}

fn count_char(text: &str, ch: char) -> usize {
    text.chars().filter(|&c| c == ch).count()
}

fn count_pattern(text: &str, pattern: &str) -> usize {
    text.matches(pattern).count()
}

fn count_contractions(words: &[String]) -> usize {
    // Common English contractions
    let contractions = [
        "don't", "doesn't", "didn't", "won't", "wouldn't", "can't", "couldn't",
        "shouldn't", "isn't", "aren't", "wasn't", "weren't", "hasn't", "haven't",
        "hadn't", "i'm", "i've", "i'll", "i'd", "you're", "you've", "you'll",
        "you'd", "he's", "he'll", "he'd", "she's", "she'll", "she'd",
        "it's", "it'll", "we're", "we've", "we'll", "we'd",
        "they're", "they've", "they'll", "they'd",
        "that's", "that'll", "that'd", "who's", "who'll", "who'd",
        "what's", "what'll", "what'd", "where's", "where'd",
        "there's", "there'd", "here's", "let's",
        "ain't", "o'clock", "ma'am", "y'all",
        "cannot", // not a contraction but often tested alongside
    ];
    let set: std::collections::HashSet<&str> = contractions.into_iter().collect();

    words.iter().filter(|w| {
        // Check exact match or if the word contains an apostrophe
        set.contains(w.as_str()) || (w.contains('\'') && w.len() > 2)
    }).count()
}

fn count_word_list(words: &[String], list: &[&str]) -> usize {
    let set: std::collections::HashSet<&str> = list.iter().copied().collect();
    words.iter().filter(|w| set.contains(w.as_str())).count()
}

/// Hedge words: markers of uncertainty, tentativeness, or qualification.
/// These are stylistic markers — some authors hedge frequently, others don't.
const HEDGE_WORDS: &[&str] = &[
    "perhaps", "maybe", "possibly", "probably", "presumably",
    "apparently", "seemingly", "supposedly", "arguably",
    "somewhat", "rather", "fairly", "quite", "relatively",
    "roughly", "approximately", "generally", "typically",
    "usually", "often", "sometimes", "occasionally",
    "might", "could", "may", "seem", "seems", "seemed",
    "appear", "appears", "appeared", "suggest", "suggests",
    "tend", "tends", "tended",
    "likely", "unlikely", "possible", "impossible",
    "certain", "uncertain", "unclear",
    "almost", "nearly", "virtually", "essentially",
    "basically", "fundamentally",
];

/// Intensifiers: emphasis markers.
const INTENSIFIERS: &[&str] = &[
    "very", "extremely", "incredibly", "remarkably", "exceptionally",
    "absolutely", "completely", "totally", "entirely", "utterly",
    "thoroughly", "perfectly", "genuinely", "truly", "really",
    "deeply", "highly", "greatly", "strongly", "firmly",
    "particularly", "especially", "specifically",
    "definitely", "certainly", "surely", "clearly", "obviously",
    "undoubtedly", "unquestionably",
    "so", "too", "quite", "awfully", "terribly",
];
