use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Discourse and readability profile for a text.
///
/// Note: Topic/keyword analysis is intentionally excluded because topic
/// content confounds authorship signals. Instead, this module focuses on
/// discourse markers (which ARE stylistic) and readability scores
/// (which correlate with author style).
///
/// References:
/// - Stamatatos (2009): Topic confounds in authorship attribution
/// - Argamon et al. (2007): Discourse features as style markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticProfile {
    pub paragraph_count: usize,

    /// Discourse marker frequencies (marker → count per 1000 words)
    pub discourse_markers: HashMap<String, f64>,
    /// Total discourse marker usage ratio
    pub discourse_marker_ratio: f64,

    /// Readability scores
    pub flesch_kincaid_grade: f64,
    pub gunning_fog_index: f64,
    pub coleman_liau_index: f64,
    pub automated_readability_index: f64,

    /// Average syllables per word (used in readability formulas)
    pub avg_syllables_per_word: f64,
}

/// Perform discourse and readability analysis on text.
pub fn analyze(text: &str) -> SemanticProfile {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let paragraph_count = paragraphs.len();

    let words: Vec<String> = text.unicode_words().map(|w| w.to_lowercase()).collect();
    let total_words = words.len();

    // Discourse markers
    let (discourse_markers, discourse_marker_ratio) = analyze_discourse_markers(&words, total_words);

    // Readability
    let sentences = count_sentences(text);
    let syllable_counts: Vec<usize> = words.iter().map(|w| count_syllables(w)).collect();
    let total_syllables: usize = syllable_counts.iter().sum();
    let avg_syllables_per_word = if total_words > 0 {
        total_syllables as f64 / total_words as f64
    } else {
        0.0
    };

    let complex_words = syllable_counts.iter().filter(|&&s| s >= 3).count();
    let total_chars: usize = words.iter().map(|w| w.len()).sum();

    let flesch_kincaid_grade = compute_flesch_kincaid(total_words, sentences, total_syllables);
    let gunning_fog_index = compute_gunning_fog(total_words, sentences, complex_words);
    let coleman_liau_index = compute_coleman_liau(total_chars, total_words, sentences);
    let automated_readability_index = compute_ari(total_chars, total_words, sentences);

    SemanticProfile {
        paragraph_count,
        discourse_markers,
        discourse_marker_ratio,
        flesch_kincaid_grade,
        gunning_fog_index,
        coleman_liau_index,
        automated_readability_index,
        avg_syllables_per_word,
    }
}

/// Analyze discourse marker usage.
fn analyze_discourse_markers(words: &[String], total_words: usize) -> (HashMap<String, f64>, f64) {
    if total_words == 0 {
        return (HashMap::new(), 0.0);
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total_markers = 0;

    // Single-word markers
    let single_markers: std::collections::HashSet<&str> = [
        "however", "therefore", "thus", "hence", "moreover", "furthermore",
        "nevertheless", "nonetheless", "meanwhile", "otherwise", "instead",
        "accordingly", "consequently", "subsequently", "conversely",
        "alternatively", "similarly", "likewise", "indeed", "certainly",
        "specifically", "particularly", "notably", "importantly",
        "additionally", "finally", "initially", "ultimately",
        "essentially", "basically", "obviously", "clearly",
        "apparently", "evidently", "presumably", "arguably",
        "admittedly", "granted", "regardless", "incidentally",
    ].into_iter().collect();

    for word in words {
        if single_markers.contains(word.as_str()) {
            *counts.entry(word.clone()).or_insert(0) += 1;
            total_markers += 1;
        }
    }

    // Multi-word markers (check bigrams/trigrams)
    let multi_markers = [
        "in addition", "on the other hand", "for example", "for instance",
        "in contrast", "in fact", "as a result", "in other words",
        "in particular", "on the contrary", "at the same time",
        "in the meantime", "by contrast", "in conclusion",
        "to begin with", "first of all", "in summary",
        "to summarize", "in short", "after all", "above all",
        "as well", "even so", "that is", "in general",
    ];

    let text_lower: String = words.join(" ");
    for marker in &multi_markers {
        let count = text_lower.matches(marker).count();
        if count > 0 {
            *counts.entry(marker.to_string()).or_insert(0) += count;
            total_markers += count;
        }
    }

    // Normalize to per 1000 words
    let scale = 1000.0 / total_words as f64;
    let frequencies: HashMap<String, f64> = counts
        .into_iter()
        .map(|(marker, count)| (marker, count as f64 * scale))
        .collect();

    let ratio = total_markers as f64 / total_words as f64;

    (frequencies, ratio)
}

/// Count sentences in text (simple heuristic).
fn count_sentences(text: &str) -> usize {
    let count = text.chars()
        .filter(|&c| c == '.' || c == '!' || c == '?')
        .count();
    count.max(1) // At least 1 sentence
}

/// Count syllables in a word (English heuristic).
///
/// Uses a simplified algorithm that counts vowel groups.
fn count_syllables(word: &str) -> usize {
    if word.is_empty() {
        return 0;
    }
    if word.len() <= 3 {
        return 1;
    }

    let word = word.to_lowercase();
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut count = 0;
    let mut prev_vowel = false;
    let chars: Vec<char> = word.chars().collect();

    for &ch in &chars {
        let is_vowel = vowels.contains(&ch);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }

    // Silent 'e' at end
    if word.ends_with('e') && count > 1 {
        count -= 1;
    }

    // Words ending in 'le' after consonant
    if word.ends_with("le") && chars.len() > 2 {
        let before_le = chars[chars.len() - 3];
        if !vowels.contains(&before_le) && count == 0 {
            count = 1;
        }
    }

    count.max(1)
}

/// Flesch-Kincaid Grade Level.
/// FK = 0.39 * (words/sentences) + 11.8 * (syllables/words) - 15.59
fn compute_flesch_kincaid(words: usize, sentences: usize, syllables: usize) -> f64 {
    if words == 0 || sentences == 0 {
        return 0.0;
    }
    let wps = words as f64 / sentences as f64;
    let spw = syllables as f64 / words as f64;
    0.39 * wps + 11.8 * spw - 15.59
}

/// Gunning Fog Index.
/// GF = 0.4 * ((words/sentences) + 100 * (complex_words/words))
fn compute_gunning_fog(words: usize, sentences: usize, complex_words: usize) -> f64 {
    if words == 0 || sentences == 0 {
        return 0.0;
    }
    let wps = words as f64 / sentences as f64;
    let cwp = complex_words as f64 / words as f64;
    0.4 * (wps + 100.0 * cwp)
}

/// Coleman-Liau Index.
/// CLI = 0.0588 * L - 0.296 * S - 15.8
/// where L = avg letters per 100 words, S = avg sentences per 100 words
fn compute_coleman_liau(chars: usize, words: usize, sentences: usize) -> f64 {
    if words == 0 {
        return 0.0;
    }
    let l = chars as f64 / words as f64 * 100.0;
    let s = sentences as f64 / words as f64 * 100.0;
    0.0588 * l - 0.296 * s - 15.8
}

/// Automated Readability Index.
/// ARI = 4.71 * (chars/words) + 0.5 * (words/sentences) - 21.43
fn compute_ari(chars: usize, words: usize, sentences: usize) -> f64 {
    if words == 0 || sentences == 0 {
        return 0.0;
    }
    4.71 * (chars as f64 / words as f64) + 0.5 * (words as f64 / sentences as f64) - 21.43
}
