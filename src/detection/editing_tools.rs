//! Editing tool normalization for false positive reduction.
//!
//! Grammarly, ProWritingAid, and similar tools smooth out natural writing
//! burstiness, normalize sentence lengths, and improve structural consistency.
//! These modifications overlap with AI writing patterns and cause false positives.
//!
//! When editing tool patterns are detected, apply -0.15 score adjustment.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// Editing tool detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditingToolResult {
    /// Whether editing tool patterns were detected.
    pub detected: bool,
    /// Confidence in detection (0.0–1.0).
    pub confidence: f64,
    /// Score adjustment to apply (negative).
    pub score_adjustment: f64,
    /// Specific patterns found.
    pub indicators: Vec<EditingToolIndicator>,
}

/// A single editing tool indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditingToolIndicator {
    pub name: String,
    pub score: f64,
    pub description: String,
}

/// Analyze text for editing tool patterns.
pub fn analyze(text: &str) -> EditingToolResult {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    if words.len() < 200 {
        return EditingToolResult {
            detected: false,
            confidence: 0.0,
            score_adjustment: 0.0,
            indicators: Vec::new(),
        };
    }

    let mut indicators = Vec::new();

    // 1. Perfect comma usage (Grammarly signature)
    let comma_score = detect_perfect_commas(text);
    indicators.push(EditingToolIndicator {
        name: "Comma perfection".to_string(),
        score: comma_score,
        description: if comma_score > 0.3 {
            "Unusually perfect comma placement suggests grammar tool usage".to_string()
        } else {
            "Comma usage appears natural".to_string()
        },
    });

    // 2. No sentence fragments (natural writing has some)
    let fragment_score = detect_no_fragments(text);
    indicators.push(EditingToolIndicator {
        name: "Fragment absence".to_string(),
        score: fragment_score,
        description: if fragment_score > 0.3 {
            "Complete absence of sentence fragments in long text suggests editing tool".to_string()
        } else {
            "Sentence completeness appears natural".to_string()
        },
    });

    // 3. Transition word over-regularization
    let transition_score = detect_transition_regularity(text);
    indicators.push(EditingToolIndicator {
        name: "Transition word regularity".to_string(),
        score: transition_score,
        description: if transition_score > 0.3 {
            "Overly regular transition word placement suggests automated editing".to_string()
        } else {
            "Transition word usage appears natural".to_string()
        },
    });

    // 4. Passive voice elimination (ProWritingAid aggressively removes passive)
    let passive_score = detect_passive_elimination(text);
    indicators.push(EditingToolIndicator {
        name: "Passive voice absence".to_string(),
        score: passive_score,
        description: if passive_score > 0.3 {
            "Near-zero passive voice in a long text suggests automated rewriting".to_string()
        } else {
            "Passive voice usage appears natural".to_string()
        },
    });

    // 5. Consistent sentence length (tools normalize this)
    let length_score = detect_normalized_lengths(text);
    indicators.push(EditingToolIndicator {
        name: "Sentence length normalization".to_string(),
        score: length_score,
        description: if length_score > 0.3 {
            "Unusually uniform sentence lengths suggest editing tool normalization".to_string()
        } else {
            "Sentence length variation appears natural".to_string()
        },
    });

    let total: f64 = indicators.iter().map(|i| i.score).sum();
    let avg = total / indicators.len() as f64;

    let detected = avg > 0.30;
    let confidence = avg.min(1.0);

    let score_adjustment = if detected { -0.15 } else { 0.0 };

    EditingToolResult {
        detected,
        confidence,
        score_adjustment,
        indicators,
    }
}

/// Detect unusually perfect comma placement.
fn detect_perfect_commas(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| s.split_whitespace().count() >= 5)
        .collect();

    if sentences.len() < 10 {
        return 0.0;
    }

    // Check that commas appear after introductory phrases consistently
    let introductory_words = [
        "however", "therefore", "furthermore", "moreover", "additionally",
        "consequently", "nevertheless", "meanwhile", "unfortunately",
        "fortunately", "interestingly", "surprisingly", "importantly",
        "finally", "firstly", "secondly", "subsequently", "similarly",
    ];

    let mut intro_with_comma = 0;
    let mut intro_total = 0;

    for sentence in &sentences {
        let words: Vec<&str> = sentence.split_whitespace().collect();
        if let Some(first) = words.first() {
            let clean: String = first.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
            if introductory_words.contains(&clean.as_str()) {
                intro_total += 1;
                // Check if comma follows
                if words.len() > 1 && words[0].ends_with(',') || (words.len() > 1 && words[1].starts_with(',')) {
                    intro_with_comma += 1;
                } else if sentence.contains(',') && sentence.find(',').unwrap_or(usize::MAX) < 20 {
                    intro_with_comma += 1;
                }
            }
        }
    }

    if intro_total >= 3 && intro_with_comma == intro_total {
        0.5 // Perfect introductory comma usage
    } else {
        0.0
    }
}

/// Detect absence of sentence fragments.
fn detect_no_fragments(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .collect();

    if sentences.len() < 15 {
        return 0.0;
    }

    let short_sentences = sentences
        .iter()
        .filter(|s| s.split_whitespace().count() <= 3)
        .count();

    let fragment_ratio = short_sentences as f64 / sentences.len() as f64;

    // Natural writing typically has some very short sentences/fragments
    // Complete absence in long text is suspicious
    if fragment_ratio < 0.02 && sentences.len() > 20 {
        0.4
    } else {
        0.0
    }
}

/// Detect overly regular transition word placement.
fn detect_transition_regularity(text: &str) -> f64 {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .filter(|p| p.split_whitespace().count() >= 10)
        .collect();

    if paragraphs.len() < 4 {
        return 0.0;
    }

    let transition_words = [
        "however", "therefore", "furthermore", "moreover", "additionally",
        "consequently", "nevertheless", "in addition", "on the other hand",
        "in contrast", "similarly", "likewise", "as a result", "for example",
        "in particular", "specifically", "notably", "importantly",
    ];

    // Check if transitions appear at the start of every paragraph
    let para_starts_with_transition = paragraphs
        .iter()
        .filter(|p| {
            let lower = p.to_lowercase();
            transition_words.iter().any(|t| lower.trim_start().starts_with(t))
        })
        .count();

    let ratio = para_starts_with_transition as f64 / paragraphs.len() as f64;

    if ratio > 0.60 { 0.5 } else if ratio > 0.40 { 0.2 } else { 0.0 }
}

/// Detect near-zero passive voice (tools eliminate it).
fn detect_passive_elimination(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| s.split_whitespace().count() >= 5)
        .collect();

    if sentences.len() < 15 {
        return 0.0;
    }

    let be_forms = ["was", "were", "is", "are", "been", "being"];
    let mut passive_count = 0;

    for sentence in &sentences {
        let words: Vec<String> = sentence
            .split_whitespace()
            .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect())
            .collect();

        for i in 0..words.len().saturating_sub(1) {
            if be_forms.contains(&words[i].as_str()) {
                if let Some(next) = words.get(i + 1) {
                    if next.ends_with("ed") || next.ends_with("en") {
                        passive_count += 1;
                        break;
                    }
                }
            }
        }
    }

    let passive_ratio = passive_count as f64 / sentences.len() as f64;

    // Natural English has ~10-15% passive voice. Near-zero is suspicious.
    if passive_ratio < 0.02 && sentences.len() > 20 {
        0.4
    } else {
        0.0
    }
}

/// Detect unnaturally uniform sentence lengths.
fn detect_normalized_lengths(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| s.split_whitespace().count() >= 3)
        .collect();

    if sentences.len() < 10 {
        return 0.0;
    }

    let lengths: Vec<f64> = sentences
        .iter()
        .map(|s| s.split_whitespace().count() as f64)
        .collect();

    let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
    let variance = lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lengths.len() as f64;
    let cv = if mean > 0.0 { variance.sqrt() / mean } else { 0.0 };

    // Natural writing has CV ~0.45-0.65 for sentence lengths.
    // Tool-edited text tends toward CV < 0.30.
    if cv < 0.25 { 0.5 } else if cv < 0.30 { 0.2 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text() {
        let result = analyze("Short text.");
        assert!(!result.detected);
    }

    #[test]
    fn test_natural_text() {
        let text = "I went to the store. Bought some milk. The weather was terrible — \
                     rain and wind all day long. However, I managed to get there before \
                     closing time. Really? Yes, barely. \
                     The roads were slippery and my car was almost stuck in the mud. \
                     But I persevered. Sometimes you just have to push through. \
                     That's what my grandmother always said. She was a wise woman. \
                     Very practical. Never one to complain about anything really.";
        let result = analyze(text);
        // Natural writing should not trigger strong detection
        assert!(result.confidence < 0.5, "Natural text editing tool confidence too high: {}", result.confidence);
    }
}
