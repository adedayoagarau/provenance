//! Tier 3–5 detection features for AI writing analysis.
//!
//! 18 additional features covering vocabulary sophistication, syntactic patterns,
//! register consistency, and formatting. These have lower individual effect sizes
//! (d ≈ 0.2–0.4) but strengthen the ensemble signal significantly.
//!
//! Tier 3 (d ≈ 0.35–0.45): Vocabulary sophistication curves, synonym over-variation,
//!     determiner-noun distance, modal verb patterns
//! Tier 4 (d ≈ 0.25–0.35): Register consistency, connective redundancy,
//!     adverb placement, pronoun shift patterns
//! Tier 5 (d ≈ 0.15–0.25): Punctuation diversity, quote integration,
//!     list/enumeration patterns, paragraph transition coherence

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use unicode_segmentation::UnicodeSegmentation;

/// Combined Tier 3–5 feature results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatureResult {
    // --- Tier 3: Vocabulary & Syntax (d ≈ 0.35-0.45) ---

    /// Vocabulary sophistication curve: change in avg word length over text quarters.
    /// Human: tends to increase slightly; AI: remains flat.
    pub vocab_sophistication_slope: Option<f64>,

    /// Synonym variation rate: unique words per content word position.
    /// AI over-varies (too many near-synonyms in short spans).
    pub synonym_overvariation: Option<f64>,

    /// Mean distance (in words) between determiners and their head nouns.
    /// AI tends toward shorter, simpler NP structures.
    pub determiner_noun_distance: Option<f64>,

    /// Modal verb density (per 1000 words).
    /// AI over-uses certain modals (can, will, would).
    pub modal_density: Option<f64>,

    /// Modal diversity: unique modals / total modal uses.
    pub modal_diversity: Option<f64>,

    // --- Tier 4: Register & Coherence (d ≈ 0.25-0.35) ---

    /// Register consistency: variance of formality scores across paragraphs.
    /// Lower variance = more consistent = more AI-like.
    pub register_consistency: Option<f64>,

    /// Connective redundancy: ratio of redundant connective pairs.
    /// AI tends to over-connect ideas with explicit markers.
    pub connective_redundancy: Option<f64>,

    /// Sentence-initial adverb ratio.
    /// AI favors sentence-initial adverbs more than humans.
    pub initial_adverb_ratio: Option<f64>,

    /// Pronoun referent shift rate: how often pronoun subjects change.
    /// AI tends to maintain the same subject longer.
    pub pronoun_shift_rate: Option<f64>,

    // --- Tier 5: Formatting & Surface (d ≈ 0.15-0.25) ---

    /// Punctuation diversity index (Shannon entropy of punctuation types).
    /// Lower = less diverse = more AI-like.
    pub punctuation_diversity: Option<f64>,

    /// Quote integration smoothness: ratio of quotes with signal phrases.
    /// AI tends to integrate quotes more formulaically.
    pub quote_integration_ratio: Option<f64>,

    /// Enumeration pattern ratio: sentences with list markers.
    /// AI over-uses enumeration (firstly, secondly, etc.).
    pub enumeration_ratio: Option<f64>,

    /// Paragraph transition coherence: lexical overlap between adjacent paragraphs.
    /// AI tends to have lower overlap (topic jumps).
    pub paragraph_transition_overlap: Option<f64>,

    /// Number of features successfully computed.
    pub features_computed: usize,
}

/// Analyze Tier 3–5 features.
pub fn analyze(text: &str) -> AdvancedFeatureResult {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();
    let word_count = words.len();
    let mut features_computed = 0;

    let sentences = split_sentences(text);
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    // --- Tier 3 ---

    // 1. Vocabulary sophistication slope
    let vocab_sophistication_slope = if word_count >= 200 {
        let v = compute_vocab_sophistication_slope(&words);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 2. Synonym overvariation
    let synonym_overvariation = if word_count >= 200 {
        let v = compute_synonym_overvariation(&words);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 3. Determiner-noun distance
    let determiner_noun_distance = if word_count >= 100 {
        let v = compute_det_noun_distance(&words);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 4-5. Modal verb patterns
    let (modal_density, modal_diversity) = if word_count >= 200 {
        let (d, div) = compute_modal_patterns(&words);
        features_computed += 2;
        (Some(d), Some(div))
    } else {
        (None, None)
    };

    // --- Tier 4 ---

    // 6. Register consistency
    let register_consistency = if paragraphs.len() >= 3 {
        let v = compute_register_consistency(&paragraphs);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 7. Connective redundancy
    let connective_redundancy = if sentences.len() >= 10 {
        let v = compute_connective_redundancy(&sentences);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 8. Initial adverb ratio
    let initial_adverb_ratio = if sentences.len() >= 10 {
        let v = compute_initial_adverb_ratio(&sentences);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 9. Pronoun shift rate
    let pronoun_shift_rate = if sentences.len() >= 10 {
        let v = compute_pronoun_shift_rate(&sentences);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // --- Tier 5 ---

    // 10. Punctuation diversity
    let punctuation_diversity = if word_count >= 200 {
        let v = compute_punctuation_diversity(text);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 11. Quote integration
    let quote_integration_ratio = if text.contains('"') || text.contains('\u{201C}') {
        let v = compute_quote_integration(text);
        if v >= 0.0 {
            features_computed += 1;
            Some(v)
        } else {
            None
        }
    } else {
        None
    };

    // 12. Enumeration ratio
    let enumeration_ratio = if sentences.len() >= 10 {
        let v = compute_enumeration_ratio(&sentences);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 13. Paragraph transition overlap
    let paragraph_transition_overlap = if paragraphs.len() >= 3 {
        let v = compute_paragraph_overlap(&paragraphs);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    AdvancedFeatureResult {
        vocab_sophistication_slope,
        synonym_overvariation,
        determiner_noun_distance,
        modal_density,
        modal_diversity,
        register_consistency,
        connective_redundancy,
        initial_adverb_ratio,
        pronoun_shift_rate,
        punctuation_diversity,
        quote_integration_ratio,
        enumeration_ratio,
        paragraph_transition_overlap,
        features_computed,
    }
}

/// Vocabulary sophistication: slope of average word length across quarters.
fn compute_vocab_sophistication_slope(words: &[String]) -> f64 {
    let quarter = words.len() / 4;
    if quarter < 20 {
        return 0.0;
    }

    let quarter_means: Vec<f64> = (0..4)
        .map(|i| {
            let chunk = &words[i * quarter..(i + 1) * quarter];
            chunk.iter().map(|w| w.len() as f64).sum::<f64>() / chunk.len() as f64
        })
        .collect();

    // Simple slope: (last - first) / 3
    (quarter_means[3] - quarter_means[0]) / 3.0
}

/// Synonym overvariation: ratio of unique content words to total content words
/// in small windows. AI generates higher unique-to-total ratios.
fn compute_synonym_overvariation(words: &[String]) -> f64 {
    let func_set: HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "this", "that", "it", "he", "she", "they",
        "we", "you", "i", "not", "no", "so", "very", "just",
    ].into_iter().collect();

    let content_words: Vec<&str> = words.iter()
        .filter(|w| !func_set.contains(w.as_str()) && w.len() > 2)
        .map(|w| w.as_str())
        .collect();

    if content_words.len() < 50 {
        return 0.0;
    }

    // Windowed unique ratio
    let window_size = 50;
    let mut ratios = Vec::new();
    for chunk in content_words.chunks(window_size) {
        let unique: HashSet<&&str> = chunk.iter().collect();
        ratios.push(unique.len() as f64 / chunk.len() as f64);
    }

    if ratios.is_empty() {
        return 0.0;
    }

    ratios.iter().sum::<f64>() / ratios.len() as f64
}

/// Mean distance between determiners and the next noun-like word.
fn compute_det_noun_distance(words: &[String]) -> f64 {
    let determiners: HashSet<&str> = [
        "the", "a", "an", "this", "that", "these", "those",
        "my", "your", "his", "her", "its", "our", "their",
        "some", "any", "every", "each", "no",
    ].into_iter().collect();

    let mut total_distance = 0;
    let mut count = 0;

    for (i, word) in words.iter().enumerate() {
        if determiners.contains(word.as_str()) {
            // Find next word that looks like a noun (not an adjective/adverb)
            for j in (i + 1)..words.len().min(i + 8) {
                let w = &words[j];
                // Heuristic: nouns don't end in -ly, -ful, -ous, -ive
                if w.len() > 2
                    && !w.ends_with("ly")
                    && !w.ends_with("ful")
                    && !w.ends_with("ous")
                    && !w.ends_with("ive")
                    && !determiners.contains(w.as_str())
                {
                    total_distance += j - i;
                    count += 1;
                    break;
                }
            }
        }
    }

    if count > 0 {
        total_distance as f64 / count as f64
    } else {
        0.0
    }
}

/// Modal verb patterns: density and diversity.
fn compute_modal_patterns(words: &[String]) -> (f64, f64) {
    let modals: HashSet<&str> = [
        "can", "could", "will", "would", "shall", "should",
        "may", "might", "must",
    ].into_iter().collect();

    let mut modal_counts: HashMap<&str, usize> = HashMap::new();
    let mut total = 0;

    for word in words {
        if modals.contains(word.as_str()) {
            *modal_counts.entry(word.as_str()).or_insert(0) += 1;
            total += 1;
        }
    }

    let density = total as f64 / words.len() as f64 * 1000.0;
    let diversity = if total > 0 {
        modal_counts.len() as f64 / total as f64
    } else {
        0.0
    };

    (density, diversity)
}

/// Register consistency: variance of formality proxy across paragraphs.
fn compute_register_consistency(paragraphs: &[&str]) -> f64 {
    let formality_scores: Vec<f64> = paragraphs.iter()
        .filter(|p| p.split_whitespace().count() >= 10)
        .map(|p| estimate_formality(p))
        .collect();

    if formality_scores.len() < 3 {
        return 0.0;
    }

    let mean = formality_scores.iter().sum::<f64>() / formality_scores.len() as f64;
    let var = formality_scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>()
        / (formality_scores.len() - 1) as f64;
    var
}

/// Simple formality estimator (average word length + passive ratio proxy).
fn estimate_formality(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }

    let avg_word_len = words.iter().map(|w| w.len() as f64).sum::<f64>() / words.len() as f64;
    let long_word_ratio = words.iter().filter(|w| w.len() > 8).count() as f64 / words.len() as f64;

    avg_word_len * 0.6 + long_word_ratio * 10.0 * 0.4
}

/// Connective redundancy: consecutive sentences both starting with connectives.
fn compute_connective_redundancy(sentences: &[String]) -> f64 {
    let connectives: HashSet<&str> = [
        "however", "therefore", "furthermore", "moreover", "additionally",
        "consequently", "nevertheless", "meanwhile", "similarly", "likewise",
        "thus", "hence", "accordingly", "subsequently",
    ].into_iter().collect();

    let mut redundant = 0;
    for i in 0..sentences.len().saturating_sub(1) {
        let first_a = sentences[i].split_whitespace().next()
            .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect::<String>());
        let first_b = sentences[i + 1].split_whitespace().next()
            .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect::<String>());

        if let (Some(a), Some(b)) = (first_a, first_b) {
            if connectives.contains(a.as_str()) && connectives.contains(b.as_str()) {
                redundant += 1;
            }
        }
    }

    redundant as f64 / (sentences.len().saturating_sub(1).max(1)) as f64
}

/// Ratio of sentences starting with adverbs.
fn compute_initial_adverb_ratio(sentences: &[String]) -> f64 {
    let count = sentences.iter().filter(|s| {
        if let Some(first) = s.split_whitespace().next() {
            let clean: String = first.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
            clean.ends_with("ly") && clean.len() > 3
        } else {
            false
        }
    }).count();

    count as f64 / sentences.len() as f64
}

/// How often the pronoun subject changes between consecutive sentences.
fn compute_pronoun_shift_rate(sentences: &[String]) -> f64 {
    let subject_pronouns: HashSet<&str> = [
        "i", "you", "he", "she", "it", "we", "they",
    ].into_iter().collect();

    let mut prev_subject: Option<String> = None;
    let mut shifts = 0;
    let mut comparisons = 0;

    for sentence in sentences {
        let first_word: String = sentence.split_whitespace()
            .next()
            .unwrap_or("")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect();

        if subject_pronouns.contains(first_word.as_str()) {
            if let Some(ref prev) = prev_subject {
                comparisons += 1;
                if prev != &first_word {
                    shifts += 1;
                }
            }
            prev_subject = Some(first_word);
        }
    }

    if comparisons > 0 {
        shifts as f64 / comparisons as f64
    } else {
        0.0
    }
}

/// Shannon entropy of punctuation character types.
fn compute_punctuation_diversity(text: &str) -> f64 {
    let punct_chars = ['.', ',', ';', ':', '!', '?', '-', '(', ')', '"', '\'',
        '\u{2014}', '\u{2013}', '\u{2026}'];

    let mut counts: HashMap<char, usize> = HashMap::new();
    let mut total = 0;

    for ch in text.chars() {
        if punct_chars.contains(&ch) {
            *counts.entry(ch).or_insert(0) += 1;
            total += 1;
        }
    }

    if total == 0 {
        return 0.0;
    }

    let n = total as f64;
    let mut entropy = 0.0;
    for &count in counts.values() {
        if count > 0 {
            let p = count as f64 / n;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Ratio of quotes that have signal phrases ("said", "according to", etc.).
fn compute_quote_integration(text: &str) -> f64 {
    let quote_markers = ['"', '\u{201C}', '\u{201D}'];
    let signal_phrases = [
        "said", "says", "according", "stated", "noted", "argued",
        "claimed", "explained", "wrote", "observed", "remarked",
        "suggested", "added", "replied", "responded",
    ];

    let lower = text.to_lowercase();
    let quote_count = text.chars().filter(|c| quote_markers.contains(c)).count() / 2;

    if quote_count == 0 {
        return -1.0; // No quotes to analyze
    }

    let signal_near_quote = signal_phrases.iter()
        .map(|sp| lower.matches(sp).count())
        .sum::<usize>();

    (signal_near_quote as f64 / quote_count as f64).min(1.0)
}

/// Ratio of sentences with enumeration markers.
fn compute_enumeration_ratio(sentences: &[String]) -> f64 {
    let enum_markers = [
        "first", "firstly", "second", "secondly", "third", "thirdly",
        "finally", "lastly", "next", "additionally", "furthermore",
        "in addition", "on one hand", "on the other hand",
    ];

    let count = sentences.iter().filter(|s| {
        let lower = s.to_lowercase();
        enum_markers.iter().any(|m| lower.starts_with(m) || lower.contains(&format!(", {}", m)))
    }).count();

    count as f64 / sentences.len() as f64
}

/// Average lexical overlap between adjacent paragraphs.
fn compute_paragraph_overlap(paragraphs: &[&str]) -> f64 {
    if paragraphs.len() < 2 {
        return 0.0;
    }

    let func_set: HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "is", "was", "are", "were", "be", "it", "this", "that",
    ].into_iter().collect();

    let mut total_overlap = 0.0;
    let mut pairs = 0;

    for i in 0..paragraphs.len() - 1 {
        let words_a: HashSet<String> = paragraphs[i].unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| !func_set.contains(w.as_str()) && w.len() > 2)
            .collect();
        let words_b: HashSet<String> = paragraphs[i + 1].unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| !func_set.contains(w.as_str()) && w.len() > 2)
            .collect();

        if !words_a.is_empty() && !words_b.is_empty() {
            let overlap = words_a.intersection(&words_b).count();
            let union = words_a.union(&words_b).count();
            total_overlap += overlap as f64 / union as f64;
            pairs += 1;
        }
    }

    if pairs > 0 { total_overlap / pairs as f64 } else { 0.0 }
}

/// Split text into sentences.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for i in 0..len {
        current.push(chars[i]);
        let is_terminal = chars[i] == '.' || chars[i] == '!' || chars[i] == '?';
        if is_terminal {
            let next_is_boundary = i + 1 >= len
                || chars[i + 1].is_whitespace()
                || chars[i + 1] == '"'
                || chars[i + 1] == ')';
            if next_is_boundary {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() && trimmed.split_whitespace().count() > 0 {
                    sentences.push(trimmed);
                }
                current.clear();
            }
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() && trimmed.split_whitespace().count() > 1 {
        sentences.push(trimmed);
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_long_text() {
        let text = "The ancient city walls stood tall against the morning sky. \
                     However, centuries of weathering had taken their toll. \
                     Furthermore, recent excavations revealed hidden chambers below. \
                     The archaeologists carefully documented each finding. \
                     Meanwhile, tourists continued to visit the popular site.\n\n\
                     Deep beneath the surface, pottery fragments told a story. \
                     Each piece represented a different era of civilization. \
                     Similarly, the metalwork showed evolving craftsmanship. \
                     The team catalogued thousands of artifacts during the season. \
                     Their discoveries would reshape our understanding of the past.\n\n\
                     Modern technology helped analyze the ancient materials. \
                     Carbon dating provided accurate timelines for each layer. \
                     Additionally, spectroscopy revealed the mineral compositions. \
                     The results confirmed several long-standing hypotheses. \
                     New questions emerged from the unexpected discoveries.\n\n\
                     Publication of the findings attracted international attention. \
                     Several universities requested access to the data. \
                     The team presented their work at major conferences. \
                     Funding for continued research was quickly secured. \
                     The next excavation season promised even more revelations.";
        let result = analyze(text);
        assert!(result.features_computed >= 5, "Expected at least 5 features, got {}", result.features_computed);
    }

    #[test]
    fn test_modal_patterns() {
        let words: Vec<String> = "she can go but she would not and they might stay"
            .split_whitespace()
            .map(|w| w.to_string())
            .collect();
        let (density, diversity) = compute_modal_patterns(&words);
        assert!(density > 0.0);
        assert!(diversity > 0.0);
    }

    #[test]
    fn test_punctuation_diversity() {
        let text = "Hello, world! How are you? Fine; thank you. Great — wonderful.";
        let entropy = compute_punctuation_diversity(text);
        assert!(entropy > 1.0, "Should have reasonable punctuation entropy: {entropy}");
    }
}
