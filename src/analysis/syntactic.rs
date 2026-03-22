/// Syntactic analysis profile for a text.
#[derive(Debug, Clone)]
pub struct SyntacticProfile {
    pub total_sentences: usize,
    pub avg_sentence_length: f64,
    pub sentence_length_variance: f64,
    pub avg_words_per_sentence: f64,
}

/// Perform syntactic analysis on text.
pub fn analyze(text: &str) -> SyntacticProfile {
    let sentences: Vec<&str> = text
        .split(|c| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let total_sentences = sentences.len();

    let sentence_word_counts: Vec<usize> = sentences
        .iter()
        .map(|s| s.split_whitespace().count())
        .collect();

    let total_words: usize = sentence_word_counts.iter().sum();
    let avg_words_per_sentence = if total_sentences > 0 {
        total_words as f64 / total_sentences as f64
    } else {
        0.0
    };

    let sentence_lengths: Vec<usize> = sentences.iter().map(|s| s.len()).collect();
    let avg_sentence_length = if total_sentences > 0 {
        sentence_lengths.iter().sum::<usize>() as f64 / total_sentences as f64
    } else {
        0.0
    };

    let sentence_length_variance = if total_sentences > 1 {
        let mean = avg_sentence_length;
        let sum_sq: f64 = sentence_lengths
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

    SyntacticProfile {
        total_sentences,
        avg_sentence_length,
        sentence_length_variance,
        avg_words_per_sentence,
    }
}
