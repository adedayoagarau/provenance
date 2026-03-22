use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// Function word frequency profile.
///
/// Function words (determiners, prepositions, conjunctions, pronouns, auxiliaries)
/// are the most reliable stylometric feature class. They are used unconsciously
/// and are extremely difficult to deliberately alter.
///
/// References:
/// - Mosteller & Wallace (1964): Federalist Papers attribution
/// - Burrows (2002): Delta method uses MFW (mostly function words)
/// - Argamon et al. (2007): Function words as style markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionWordProfile {
    /// Relative frequency of each function word (per 1000 words)
    pub frequencies: HashMap<String, f64>,
    /// Total function words found / total words
    pub function_word_ratio: f64,
    /// Number of distinct function words used
    pub function_word_diversity: usize,
    /// Total words in the analyzed text
    pub total_words: usize,
}

/// Analyze function word usage in text.
pub fn analyze(text: &str) -> FunctionWordProfile {
    let words: Vec<String> = text
        .unicode_words()
        .map(|w| w.to_lowercase())
        .collect();

    let total_words = words.len();
    if total_words == 0 {
        return FunctionWordProfile {
            frequencies: HashMap::new(),
            function_word_ratio: 0.0,
            function_word_diversity: 0,
            total_words: 0,
        };
    }

    let fw_set = function_word_set();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total_fw = 0usize;

    for word in &words {
        if fw_set.contains(word.as_str()) {
            *counts.entry(word.clone()).or_insert(0) += 1;
            total_fw += 1;
        }
    }

    // Normalize to per-1000-words
    let scale = 1000.0 / total_words as f64;
    let frequencies: HashMap<String, f64> = counts
        .iter()
        .map(|(word, &count)| (word.clone(), count as f64 * scale))
        .collect();

    FunctionWordProfile {
        function_word_diversity: frequencies.len(),
        function_word_ratio: total_fw as f64 / total_words as f64,
        frequencies,
        total_words,
    }
}

/// Comprehensive English function word list.
///
/// Sourced from the union of:
/// - Mosteller & Wallace word list
/// - Burrows' Delta MFW lists
/// - PAN competition standard lists
/// - Koppel, Schler & Argamon (2009) function word set
///
/// Categories: determiners, prepositions, conjunctions, pronouns,
/// auxiliary/modal verbs, adverbs, quantifiers, and other grammatical words.
fn function_word_set() -> std::collections::HashSet<&'static str> {
    [
        // Determiners & Articles
        "the", "a", "an", "this", "that", "these", "those",
        "my", "your", "his", "her", "its", "our", "their",
        "some", "any", "no", "every", "each", "all", "both",
        "few", "several", "many", "much", "more", "most",
        "other", "another", "such", "what", "which", "whose",
        // Prepositions
        "of", "in", "to", "for", "with", "on", "at", "from",
        "by", "about", "as", "into", "through", "during", "before",
        "after", "above", "below", "between", "under", "over",
        "against", "without", "within", "along", "following",
        "across", "behind", "beyond", "plus", "except", "up",
        "down", "off", "since", "until", "upon", "around",
        "among", "throughout", "despite", "towards", "toward",
        "near", "beside", "besides", "per", "via",
        // Conjunctions
        "and", "but", "or", "nor", "for", "yet", "so",
        "because", "although", "though", "while", "if", "when",
        "where", "unless", "since", "whereas", "whether",
        "once", "than", "that", "till", "until", "whenever",
        "wherever", "either", "neither", "both",
        // Personal Pronouns
        "i", "me", "my", "mine", "myself",
        "you", "your", "yours", "yourself", "yourselves",
        "he", "him", "his", "himself",
        "she", "her", "hers", "herself",
        "it", "its", "itself",
        "we", "us", "our", "ours", "ourselves",
        "they", "them", "their", "theirs", "themselves",
        // Relative & Interrogative Pronouns
        "who", "whom", "whose", "which", "that",
        "what", "whoever", "whomever", "whatever", "whichever",
        // Demonstrative & Indefinite Pronouns
        "one", "ones", "anyone", "everyone", "someone", "no one",
        "anybody", "everybody", "somebody", "nobody",
        "anything", "everything", "something", "nothing",
        "somewhere", "anywhere", "everywhere", "nowhere",
        // Auxiliary & Modal Verbs
        "be", "am", "is", "are", "was", "were", "been", "being",
        "have", "has", "had", "having",
        "do", "does", "did", "doing",
        "will", "would", "shall", "should",
        "can", "could", "may", "might", "must",
        "need", "dare", "ought",
        // Common Adverbs (grammatical, not content)
        "not", "never", "always", "often", "also", "just",
        "only", "still", "already", "even", "now", "then",
        "here", "there", "very", "too", "quite", "rather",
        "almost", "enough", "perhaps", "maybe", "probably",
        "certainly", "definitely", "surely", "indeed",
        "really", "actually", "simply", "merely", "hardly",
        "scarcely", "barely", "nearly", "approximately",
        "ever", "yet", "again", "soon", "already",
        "away", "back", "well", "else", "otherwise",
        // Existential & other
        "there", "here",
        // Negation
        "not", "no", "none", "neither", "nor", "never",
        // Miscellaneous function words
        "how", "why", "where", "when",
        "however", "therefore", "thus", "hence",
        "moreover", "furthermore", "nevertheless", "nonetheless",
        "meanwhile", "otherwise", "instead", "accordingly",
        "consequently", "subsequently",
        "able", "like", "get", "got", "go", "went", "gone",
        "come", "came", "make", "made", "take", "took", "taken",
        "give", "gave", "given", "let", "seem", "keep", "kept",
    ].into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_word_analysis() {
        let text = "The quick brown fox jumps over the lazy dog and the cat sleeps on the mat";
        let profile = analyze(text);

        assert!(profile.total_words > 0);
        assert!(profile.function_word_ratio > 0.0);
        assert!(profile.function_word_ratio <= 1.0);
        // "the" appears 4 times in 15 words = 266.67 per 1000
        assert!(profile.frequencies.contains_key("the"));
        assert!(profile.frequencies["the"] > 200.0);
    }

    #[test]
    fn test_empty_text() {
        let profile = analyze("");
        assert_eq!(profile.total_words, 0);
        assert_eq!(profile.function_word_ratio, 0.0);
    }
}
