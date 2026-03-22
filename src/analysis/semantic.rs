use std::collections::HashMap;

/// Semantic analysis profile for a text.
#[derive(Debug, Clone)]
pub struct SemanticProfile {
    pub paragraph_count: usize,
    pub topic_keywords: Vec<(String, usize)>,
    pub coherence_score: f64,
}

/// Perform semantic analysis on text.
pub fn analyze(text: &str) -> SemanticProfile {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let paragraph_count = paragraphs.len();

    // Extract top keywords by frequency (excluding common stop words)
    let stop_words = stop_words();
    let mut word_freq: HashMap<String, usize> = HashMap::new();
    for word in text.split_whitespace() {
        let cleaned = word.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        if cleaned.len() > 2 && !stop_words.contains(cleaned.as_str()) {
            *word_freq.entry(cleaned).or_insert(0) += 1;
        }
    }

    let mut topic_keywords: Vec<(String, usize)> = word_freq.into_iter().collect();
    topic_keywords.sort_by(|a, b| b.1.cmp(&a.1));
    topic_keywords.truncate(20);

    // Simple coherence score based on paragraph connectivity
    let coherence_score = if paragraph_count > 1 { 0.5 } else { 1.0 };

    SemanticProfile {
        paragraph_count,
        topic_keywords,
        coherence_score,
    }
}

fn stop_words() -> std::collections::HashSet<&'static str> {
    [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "shall", "can", "this", "that",
        "these", "those", "it", "its", "not", "no", "nor", "as", "if", "than",
    ].into_iter().collect()
}
