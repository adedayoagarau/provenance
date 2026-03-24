//! Three-mode analysis architecture.
//!
//! The mode is auto-selected based on whether an author profile is provided:
//!
//! - **AiDetection**: No author profile. Pure AI detection using statistical features.
//!   This is what GPTZero does. Entry point for cold documents.
//!
//! - **Hybrid**: 1-2 baseline writing samples. Combines AI detection with a
//!   lightweight stylometric comparison. This is where Provenance beats everyone —
//!   GPTZero can't do this at all. Even a single writing sample significantly
//!   improves accuracy by distinguishing "this doesn't match the author" from
//!   "this was written by AI."
//!
//! - **AuthorshipVerification**: 3+ baseline samples. Full stylometric comparison
//!   using 740+ features. AI detection score becomes one input to the composite,
//!   weighted alongside authorship distance metrics.
//!
//! All three layers (AI detection, author comparison, process integrity) always run;
//! the mode determines the weights in the final composite score.

use serde::{Deserialize, Serialize};

use crate::analysis;
use crate::identity::{self, comparison::ComparisonResult, profile::AuthorProfile};
use crate::utils::errors::{ProvenanceError, Result};

use super::{DetectionResult, DetectionTier};

/// Analysis mode, auto-selected based on available evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisMode {
    /// No author profile — pure AI detection.
    AiDetection,
    /// 1-2 baseline samples — AI detection + lightweight stylometric comparison.
    Hybrid,
    /// 3+ baseline samples — full authorship verification with AI detection as one signal.
    AuthorshipVerification,
}

impl AnalysisMode {
    pub fn label(&self) -> &'static str {
        match self {
            AnalysisMode::AiDetection => "AI Detection (no baseline)",
            AnalysisMode::Hybrid => "Hybrid (AI detection + stylometric comparison)",
            AnalysisMode::AuthorshipVerification => "Authorship Verification (full profile)",
        }
    }

    /// Auto-select mode based on number of baseline samples.
    pub fn from_sample_count(count: usize) -> Self {
        match count {
            0 => AnalysisMode::AiDetection,
            1..=2 => AnalysisMode::Hybrid,
            _ => AnalysisMode::AuthorshipVerification,
        }
    }
}

/// Layer weights for each mode.
/// (ai_detection_weight, authorship_weight, process_integrity_weight)
struct ModeWeights {
    ai_detection: f64,
    authorship: f64,
}

impl ModeWeights {
    fn for_mode(mode: AnalysisMode) -> Self {
        match mode {
            AnalysisMode::AiDetection => ModeWeights {
                ai_detection: 1.0,
                authorship: 0.0,
            },
            AnalysisMode::Hybrid => ModeWeights {
                ai_detection: 0.55,
                authorship: 0.45,
            },
            AnalysisMode::AuthorshipVerification => ModeWeights {
                ai_detection: 0.25,
                authorship: 0.75,
            },
        }
    }
}

/// Extended result including authorship comparison when available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModeResult {
    /// Which mode was used.
    pub mode: AnalysisMode,
    /// Mode description.
    pub mode_label: String,
    /// The core AI detection result (always computed).
    pub detection: DetectionResult,
    /// Authorship comparison result (Hybrid and AuthorshipVerification modes only).
    pub authorship: Option<AuthorshipResult>,
    /// Combined composite score (0.0 = definitely this author + human, 1.0 = not this author + AI).
    pub composite_score: f64,
    /// Combined tier classification.
    pub composite_tier: DetectionTier,
    /// Combined tier label.
    pub composite_tier_label: String,
}

/// Authorship comparison portion of the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorshipResult {
    /// Author name from the profile.
    pub author_name: String,
    /// Number of baseline samples in the profile.
    pub sample_count: usize,
    /// Authorship confidence (0.0 = no match, 1.0 = strong match).
    pub match_confidence: f64,
    /// Raw comparison result from the identity module.
    pub comparison: ComparisonResult,
    /// How authorship comparison affected the overall interpretation.
    pub interpretation: String,
}

const MIN_WORDS: usize = 500;

/// Run multi-mode analysis.
///
/// If `profile` is `None`, runs in AiDetection mode.
/// If `profile` has 1-2 samples, runs in Hybrid mode.
/// If `profile` has 3+ samples, runs in AuthorshipVerification mode.
pub fn analyze_multimode(
    text: &str,
    profile: Option<&AuthorProfile>,
) -> Result<MultiModeResult> {
    let word_count = text.split_whitespace().count();
    if word_count < MIN_WORDS {
        return Err(ProvenanceError::InsufficientText {
            min_words: MIN_WORDS,
            actual_words: word_count,
        });
    }

    let mode = match profile {
        None => AnalysisMode::AiDetection,
        Some(p) => AnalysisMode::from_sample_count(p.sample_count),
    };

    // Always run the core detection pipeline
    let detection = super::detect(text)?;

    // Run authorship comparison if profile is available
    let authorship = if let Some(prof) = profile {
        let analysis_result = analysis::analyze_text(text)?;
        let comparison = identity::comparison::compare(&analysis_result, prof);
        let match_confidence = comparison.confidence.value;

        let interpretation = generate_authorship_interpretation(
            mode,
            match_confidence,
            detection.score.adjusted_score,
            &prof.name,
        );

        Some(AuthorshipResult {
            author_name: prof.name.clone(),
            sample_count: prof.sample_count,
            match_confidence,
            comparison,
            interpretation,
        })
    } else {
        None
    };

    // Compute composite score
    let weights = ModeWeights::for_mode(mode);
    let authorship_signal = authorship
        .as_ref()
        .map(|a| 1.0 - a.match_confidence) // Invert: low match = high suspicion
        .unwrap_or(0.0);

    let composite_score = (
        weights.ai_detection * detection.score.adjusted_score
        + weights.authorship * authorship_signal
    ).clamp(0.0, 1.0);

    let composite_tier = DetectionTier::from_score(composite_score);

    Ok(MultiModeResult {
        mode,
        mode_label: mode.label().to_string(),
        detection,
        authorship,
        composite_score,
        composite_tier,
        composite_tier_label: composite_tier.label().to_string(),
    })
}

/// Run multi-mode analysis from file paths.
pub fn analyze_multimode_file(
    file_path: &str,
    profile_path: Option<&str>,
) -> Result<MultiModeResult> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    let text = crate::extraction::extract_text(file_path)?;

    let profile = if let Some(pp) = profile_path {
        Some(identity::profile::load(pp)?)
    } else {
        None
    };

    analyze_multimode(&text, profile.as_ref())
}

/// Generate interpretation text for the authorship comparison.
fn generate_authorship_interpretation(
    _mode: AnalysisMode,
    match_confidence: f64,
    ai_score: f64,
    author_name: &str,
) -> String {
    match (match_confidence > 0.6, ai_score > 0.5) {
        (true, false) => format!(
            "The writing style matches {}'s profile (confidence: {:.0}%) and shows \
             human writing patterns. This is consistent with {} being the genuine author.",
            author_name, match_confidence * 100.0, author_name
        ),
        (true, true) => format!(
            "The writing style matches {}'s profile (confidence: {:.0}%), but AI \
             indicators are elevated. This could indicate that {} used AI assistance \
             while maintaining their personal style, or that an AI was prompted to \
             mimic {}'s writing style.",
            author_name, match_confidence * 100.0, author_name, author_name
        ),
        (false, false) => format!(
            "The writing style does not match {}'s profile (confidence: {:.0}%), \
             but the text shows human writing patterns. This may indicate a different \
             human author, or that {}'s writing style has changed significantly.",
            author_name, match_confidence * 100.0, author_name
        ),
        (false, true) => format!(
            "The writing style does not match {}'s profile (confidence: {:.0}%) and \
             shows AI-associated patterns. This is the strongest indicator that the \
             text was not authored by {} and may have been AI-generated.",
            author_name, match_confidence * 100.0, author_name
        ),
    }
}

/// Format multi-mode result as text.
pub fn format_text(result: &MultiModeResult) -> String {
    let mut out = String::new();

    out.push_str("═══════════════════════════════════════════════════════════\n");
    out.push_str("              PROVENANCE ANALYSIS REPORT\n");
    out.push_str("═══════════════════════════════════════════════════════════\n\n");

    out.push_str(&format!("  Mode:       {}\n", result.mode_label));
    out.push_str(&format!("  Composite:  {:.2} — {} ({})\n",
        result.composite_score,
        result.composite_tier_label,
        result.composite_tier.action()));

    if let Some(ref auth) = result.authorship {
        out.push_str(&format!("\n─── Authorship Comparison ───────────────────────────────\n\n"));
        out.push_str(&format!("  Author:     {}\n", auth.author_name));
        out.push_str(&format!("  Samples:    {}\n", auth.sample_count));
        let level_label = match auth.comparison.confidence.level {
            crate::identity::confidence::ConfidenceLevel::High => "High",
            crate::identity::confidence::ConfidenceLevel::Medium => "Medium",
            crate::identity::confidence::ConfidenceLevel::Low => "Low",
            crate::identity::confidence::ConfidenceLevel::Insufficient => "Insufficient",
        };
        out.push_str(&format!("  Match:      {:.0}% ({})\n",
            auth.match_confidence * 100.0, level_label));
        out.push_str(&format!("\n  {}\n", auth.interpretation));
    }

    out.push_str(&format!("\n─── AI Detection Layer ──────────────────────────────────\n\n"));
    out.push_str(&super::format_text(&result.detection));

    out
}

/// Format multi-mode result as JSON.
pub fn format_json(result: &MultiModeResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_selection() {
        assert_eq!(AnalysisMode::from_sample_count(0), AnalysisMode::AiDetection);
        assert_eq!(AnalysisMode::from_sample_count(1), AnalysisMode::Hybrid);
        assert_eq!(AnalysisMode::from_sample_count(2), AnalysisMode::Hybrid);
        assert_eq!(AnalysisMode::from_sample_count(3), AnalysisMode::AuthorshipVerification);
        assert_eq!(AnalysisMode::from_sample_count(10), AnalysisMode::AuthorshipVerification);
    }

    #[test]
    fn test_mode_weights() {
        let ai = ModeWeights::for_mode(AnalysisMode::AiDetection);
        assert_eq!(ai.ai_detection, 1.0);
        assert_eq!(ai.authorship, 0.0);

        let hybrid = ModeWeights::for_mode(AnalysisMode::Hybrid);
        assert!((hybrid.ai_detection + hybrid.authorship - 1.0).abs() < 0.01);

        let full = ModeWeights::for_mode(AnalysisMode::AuthorshipVerification);
        assert!(full.authorship > full.ai_detection);
    }

    #[test]
    fn test_interpretation_generation() {
        let interp = generate_authorship_interpretation(
            AnalysisMode::Hybrid, 0.8, 0.2, "Alice"
        );
        assert!(interp.contains("Alice"));
        assert!(interp.contains("genuine author"));
    }
}
