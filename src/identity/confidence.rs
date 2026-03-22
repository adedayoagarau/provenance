use serde::{Deserialize, Serialize};

/// Confidence score with contextual factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub value: f64,
    pub level: ConfidenceLevel,
    pub word_count_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
    Insufficient,
}

/// Compute a confidence score, adjusted for text length.
pub fn compute(raw_score: f64, word_count: usize) -> ConfidenceScore {
    // Longer texts provide more reliable signals
    let word_count_factor = match word_count {
        0..=99 => 0.5,
        100..=499 => 0.7,
        500..=1999 => 0.9,
        _ => 1.0,
    };

    let adjusted = raw_score * word_count_factor;

    let level = match adjusted {
        v if v >= 0.85 => ConfidenceLevel::High,
        v if v >= 0.65 => ConfidenceLevel::Medium,
        v if v >= 0.40 => ConfidenceLevel::Low,
        _ => ConfidenceLevel::Insufficient,
    };

    ConfidenceScore {
        value: adjusted,
        level,
        word_count_factor,
    }
}
