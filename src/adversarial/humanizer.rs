//! Humanizer resistance — detects patterns left by AI humanizer tools.
//!
//! Humanizer tools (Undetectable.ai, QuillBot, StealthGPT, etc.) modify AI-generated
//! text to evade detectors. They leave characteristic artifacts that can be detected
//! through statistical analysis of the modification patterns.

use serde::{Deserialize, Serialize};

/// Known humanizer tools and their characteristic patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HumanizerTool {
    UndetectableAi,
    QuillBot,
    StealthGPT,
    WriteHuman,
    HideMyAI,
    Smodin,
    WordAi,
    SpinRewriter,
    Generic,
}

impl HumanizerTool {
    pub fn label(&self) -> &'static str {
        match self {
            HumanizerTool::UndetectableAi => "Undetectable.ai",
            HumanizerTool::QuillBot => "QuillBot",
            HumanizerTool::StealthGPT => "StealthGPT",
            HumanizerTool::WriteHuman => "WriteHuman",
            HumanizerTool::HideMyAI => "HideMyAI",
            HumanizerTool::Smodin => "Smodin",
            HumanizerTool::WordAi => "WordAi",
            HumanizerTool::SpinRewriter => "SpinRewriter",
            HumanizerTool::Generic => "Unknown Humanizer",
        }
    }
}

/// Result of humanizer detection analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanizerDetectionResult {
    /// Whether humanizer artifacts were detected.
    pub detected: bool,
    /// Confidence in the detection (0.0-1.0).
    pub confidence: f64,
    /// Which humanizer tool is suspected (if detected).
    pub suspected_tool: Option<HumanizerTool>,
    /// Individual indicator scores.
    pub indicators: Vec<HumanizerIndicator>,
    /// Summary assessment.
    pub assessment: String,
}

/// A single humanizer detection indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanizerIndicator {
    /// Name of the indicator.
    pub name: String,
    /// Score (0.0-1.0, higher = more suspicious).
    pub score: f64,
    /// Description of what was found.
    pub description: String,
    /// Weight of this indicator in the overall score.
    pub weight: f64,
}

/// Analyze text for humanizer tool artifacts.
///
/// Humanizer tools leave statistical fingerprints:
/// 1. Synonym diversity anomalies — unusual synonym choices for common words
/// 2. Sentence structure homogeneity — humanizers often produce uniform structure
/// 3. Readability inconsistency — sections vary unnaturally in complexity
/// 4. Function word distribution shifts — humanizers alter function word patterns
/// 5. Collocation disruption — unusual word pair co-occurrences
/// 6. Register inconsistency — mixed formality within passages
pub fn detect_humanizer(text: &str) -> HumanizerDetectionResult {
    if text.len() < 200 {
        return HumanizerDetectionResult {
            detected: false,
            confidence: 0.0,
            suspected_tool: None,
            indicators: Vec::new(),
            assessment: "Insufficient text for humanizer detection (need 200+ characters)".into(),
        };
    }

    let mut indicators = Vec::new();

    // Indicator 1: Synonym substitution patterns
    let synonym_score = detect_synonym_anomalies(text);
    indicators.push(HumanizerIndicator {
        name: "Synonym substitution patterns".into(),
        score: synonym_score,
        description: if synonym_score > 0.5 {
            "Unusual word choices suggest systematic synonym replacement".into()
        } else {
            "Word choice patterns appear natural".into()
        },
        weight: 0.20,
    });

    // Indicator 2: Sentence structure homogeneity
    let structure_score = detect_structure_homogeneity(text);
    indicators.push(HumanizerIndicator {
        name: "Sentence structure homogeneity".into(),
        score: structure_score,
        description: if structure_score > 0.5 {
            "Sentence structures are unusually uniform, suggesting mechanical rewriting".into()
        } else {
            "Sentence structure variety appears natural".into()
        },
        weight: 0.15,
    });

    // Indicator 3: Readability inconsistency
    let readability_score = detect_readability_inconsistency(text);
    indicators.push(HumanizerIndicator {
        name: "Readability inconsistency".into(),
        score: readability_score,
        description: if readability_score > 0.5 {
            "Readability varies unnaturally between passages, suggesting selective rewriting".into()
        } else {
            "Readability is consistent throughout".into()
        },
        weight: 0.15,
    });

    // Indicator 4: Function word distribution shift
    let function_word_score = detect_function_word_shift(text);
    indicators.push(HumanizerIndicator {
        name: "Function word distribution shift".into(),
        score: function_word_score,
        description: if function_word_score > 0.5 {
            "Function word usage deviates from natural patterns".into()
        } else {
            "Function word distribution appears natural".into()
        },
        weight: 0.20,
    });

    // Indicator 5: Collocation disruption
    let collocation_score = detect_collocation_disruption(text);
    indicators.push(HumanizerIndicator {
        name: "Collocation disruption".into(),
        score: collocation_score,
        description: if collocation_score > 0.5 {
            "Unusual word co-occurrence patterns suggest automated paraphrasing".into()
        } else {
            "Word co-occurrence patterns are typical".into()
        },
        weight: 0.15,
    });

    // Indicator 6: Punctuation normalization
    let punctuation_score = detect_punctuation_normalization(text);
    indicators.push(HumanizerIndicator {
        name: "Punctuation normalization".into(),
        score: punctuation_score,
        description: if punctuation_score > 0.5 {
            "Punctuation patterns are unusually regular, suggesting tool processing".into()
        } else {
            "Punctuation variety appears natural".into()
        },
        weight: 0.15,
    });

    // Compute weighted score
    let total_weight: f64 = indicators.iter().map(|i| i.weight).sum();
    let weighted_sum: f64 = indicators.iter().map(|i| i.score * i.weight).sum();
    let overall = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.0
    };

    let detected = overall > 0.45;
    let confidence = overall.min(1.0);

    let suspected_tool = if detected {
        // Heuristic tool identification based on indicator patterns
        identify_tool(&indicators)
    } else {
        None
    };

    let assessment = if !detected {
        "No significant humanizer artifacts detected in this text".into()
    } else if confidence > 0.7 {
        format!(
            "Strong indicators of humanizer tool processing detected{}. Multiple statistical anomalies suggest systematic text modification.",
            suspected_tool.map(|t| format!(" (likely {})", t.label())).unwrap_or_default()
        )
    } else {
        "Some indicators of possible humanizer processing, but evidence is not conclusive. Additional signals recommended for definitive assessment.".into()
    };

    HumanizerDetectionResult {
        detected,
        confidence,
        suspected_tool,
        indicators,
        assessment,
    }
}

/// Detect unusual synonym choices that suggest automated substitution.
fn detect_synonym_anomalies(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 50 {
        return 0.0;
    }

    // Check for uncommon synonyms of very common words.
    // Humanizers often replace "big" → "substantial", "good" → "commendable", etc.
    let uncommon_synonyms = [
        "utilize", "commence", "endeavor", "facilitate", "ascertain",
        "moreover", "furthermore", "consequently", "notwithstanding",
        "plethora", "myriad", "multitude", "substantive", "commendable",
        "paramount", "pivotal", "delve", "tapestry", "landscape",
        "leverage", "robust", "holistic", "synergy", "paradigm",
    ];

    let word_count = words.len() as f64;
    let uncommon_count = words
        .iter()
        .filter(|w| {
            let lower = w.to_lowercase();
            let clean: String = lower.chars().filter(|c| c.is_alphabetic()).collect();
            uncommon_synonyms.contains(&clean.as_str())
        })
        .count() as f64;

    let density = uncommon_count / word_count * 100.0;

    // Threshold: >2% uncommon synonym density is suspicious
    (density / 4.0).min(1.0)
}

/// Detect unusually uniform sentence structure.
fn detect_structure_homogeneity(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| s.split_whitespace().count() >= 3)
        .collect();

    if sentences.len() < 5 {
        return 0.0;
    }

    // Check variance in sentence opening patterns
    let openings: Vec<&str> = sentences
        .iter()
        .filter_map(|s| s.split_whitespace().next())
        .collect();

    let unique_openings = openings.iter().collect::<std::collections::HashSet<_>>().len();
    let opening_diversity = unique_openings as f64 / openings.len() as f64;

    // Check sentence length variance
    let lengths: Vec<f64> = sentences
        .iter()
        .map(|s| s.split_whitespace().count() as f64)
        .collect();

    let mean_len = lengths.iter().sum::<f64>() / lengths.len() as f64;
    let variance = lengths.iter().map(|l| (l - mean_len).powi(2)).sum::<f64>() / lengths.len() as f64;
    let cv = if mean_len > 0.0 { variance.sqrt() / mean_len } else { 0.0 };

    // Low CV + low opening diversity = suspicious
    let length_score = if cv < 0.20 { 0.8 } else if cv < 0.30 { 0.4 } else { 0.1 };
    let opening_score = if opening_diversity < 0.30 { 0.7 } else if opening_diversity < 0.50 { 0.3 } else { 0.1 };

    (length_score + opening_score) / 2.0
}

/// Detect unnatural readability variation between passages.
fn detect_readability_inconsistency(text: &str) -> f64 {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .filter(|p| p.split_whitespace().count() >= 20)
        .collect();

    if paragraphs.len() < 3 {
        return 0.0;
    }

    // Compute average word length per paragraph as a simple readability proxy
    let avg_word_lengths: Vec<f64> = paragraphs
        .iter()
        .map(|p| {
            let words: Vec<&str> = p.split_whitespace().collect();
            if words.is_empty() {
                return 0.0;
            }
            words.iter().map(|w| w.len() as f64).sum::<f64>() / words.len() as f64
        })
        .collect();

    let mean = avg_word_lengths.iter().sum::<f64>() / avg_word_lengths.len() as f64;
    let variance = avg_word_lengths
        .iter()
        .map(|l| (l - mean).powi(2))
        .sum::<f64>()
        / avg_word_lengths.len() as f64;
    let cv = if mean > 0.0 { variance.sqrt() / mean } else { 0.0 };

    // Very high or very low CV is suspicious
    // Humanizers sometimes create sections with wildly different complexity
    if cv > 0.15 { (cv * 3.0).min(1.0) } else { 0.0 }
}

/// Detect shifts in function word distribution.
fn detect_function_word_shift(text: &str) -> f64 {
    let words: Vec<String> = text
        .split_whitespace()
        .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect())
        .filter(|w: &String| !w.is_empty())
        .collect();

    if words.len() < 100 {
        return 0.0;
    }

    // Expected function word ratio in natural English text: ~40-55%
    let function_words = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "to", "of", "in", "for",
        "on", "with", "at", "by", "from", "as", "into", "through", "during",
        "before", "after", "above", "below", "and", "but", "or", "nor",
        "not", "so", "yet", "both", "either", "neither", "each", "every",
        "all", "any", "few", "more", "most", "other", "some", "such",
        "no", "only", "same", "than", "too", "very", "just", "also",
        "i", "me", "my", "we", "us", "our", "you", "your", "he", "him",
        "his", "she", "her", "it", "its", "they", "them", "their",
        "this", "that", "these", "those", "which", "who", "whom",
    ];

    let fw_count = words
        .iter()
        .filter(|w| function_words.contains(&w.as_str()))
        .count();

    let fw_ratio = fw_count as f64 / words.len() as f64;

    // Unusual function word ratio (too high or too low)
    if fw_ratio < 0.35 {
        // Humanizers often reduce function words by substituting more complex phrases
        ((0.35 - fw_ratio) * 5.0).min(1.0)
    } else if fw_ratio > 0.60 {
        ((fw_ratio - 0.60) * 5.0).min(1.0)
    } else {
        0.0
    }
}

/// Detect unusual word co-occurrence patterns.
fn detect_collocation_disruption(text: &str) -> f64 {
    let words: Vec<String> = text
        .split_whitespace()
        .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect())
        .filter(|w: &String| !w.is_empty())
        .collect();

    if words.len() < 50 {
        return 0.0;
    }

    // Check for disrupted common collocations
    // Humanizers often break natural word pairs
    let common_collocations = [
        ("make", "decision"), ("pay", "attention"), ("take", "place"),
        ("come", "true"), ("keep", "mind"), ("give", "rise"),
        ("play", "role"), ("strong", "evidence"), ("wide", "range"),
        ("highly", "likely"), ("closely", "related"), ("deeply", "rooted"),
    ];

    let bigrams: Vec<(&str, &str)> = words
        .windows(2)
        .map(|w| (w[0].as_str(), w[1].as_str()))
        .collect();

    // Check if any disrupted collocations appear (e.g., "render decision" instead of "make decision")
    // This is a simplified heuristic
    let expected_found = common_collocations
        .iter()
        .filter(|(a, b)| bigrams.iter().any(|(x, y)| x == a && y == b))
        .count();

    // Low collocation presence in formal text is slightly suspicious
    if words.len() > 500 && expected_found == 0 {
        0.3
    } else {
        0.0
    }
}

/// Detect overly regular punctuation patterns.
fn detect_punctuation_normalization(text: &str) -> f64 {
    let sentences: Vec<&str> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .collect();

    if sentences.len() < 5 {
        return 0.0;
    }

    // Check comma frequency per sentence
    let comma_counts: Vec<usize> = sentences
        .iter()
        .map(|s| s.chars().filter(|&c| c == ',').count())
        .collect();

    let mean_commas = comma_counts.iter().sum::<usize>() as f64 / comma_counts.len() as f64;
    let variance = comma_counts
        .iter()
        .map(|&c| (c as f64 - mean_commas).powi(2))
        .sum::<f64>()
        / comma_counts.len() as f64;

    let cv = if mean_commas > 0.0 {
        variance.sqrt() / mean_commas
    } else {
        0.0
    };

    // Very low variance in comma usage suggests mechanical processing
    if cv < 0.3 && sentences.len() > 8 {
        0.5
    } else {
        0.0
    }
}

/// Attempt to identify which humanizer tool was used based on indicator patterns.
fn identify_tool(indicators: &[HumanizerIndicator]) -> Option<HumanizerTool> {
    let synonym_score = indicators
        .iter()
        .find(|i| i.name.contains("Synonym"))
        .map(|i| i.score)
        .unwrap_or(0.0);

    let structure_score = indicators
        .iter()
        .find(|i| i.name.contains("structure"))
        .map(|i| i.score)
        .unwrap_or(0.0);

    // Very high synonym + low structure = likely QuillBot
    if synonym_score > 0.7 && structure_score < 0.3 {
        return Some(HumanizerTool::QuillBot);
    }

    // High across all indicators = likely Undetectable.ai (aggressive)
    if synonym_score > 0.5 && structure_score > 0.5 {
        return Some(HumanizerTool::UndetectableAi);
    }

    Some(HumanizerTool::Generic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_humanizer_short_text() {
        let result = detect_humanizer("Too short.");
        assert!(!result.detected);
    }

    #[test]
    fn test_detect_humanizer_natural_text() {
        let text = "The quick brown fox jumped over the lazy dog. She went to the store \
                    to buy some groceries. After that, she came home and made dinner for \
                    her family. The children played in the yard while she cooked. It was \
                    a beautiful evening with the sun setting over the hills. They ate \
                    together as a family, talking about their day at school and work. \
                    The youngest child told a funny story about what happened at recess. \
                    Everyone laughed and enjoyed the meal together. After dinner, they \
                    watched a movie before bedtime. It was a normal, quiet evening at home.";
        let result = detect_humanizer(text);
        // Natural text should not trigger strong detection
        assert!(result.confidence < 0.7, "Natural text confidence too high: {}", result.confidence);
    }

    #[test]
    fn test_detect_humanizer_suspicious_text() {
        // Text with heavy uncommon synonym usage (humanizer-like)
        let text = "The paramount objective of this endeavor is to facilitate the \
                    comprehensive understanding of multifaceted paradigms. We must \
                    leverage our robust methodologies to ascertain the substantive \
                    implications of these pivotal findings. Furthermore, it is \
                    commendable that our holistic approach has yielded a plethora \
                    of actionable insights. The myriad challenges notwithstanding, \
                    we have commenced a thorough investigation into the underlying \
                    mechanisms. This endeavor will utilize cutting-edge analytical \
                    frameworks to delve into the intricate tapestry of contemporary \
                    research landscapes. Our findings substantiate the paramount \
                    importance of leveraging interdisciplinary synergies.";
        let result = detect_humanizer(text);
        assert!(
            result.indicators.iter().any(|i| i.name.contains("Synonym") && i.score > 0.3),
            "Should detect synonym anomalies in suspicious text"
        );
    }

    #[test]
    fn test_humanizer_tool_labels() {
        assert_eq!(HumanizerTool::UndetectableAi.label(), "Undetectable.ai");
        assert_eq!(HumanizerTool::QuillBot.label(), "QuillBot");
        assert_eq!(HumanizerTool::Generic.label(), "Unknown Humanizer");
    }
}
