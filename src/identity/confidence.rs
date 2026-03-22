use serde::{Deserialize, Serialize};

/// Confidence score with contextual factors.
///
/// Combines raw similarity score with text-length penalty and
/// feature availability factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// Final adjusted confidence (0.0 - 1.0)
    pub value: f64,
    /// Classification level
    pub level: ConfidenceLevel,
    /// Multiplier applied for text length
    pub word_count_factor: f64,
    /// How many features contributed to the score
    pub feature_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
    Insufficient,
}

/// Compute a confidence score from a raw similarity value (0.0 - 1.0)
/// adjusted for text length.
pub fn compute(raw_score: f64, word_count: usize) -> ConfidenceScore {
    let word_count_factor = text_length_factor(word_count);
    let adjusted = raw_score * word_count_factor;

    let level = classify(adjusted);

    ConfidenceScore {
        value: adjusted,
        level,
        word_count_factor,
        feature_count: 0,
    }
}

/// Compute confidence from a Cosine Delta score.
///
/// Delta scores are distances (lower = more similar), so we convert
/// to a similarity score using a sigmoid-like calibration.
pub fn from_delta(cosine_delta: f64, word_count: usize, feature_count: usize) -> ConfidenceScore {
    // Convert delta distance to probability via sigmoid calibration.
    // Calibration parameters based on typical Delta score ranges:
    // - Same author: cosine delta typically 0.0 - 0.3
    // - Different author: cosine delta typically 0.3 - 1.0+
    // Sigmoid center at 0.3, steepness 10
    let raw_similarity = 1.0 / (1.0 + (10.0 * (cosine_delta - 0.3)).exp());

    let word_count_factor = text_length_factor(word_count);

    // Feature availability factor: more features = more reliable
    let feature_factor = match feature_count {
        0..=10 => 0.6,
        11..=50 => 0.8,
        51..=200 => 0.9,
        _ => 1.0,
    };

    let adjusted = raw_similarity * word_count_factor * feature_factor;
    let level = classify(adjusted);

    ConfidenceScore {
        value: adjusted,
        level,
        word_count_factor,
        feature_count,
    }
}

/// Text length penalty factor.
///
/// Short texts produce unreliable feature vectors. Below 500 words,
/// confidence is progressively penalized.
fn text_length_factor(word_count: usize) -> f64 {
    match word_count {
        0..=99 => 0.4,
        100..=249 => 0.6,
        250..=499 => 0.75,
        500..=999 => 0.9,
        1000..=1999 => 0.95,
        _ => 1.0,
    }
}

fn classify(score: f64) -> ConfidenceLevel {
    match score {
        v if v >= 0.85 => ConfidenceLevel::High,
        v if v >= 0.65 => ConfidenceLevel::Medium,
        v if v >= 0.40 => ConfidenceLevel::Low,
        _ => ConfidenceLevel::Insufficient,
    }
}
