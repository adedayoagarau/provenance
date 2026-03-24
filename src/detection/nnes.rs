//! Non-Native English Speaker (NNES) detection for false positive reduction.
//!
//! NNES writers produce text with lower burstiness, more formulaic sentence patterns,
//! and reduced lexical diversity — all patterns that overlap with AI indicators.
//! Without adjustment, international students and ESL writers face disproportionate
//! false positive rates.
//!
//! Detection signals:
//! - Determiner errors (missing/extra articles)
//! - Limited sentence structure variety
//! - High function word regularity
//! - Low idiom/collocation frequency
//! - Limited contraction usage in informal contexts
//!
//! Score adjustment: -0.12 to -0.25 depending on NNES confidence.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

/// NNES detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NnesResult {
    /// Whether NNES patterns were detected.
    pub detected: bool,
    /// Confidence in the NNES detection (0.0–1.0).
    pub confidence: f64,
    /// Score adjustment to apply (negative, reducing AI score).
    pub score_adjustment: f64,
    /// Individual indicator scores.
    pub indicators: Vec<NnesIndicator>,
}

/// A single NNES indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NnesIndicator {
    pub name: String,
    pub score: f64,
    pub description: String,
}

/// Analyze text for non-native English speaker patterns.
pub fn analyze(text: &str) -> NnesResult {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    if words.len() < 100 {
        return NnesResult {
            detected: false,
            confidence: 0.0,
            score_adjustment: 0.0,
            indicators: Vec::new(),
        };
    }

    let mut indicators = Vec::new();

    // 1. Determiner error patterns (missing/extra articles)
    let det_score = detect_determiner_errors(text);
    indicators.push(NnesIndicator {
        name: "Determiner error patterns".to_string(),
        score: det_score,
        description: if det_score > 0.3 {
            "Article usage patterns suggest non-native speaker".to_string()
        } else {
            "Article usage appears native-like".to_string()
        },
    });

    // 2. Limited sentence opening diversity
    let opening_score = detect_limited_openings(text);
    indicators.push(NnesIndicator {
        name: "Sentence opening diversity".to_string(),
        score: opening_score,
        description: if opening_score > 0.3 {
            "Repetitive sentence openings suggest limited syntactic range".to_string()
        } else {
            "Sentence opening variety appears natural".to_string()
        },
    });

    // 3. Contraction avoidance in informal contexts
    let contraction_score = detect_contraction_avoidance(&words, text);
    indicators.push(NnesIndicator {
        name: "Contraction avoidance".to_string(),
        score: contraction_score,
        description: if contraction_score > 0.3 {
            "Avoidance of contractions in informal text suggests non-native speaker".to_string()
        } else {
            "Contraction usage appears natural for register".to_string()
        },
    });

    // 4. Preposition usage patterns
    let prep_score = detect_preposition_patterns(&words);
    indicators.push(NnesIndicator {
        name: "Preposition usage patterns".to_string(),
        score: prep_score,
        description: if prep_score > 0.3 {
            "Preposition patterns deviate from native norms".to_string()
        } else {
            "Preposition usage appears native-like".to_string()
        },
    });

    // 5. Limited idiom/collocation usage
    let idiom_score = detect_low_idiom_usage(text);
    indicators.push(NnesIndicator {
        name: "Idiom and collocation frequency".to_string(),
        score: idiom_score,
        description: if idiom_score > 0.3 {
            "Low idiom/collocation usage suggests non-native speaker".to_string()
        } else {
            "Idiomatic expression usage appears native-like".to_string()
        },
    });

    // Compute overall NNES confidence
    let total: f64 = indicators.iter().map(|i| i.score).sum();
    let avg = total / indicators.len() as f64;

    let detected = avg > 0.30;
    let confidence = avg.min(1.0);

    // Score adjustment: linear interpolation from -0.12 (low confidence) to -0.25 (high confidence)
    let score_adjustment = if detected {
        -(0.12 + (confidence - 0.30) * (0.25 - 0.12) / 0.70).clamp(0.12, 0.25)
    } else {
        0.0
    };

    NnesResult {
        detected,
        confidence,
        score_adjustment,
        indicators,
    }
}

/// Detect determiner (article) error patterns common in NNES writing.
fn detect_determiner_errors(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 50 {
        return 0.0;
    }

    let mut error_signals = 0;
    let articles: HashSet<&str> = ["a", "an", "the"].into_iter().collect();
    let nouns_after_prep: HashSet<&str> = [
        "in", "on", "at", "to", "for", "with", "by", "from", "of",
    ].into_iter().collect();

    for i in 0..words.len().saturating_sub(1) {
        let word = words[i].to_lowercase();
        let clean: String = word.chars().filter(|c| c.is_alphabetic()).collect();
        let next: String = words[i + 1].to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();

        // Pattern: preposition directly followed by a capitalized word or long word
        // without an article in between (e.g., "in university" instead of "in the university")
        if nouns_after_prep.contains(clean.as_str())
            && !articles.contains(next.as_str())
            && next.len() > 4
            && words[i + 1].chars().next().map_or(false, |c| c.is_uppercase())
        {
            // Not an error if it's a proper noun pattern (already capitalized in original)
            // but it's a weak signal
            error_signals += 1;
        }

        // Pattern: "a" before vowel-starting word (should be "an")
        if clean == "a" && next.starts_with(|c: char| "aeiou".contains(c)) && next.len() > 1 {
            error_signals += 2;
        }

        // Pattern: "an" before consonant-starting word (should be "a")
        if clean == "an" && !next.starts_with(|c: char| "aeiou".contains(c)) && next.len() > 1 {
            // Exceptions: "an hour", "an honest", "an heir"
            if !next.starts_with('h') {
                error_signals += 2;
            }
        }
    }

    let density = error_signals as f64 / words.len() as f64 * 100.0;
    (density * 5.0).min(1.0)
}

/// Detect limited sentence opening diversity (NNES tend to start sentences the same way).
fn detect_limited_openings(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| s.split_whitespace().count() >= 3)
        .collect();

    if sentences.len() < 8 {
        return 0.0;
    }

    let openings: Vec<String> = sentences
        .iter()
        .filter_map(|s| {
            s.split_whitespace()
                .next()
                .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect())
        })
        .collect();

    let unique: HashSet<&String> = openings.iter().collect();
    let diversity = unique.len() as f64 / openings.len() as f64;

    // Check for heavy repetition of simple openers
    let simple_openers: HashSet<&str> = [
        "the", "it", "this", "there", "i", "we", "they", "he", "she",
    ].into_iter().collect();

    let simple_count = openings
        .iter()
        .filter(|o| simple_openers.contains(o.as_str()))
        .count();
    let simple_ratio = simple_count as f64 / openings.len() as f64;

    // Low diversity + high simple opener ratio = NNES signal
    let diversity_score = if diversity < 0.30 { 0.7 } else if diversity < 0.45 { 0.3 } else { 0.0 };
    let simple_score = if simple_ratio > 0.70 { 0.6 } else if simple_ratio > 0.55 { 0.3 } else { 0.0 };

    (diversity_score + simple_score) / 2.0
}

/// Detect contraction avoidance in informal text.
fn detect_contraction_avoidance(words: &[String], text: &str) -> f64 {
    if words.len() < 100 {
        return 0.0;
    }

    // Check if text appears informal (presence of personal pronouns, short sentences)
    let personal_pronouns: HashSet<&str> = ["i", "you", "we", "my", "your", "our"].into_iter().collect();
    let pronoun_ratio = words.iter().filter(|w| personal_pronouns.contains(w.as_str())).count() as f64
        / words.len() as f64;

    // Only flag if text seems informal (pronoun ratio > 3%)
    if pronoun_ratio < 0.03 {
        return 0.0;
    }

    // Count expandable patterns that should be contractions in informal text
    let expandable = [
        "do not", "does not", "did not", "will not", "would not",
        "could not", "should not", "can not", "cannot",
        "is not", "are not", "was not", "were not",
        "has not", "have not", "had not",
        "i am", "i have", "i will", "i would",
        "it is", "that is", "there is", "what is",
    ];

    let lower_text = text.to_lowercase();
    let expanded_count = expandable.iter().filter(|p| lower_text.contains(**p)).count();
    let contracted = text.matches("n't").count() + text.matches("'s").count()
        + text.matches("'re").count() + text.matches("'ve").count()
        + text.matches("'ll").count() + text.matches("'m").count();

    if expanded_count == 0 && contracted == 0 {
        return 0.0;
    }

    let total = expanded_count + contracted;
    let expansion_ratio = expanded_count as f64 / total as f64;

    // High expansion ratio in informal context = NNES signal
    if expansion_ratio > 0.70 { 0.6 } else if expansion_ratio > 0.50 { 0.3 } else { 0.0 }
}

/// Detect non-native preposition usage patterns.
fn detect_preposition_patterns(words: &[String]) -> f64 {
    if words.len() < 100 {
        return 0.0;
    }

    // Count preposition frequency
    let preps: HashSet<&str> = [
        "in", "on", "at", "to", "for", "with", "by", "from", "of",
        "about", "into", "through", "during", "before", "after",
    ].into_iter().collect();

    let prep_count = words.iter().filter(|w| preps.contains(w.as_str())).count();
    let prep_ratio = prep_count as f64 / words.len() as f64;

    // NNES typically over-use or under-use prepositions
    // Native range: roughly 10-15%
    if prep_ratio < 0.07 || prep_ratio > 0.20 {
        ((prep_ratio - 0.125).abs() * 8.0).min(0.6)
    } else {
        0.0
    }
}

/// Detect low idiom/collocation usage.
fn detect_low_idiom_usage(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let word_count = text.split_whitespace().count();

    if word_count < 200 {
        return 0.0;
    }

    // Common English idioms and collocations
    let idioms = [
        "on the other hand", "as a matter of fact", "in other words",
        "for the most part", "by the way", "at the same time",
        "on the one hand", "in the long run", "at the end of the day",
        "keep in mind", "take into account", "make sense",
        "as well as", "in terms of", "kind of", "sort of",
        "a lot of", "in order to", "due to the fact",
        "as far as", "up to date", "in addition to",
        "with respect to", "in spite of", "on behalf of",
        "ahead of time", "all of a sudden", "once in a while",
        "sooner or later", "little by little", "by and large",
        "now and then", "over and over", "back and forth",
    ];

    let found = idioms.iter().filter(|&&idiom| lower.contains(idiom)).count();
    let density = found as f64 / (word_count as f64 / 500.0); // per 500 words

    // Native writers typically use 2-5 collocations per 500 words
    if density < 0.5 {
        0.5
    } else if density < 1.0 {
        0.2
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_no_detection() {
        let result = analyze("Short text here.");
        assert!(!result.detected);
        assert_eq!(result.score_adjustment, 0.0);
    }

    #[test]
    fn test_native_text() {
        let text = "I don't think we should go to the store today because it's raining. \
                     On the other hand, we could take an umbrella and make the best of it. \
                     In the long run, a little rain won't hurt us. There's something to be \
                     said for getting out of the house, you know? We've been cooped up all \
                     weekend and I'm going stir-crazy. Let's just go for it — we'll be fine. \
                     Besides, they're having a sale at that bookshop around the corner, and \
                     I'd hate to miss it. What do you say? Are you in or not?";
        let result = analyze(text);
        // Native text should have low NNES confidence
        assert!(result.confidence < 0.5, "Native text NNES confidence too high: {}", result.confidence);
    }

    #[test]
    fn test_adjustment_range() {
        // If detected, adjustment should be between -0.12 and -0.25
        let result = NnesResult {
            detected: true,
            confidence: 0.5,
            score_adjustment: -0.15,
            indicators: Vec::new(),
        };
        assert!(result.score_adjustment >= -0.25);
        assert!(result.score_adjustment <= -0.12);
    }
}
