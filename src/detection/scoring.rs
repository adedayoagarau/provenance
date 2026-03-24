//! AI detection scoring pipeline.
//!
//! Combines all detection features into a single 0.0–1.0 score using
//! weighted sigmoid compositing with register normalization.
//!
//! Pipeline: Feature Extraction → Register Normalization → Weighted Sigmoid → 5-Tier Classification

use serde::{Deserialize, Serialize};

use crate::analysis::register::Register;

use super::{
    autocorrelation::AutocorrelationResult,
    burstiness::BurstinessResult,
    hedge_ratio::HedgeRatioResult,
    interaction::InteractionResult,
    pos_entropy::PosEntropyResult,
    zipf::ZipfResult,
};

/// Raw feature values extracted from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionFeatures {
    pub burstiness: Option<BurstinessResult>,
    pub zipf: Option<ZipfResult>,
    pub hedge_ratio: Option<HedgeRatioResult>,
    pub autocorrelation: Option<AutocorrelationResult>,
    pub pos_entropy: Option<PosEntropyResult>,
    pub interaction: Option<InteractionResult>,
}

/// Individual feature score with its contribution weight and explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureScore {
    pub name: String,
    pub raw_value: f64,
    pub normalized_score: f64,
    pub weight: f64,
    pub weighted_contribution: f64,
    pub explanation: String,
}

/// 5-tier classification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionTier {
    HumanAuthored,
    LikelyHuman,
    MixedUncertain,
    AiAssisted,
    AiGenerated,
}

impl DetectionTier {
    pub fn label(&self) -> &'static str {
        match self {
            DetectionTier::HumanAuthored => "HUMAN AUTHORED",
            DetectionTier::LikelyHuman => "LIKELY HUMAN",
            DetectionTier::MixedUncertain => "MIXED / UNCERTAIN",
            DetectionTier::AiAssisted => "AI ASSISTED",
            DetectionTier::AiGenerated => "AI GENERATED",
        }
    }

    pub fn action(&self) -> &'static str {
        match self {
            DetectionTier::HumanAuthored => "Accept",
            DetectionTier::LikelyHuman => "Accept with note",
            DetectionTier::MixedUncertain => "Human review required",
            DetectionTier::AiAssisted => "Investigation recommended",
            DetectionTier::AiGenerated => "Policy enforcement action",
        }
    }

    pub fn from_score(score: f64) -> Self {
        match score {
            s if s < 0.15 => DetectionTier::HumanAuthored,
            s if s < 0.35 => DetectionTier::LikelyHuman,
            s if s < 0.65 => DetectionTier::MixedUncertain,
            s if s < 0.85 => DetectionTier::AiAssisted,
            _ => DetectionTier::AiGenerated,
        }
    }
}

/// Final AI detection score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionScore {
    /// Composite AI likelihood score (0.0 = definitely human, 1.0 = definitely AI).
    pub score: f64,
    /// Score after register normalization.
    pub adjusted_score: f64,
    /// 5-tier classification.
    pub tier: DetectionTier,
    /// Tier label.
    pub tier_label: String,
    /// Recommended action.
    pub action: String,
    /// Register adjustment applied.
    pub register_adjustment: f64,
    /// Detected register.
    pub register_label: String,
    /// Number of features that contributed to the score.
    pub features_used: usize,
    /// Per-feature breakdown.
    pub feature_scores: Vec<FeatureScore>,
    /// Overall confidence (0.0–1.0) based on how many features fired.
    pub confidence: f64,
}

/// Feature weights (sum to ~1.0).
/// Weights reflect Cohen's d effect sizes from the research.
const WEIGHT_BURSTINESS: f64 = 0.22;      // d ≈ 1.1
const WEIGHT_ZIPF: f64 = 0.20;            // d ≈ 1.0
const WEIGHT_HEDGE_RATIO: f64 = 0.15;     // d ≈ 0.76
const WEIGHT_AUTOCORRELATION: f64 = 0.13; // d ≈ 0.65
const WEIGHT_POS_ENTROPY: f64 = 0.12;     // d ≈ 0.55
const WEIGHT_DIVERSITY_LENGTH: f64 = 0.09; // interaction feature
const WEIGHT_REPETITION_DELTA: f64 = 0.09; // interaction feature

/// Compute the composite detection score from raw features.
pub fn score(features: &DetectionFeatures, register: &Register) -> DetectionScore {
    let mut feature_scores: Vec<FeatureScore> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;

    // 1. Burstiness: lower coefficient → more AI-like
    if let Some(ref b) = features.burstiness {
        let raw = b.coefficient;
        // Human: 0.35-0.55, AI: 0.15-0.25
        // Map to AI score: sigmoid centered at 0.30 (midpoint)
        let normalized = sigmoid_score(raw, 0.30, -15.0);
        let weight = WEIGHT_BURSTINESS;
        weighted_sum += normalized * weight;
        total_weight += weight;
        feature_scores.push(FeatureScore {
            name: "Burstiness coefficient".to_string(),
            raw_value: raw,
            normalized_score: normalized,
            weight,
            weighted_contribution: normalized * weight,
            explanation: format!(
                "Word usage burstiness: {:.3} (human typical: 0.35–0.55, AI typical: 0.15–0.25)",
                raw
            ),
        });
    }

    // 2. Zipf slope deviation: smaller deviation → more AI-like
    if let Some(ref z) = features.zipf {
        let raw = z.deviation;
        // Human: ~0.15, AI: ~0.05
        // Map: small deviation = more AI
        let normalized = sigmoid_score(raw, 0.10, -20.0);
        let weight = WEIGHT_ZIPF;
        weighted_sum += normalized * weight;
        total_weight += weight;
        feature_scores.push(FeatureScore {
            name: "Zipf slope deviation".to_string(),
            raw_value: raw,
            normalized_score: normalized,
            weight,
            weighted_contribution: normalized * weight,
            explanation: format!(
                "Zipf law deviation: {:.4} (human typical: ±0.15, AI typical: ±0.05). Slope: {:.3}, R²: {:.3}",
                raw, z.slope, z.r_squared
            ),
        });
    }

    // 3. Hedge-to-intensifier ratio: lower → more AI-like
    if let Some(ref h) = features.hedge_ratio {
        let raw = h.ratio;
        // Human: 1.2-1.8, AI: 0.6-0.9
        // Map: low ratio = more AI
        let normalized = sigmoid_score(raw, 1.05, -3.0);
        let weight = WEIGHT_HEDGE_RATIO;
        weighted_sum += normalized * weight;
        total_weight += weight;
        feature_scores.push(FeatureScore {
            name: "Hedge-to-intensifier ratio".to_string(),
            raw_value: raw,
            normalized_score: normalized,
            weight,
            weighted_contribution: normalized * weight,
            explanation: format!(
                "Hedge/intensifier balance: {:.2} (human typical: 1.2–1.8, AI typical: 0.6–0.9). Hedges: {}, Intensifiers: {}",
                raw, h.hedge_count, h.intensifier_count
            ),
        });
    }

    // 4. Autocorrelation: higher positive → more AI-like
    if let Some(ref a) = features.autocorrelation {
        let raw = a.lag1_autocorrelation;
        // Human: -0.05 to +0.05, AI: 0.05 to 0.15
        // Map: high autocorrelation = more AI
        let normalized = sigmoid_score(raw, 0.05, 15.0);
        let weight = WEIGHT_AUTOCORRELATION;
        weighted_sum += normalized * weight;
        total_weight += weight;
        feature_scores.push(FeatureScore {
            name: "Sentence length autocorrelation".to_string(),
            raw_value: raw,
            normalized_score: normalized,
            weight,
            weighted_contribution: normalized * weight,
            explanation: format!(
                "Sentence rhythm predictability: {:.4} (human typical: near 0, AI typical: 0.05–0.15)",
                raw
            ),
        });
    }

    // 5. POS trigram entropy: lower → more AI-like
    if let Some(ref p) = features.pos_entropy {
        let raw = p.trigram_entropy;
        // Human: ~5.0 bits, AI: 4.3-4.6 bits
        // Map: low entropy = more AI
        let normalized = sigmoid_score(raw, 4.7, -3.0);
        let weight = WEIGHT_POS_ENTROPY;
        weighted_sum += normalized * weight;
        total_weight += weight;
        feature_scores.push(FeatureScore {
            name: "POS trigram entropy".to_string(),
            raw_value: raw,
            normalized_score: normalized,
            weight,
            weighted_contribution: normalized * weight,
            explanation: format!(
                "Syntactic variety: {:.2} bits (human typical: ~5.0, AI typical: 4.3–4.6). Unique trigrams: {}",
                raw, p.unique_trigrams
            ),
        });
    }

    // 6. Diversity × length correlation: higher positive → more AI-like
    if let Some(ref i) = features.interaction {
        let raw = i.diversity_length_correlation;
        // Human: ~0, AI: 0.2-0.4
        let normalized = sigmoid_score(raw, 0.10, 8.0);
        let weight = WEIGHT_DIVERSITY_LENGTH;
        weighted_sum += normalized * weight;
        total_weight += weight;
        feature_scores.push(FeatureScore {
            name: "Lexical diversity × sentence length correlation".to_string(),
            raw_value: raw,
            normalized_score: normalized,
            weight,
            weighted_contribution: normalized * weight,
            explanation: format!(
                "Diversity-length coupling: {:.3} (human typical: ~0, AI typical: 0.2–0.4)",
                raw
            ),
        });

        // 7. Repetition position delta: lower → more AI-like (less variation)
        let raw_delta = i.repetition_position_delta;
        // Human: higher variation, AI: lower variation
        let normalized_delta = sigmoid_score(raw_delta, 0.05, -10.0);
        let weight_delta = WEIGHT_REPETITION_DELTA;
        weighted_sum += normalized_delta * weight_delta;
        total_weight += weight_delta;
        feature_scores.push(FeatureScore {
            name: "Content word repetition × text position".to_string(),
            raw_value: raw_delta,
            normalized_score: normalized_delta,
            weight: weight_delta,
            weighted_contribution: normalized_delta * weight_delta,
            explanation: format!(
                "Repetition variation across text: {:.3} (human: high variation, AI: ~40% less). First half reuse: {:.3}, second half: {:.3}",
                raw_delta, i.first_half_reuse_rate, i.second_half_reuse_rate
            ),
        });
    }

    // Compute raw composite score
    let raw_score = if total_weight > 0.0 {
        (weighted_sum / total_weight).clamp(0.0, 1.0)
    } else {
        0.5 // Insufficient data → uncertain
    };

    // Register normalization
    let register_adjustment = register_adjustment(register);
    let adjusted_score = (raw_score + register_adjustment).clamp(0.0, 1.0);

    let tier = DetectionTier::from_score(adjusted_score);
    let features_used = feature_scores.len();

    // Confidence based on feature coverage (7 features max)
    let confidence = (features_used as f64 / 7.0).min(1.0);

    DetectionScore {
        score: raw_score,
        adjusted_score,
        tier,
        tier_label: tier.label().to_string(),
        action: tier.action().to_string(),
        register_adjustment,
        register_label: register.label().to_string(),
        features_used,
        feature_scores,
        confidence,
    }
}

/// Sigmoid scoring function.
///
/// Maps a raw feature value to [0, 1] using a logistic sigmoid.
/// `center`: the value where the sigmoid outputs 0.5
/// `steepness`: positive = higher raw → higher score; negative = inverse
fn sigmoid_score(value: f64, center: f64, steepness: f64) -> f64 {
    let z = steepness * (value - center);
    1.0 / (1.0 + (-z).exp())
}

/// Register-based score adjustment to reduce false positives.
///
/// Formal/structured registers naturally score higher on AI-likeness metrics,
/// so we subtract an offset to compensate.
fn register_adjustment(register: &Register) -> f64 {
    match register {
        Register::Academic(sub) => match sub {
            crate::analysis::register::AcademicSubtype::Legal => -0.20,
            _ => -0.15,
        },
        Register::Technical(_) => -0.10,
        Register::Journalistic(_) => -0.05,
        Register::Literary(_) => 0.05,
        Register::Casual(_) => 0.00,
        Register::Professional(_) => -0.08,
        Register::Educational(_) => -0.10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid_midpoint() {
        let s = sigmoid_score(0.30, 0.30, -15.0);
        assert!((s - 0.5).abs() < 0.01, "Sigmoid at center should be ~0.5: {s}");
    }

    #[test]
    fn test_tier_classification() {
        assert_eq!(DetectionTier::from_score(0.05), DetectionTier::HumanAuthored);
        assert_eq!(DetectionTier::from_score(0.25), DetectionTier::LikelyHuman);
        assert_eq!(DetectionTier::from_score(0.50), DetectionTier::MixedUncertain);
        assert_eq!(DetectionTier::from_score(0.75), DetectionTier::AiAssisted);
        assert_eq!(DetectionTier::from_score(0.95), DetectionTier::AiGenerated);
    }

    #[test]
    fn test_register_adjustments() {
        use crate::analysis::register::AcademicSubtype;
        let legal = Register::Academic(AcademicSubtype::Legal);
        assert_eq!(register_adjustment(&legal), -0.20);

        use crate::analysis::register::CasualSubtype;
        let casual = Register::Casual(CasualSubtype::BlogPost);
        assert_eq!(register_adjustment(&casual), 0.00);
    }

    #[test]
    fn test_score_with_no_features() {
        let features = DetectionFeatures {
            burstiness: None,
            zipf: None,
            hedge_ratio: None,
            autocorrelation: None,
            pos_entropy: None,
            interaction: None,
        };
        let register = Register::Casual(crate::analysis::register::CasualSubtype::BlogPost);
        let result = score(&features, &register);
        assert_eq!(result.score, 0.5); // Uncertain when no features
        assert_eq!(result.features_used, 0);
    }
}
