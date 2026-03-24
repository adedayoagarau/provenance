//! Tier 2 structural features for AI detection.
//!
//! Six moderate-effect-size features (d ≈ 0.4–0.6) that strengthen the detection
//! signal in ensemble with Tier 1 features:
//!
//! 1. Passive voice clustering — AI distributes passive voice uniformly;
//!    humans cluster it in specific sections
//! 2. Transition word overuse — AI over-uses connectives relative to human norms
//! 3. Paragraph length uniformity — AI produces more uniform paragraph lengths
//! 4. Sentence opening diversity — AI starts sentences less diversely
//! 5. Nested clause frequency — AI produces fewer deeply nested clauses
//! 6. Comma splice avoidance — AI almost never produces comma splices

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

/// Combined Tier 2 feature results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier2Result {
    /// Passive voice clustering coefficient (variance of per-section passive ratios).
    /// Lower = more uniform = more AI-like.
    pub passive_clustering: Option<f64>,
    /// Transition word density (per 1000 words).
    /// Higher than normal = more AI-like.
    pub transition_density: Option<f64>,
    /// Coefficient of variation of paragraph word counts.
    /// Lower CV = more uniform = more AI-like.
    pub paragraph_length_cv: Option<f64>,
    /// Sentence opening diversity (unique openers / total sentences).
    /// Lower = less diverse = more AI-like.
    pub sentence_opening_diversity: Option<f64>,
    /// Estimated nested clause ratio (sentences with 2+ commas / total).
    /// Lower = fewer nested clauses = more AI-like.
    pub nested_clause_ratio: Option<f64>,
    /// Comma splice ratio (comma splices detected / total sentences).
    /// Near zero = more AI-like.
    pub comma_splice_ratio: Option<f64>,
    /// Number of features successfully computed.
    pub features_computed: usize,
}

const TRANSITION_WORDS: &[&str] = &[
    "however", "therefore", "furthermore", "moreover", "additionally",
    "consequently", "nevertheless", "meanwhile", "otherwise", "instead",
    "accordingly", "subsequently", "similarly", "likewise", "conversely",
    "nonetheless", "hence", "thus", "finally", "firstly", "secondly",
    "thirdly", "in addition", "on the other hand", "in contrast",
    "as a result", "for example", "for instance", "in particular",
    "in fact", "of course", "after all", "in other words",
    "that is", "namely", "specifically", "notably", "importantly",
    "surprisingly", "unfortunately", "fortunately", "interestingly",
    "clearly", "obviously", "apparently", "evidently", "certainly",
    "indeed", "undoubtedly", "naturally", "admittedly",
];

/// Analyze Tier 2 structural features.
pub fn analyze(text: &str) -> Tier2Result {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();
    let word_count = words.len();

    let sentences = split_sentences(text);
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let mut features_computed = 0;

    // 1. Passive voice clustering
    let passive_clustering = if sentences.len() >= 20 {
        let v = compute_passive_clustering(&sentences);
        features_computed += 1;
        Some(v)
    } else {
        None
    };

    // 2. Transition word density
    let transition_density = if word_count >= 200 {
        let lower_text = text.to_lowercase();
        let count = TRANSITION_WORDS.iter()
            .map(|t| lower_text.matches(t).count())
            .sum::<usize>();
        let density = count as f64 / word_count as f64 * 1000.0;
        features_computed += 1;
        Some(density)
    } else {
        None
    };

    // 3. Paragraph length uniformity
    let paragraph_length_cv = if paragraphs.len() >= 4 {
        let lens: Vec<f64> = paragraphs.iter()
            .map(|p| p.split_whitespace().count() as f64)
            .collect();
        let mean = lens.iter().sum::<f64>() / lens.len() as f64;
        if mean > 0.0 {
            let var = lens.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / (lens.len() - 1) as f64;
            features_computed += 1;
            Some(var.sqrt() / mean)
        } else {
            None
        }
    } else {
        None
    };

    // 4. Sentence opening diversity
    let sentence_opening_diversity = if sentences.len() >= 10 {
        let openings: Vec<String> = sentences.iter()
            .filter_map(|s| {
                s.split_whitespace().next().map(|w|
                    w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect()
                )
            })
            .collect();
        let unique: HashSet<&String> = openings.iter().collect();
        let diversity = unique.len() as f64 / openings.len() as f64;
        features_computed += 1;
        Some(diversity)
    } else {
        None
    };

    // 5. Nested clause ratio (sentences with 2+ commas as proxy)
    let nested_clause_ratio = if sentences.len() >= 10 {
        let nested = sentences.iter()
            .filter(|s| s.chars().filter(|&c| c == ',').count() >= 2)
            .count();
        features_computed += 1;
        Some(nested as f64 / sentences.len() as f64)
    } else {
        None
    };

    // 6. Comma splice ratio
    let comma_splice_ratio = if sentences.len() >= 10 {
        let splices = detect_comma_splices(text);
        features_computed += 1;
        Some(splices as f64 / sentences.len() as f64)
    } else {
        None
    };

    Tier2Result {
        passive_clustering,
        transition_density,
        paragraph_length_cv,
        sentence_opening_diversity,
        nested_clause_ratio,
        comma_splice_ratio,
        features_computed,
    }
}

/// Compute passive voice clustering: variance of per-section passive ratios.
fn compute_passive_clustering(sentences: &[String]) -> f64 {
    let chunk_size = sentences.len() / 4;
    if chunk_size < 3 {
        return 0.0;
    }

    let be_forms = ["was", "were", "is", "are", "been", "being"];
    let mut section_ratios = Vec::new();

    for chunk in sentences.chunks(chunk_size) {
        let mut passive = 0;
        for s in chunk {
            let words: Vec<String> = s.split_whitespace()
                .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect())
                .collect();
            for i in 0..words.len().saturating_sub(1) {
                if be_forms.contains(&words[i].as_str()) {
                    if let Some(next) = words.get(i + 1) {
                        if next.ends_with("ed") || next.ends_with("en") {
                            passive += 1;
                            break;
                        }
                    }
                }
            }
        }
        section_ratios.push(passive as f64 / chunk.len() as f64);
    }

    if section_ratios.len() < 2 {
        return 0.0;
    }

    let mean = section_ratios.iter().sum::<f64>() / section_ratios.len() as f64;
    let var = section_ratios.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
        / (section_ratios.len() - 1) as f64;
    var
}

/// Detect comma splices: comma between two independent clauses.
fn detect_comma_splices(text: &str) -> usize {
    let mut count = 0;
    let pronouns: HashSet<&str> = [
        "i", "you", "he", "she", "it", "we", "they", "this", "that",
        "there", "here",
    ].into_iter().collect();

    // Simple heuristic: comma followed by pronoun + verb pattern
    for part in text.split(',') {
        let trimmed = part.trim();
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() >= 3 {
            let first = words[0].to_lowercase();
            let clean: String = first.chars().filter(|c| c.is_alphabetic()).collect();
            if pronouns.contains(clean.as_str()) {
                count += 1;
            }
        }
    }

    // Subtract 1 for the first segment (not a splice)
    (count as usize).saturating_sub(1)
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
        let text = "The quick brown fox jumped over the lazy dog. \
                     However, the dog was not impressed by this display. \
                     Meanwhile, the cat watched from the windowsill with interest. \
                     The bird flew overhead, singing its morning song loudly. \
                     Furthermore, the garden was blooming with colorful flowers.\n\n\
                     She walked to the store and bought some groceries. \
                     The weather was beautiful that afternoon. \
                     He decided to go for a long run in the park. \
                     They met for coffee later that evening. \
                     It was a productive and enjoyable day overall.\n\n\
                     The children played in the yard while dinner cooked. \
                     An old cat sat on the fence watching them play. \
                     The sun was setting behind the mountains slowly. \
                     A gentle breeze carried the scent of jasmine flowers. \
                     The evening was peaceful and calm in the neighborhood.\n\n\
                     We gathered around the table for a family meal. \
                     The food was delicious and everyone ate heartily. \
                     Stories were shared and laughter filled the room. \
                     The youngest child told a joke that made everyone smile. \
                     It was exactly the kind of evening they all needed.";
        let result = analyze(text);
        assert!(result.features_computed >= 4);
        assert!(result.sentence_opening_diversity.is_some());
    }

    #[test]
    fn test_short_text() {
        let result = analyze("Too short.");
        assert_eq!(result.features_computed, 0);
    }

    #[test]
    fn test_comma_splices() {
        let count = detect_comma_splices("The sun was hot, she went inside. He was tired, he went to bed.");
        assert!(count > 0);
    }
}
