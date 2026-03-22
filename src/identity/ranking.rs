//! Multi-candidate authorship ranking.
//!
//! When multiple author profiles are available, rank them by how well they
//! match the query document. This is the comparative verification scenario:
//! "Which of these N candidates most likely wrote this?"

use serde::{Deserialize, Serialize};

use super::comparison::{self, ComparisonResult};
use super::explainability::{self, ExplainedDecision};
use super::profile::AuthorProfile;
use crate::analysis::AnalysisResult;

/// Result of ranking multiple candidates against a query document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingResult {
    /// Candidates ranked by match quality (best first).
    pub rankings: Vec<CandidateRanking>,
    /// Margin between best and second-best candidate (0 = tied, >0.2 = clear winner).
    pub top_margin: f64,
    /// Whether the ranking is definitive (top_margin > threshold).
    pub is_definitive: bool,
    /// Human-readable summary of the ranking outcome.
    pub summary: String,
}

/// A single candidate's ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRanking {
    /// Rank position (1 = best match).
    pub rank: usize,
    /// Candidate name.
    pub author_name: String,
    /// Detailed comparison result.
    pub comparison: ComparisonResult,
    /// Feature importance explanation.
    pub explanation: ExplainedDecision,
    /// Relative score (0-1, normalized across candidates).
    pub relative_score: f64,
}

/// Rank multiple author candidates against a query document.
pub fn rank_candidates(
    analysis: &AnalysisResult,
    candidates: &[AuthorProfile],
) -> RankingResult {
    if candidates.is_empty() {
        return RankingResult {
            rankings: Vec::new(),
            top_margin: 0.0,
            is_definitive: false,
            summary: "No candidates provided for ranking.".into(),
        };
    }

    // Compare against each candidate
    let mut comparisons: Vec<(ComparisonResult, ExplainedDecision)> = candidates
        .iter()
        .map(|profile| {
            let comp = comparison::compare(analysis, profile);
            let explained = explainability::explain(&comp, analysis);
            (comp, explained)
        })
        .collect();

    // Sort by confidence (highest first)
    comparisons.sort_by(|a, b| {
        b.0.confidence
            .value
            .partial_cmp(&a.0.confidence.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Compute relative scores (normalized so best = 1.0)
    let max_confidence = comparisons
        .first()
        .map(|(c, _)| c.confidence.value)
        .unwrap_or(0.0);

    let rankings: Vec<CandidateRanking> = comparisons
        .into_iter()
        .enumerate()
        .map(|(i, (comp, explained))| {
            let relative_score = if max_confidence > 0.0 {
                comp.confidence.value / max_confidence
            } else {
                0.0
            };
            CandidateRanking {
                rank: i + 1,
                author_name: comp.author_name.clone(),
                comparison: comp,
                explanation: explained,
                relative_score,
            }
        })
        .collect();

    // Compute margin
    let top_margin = if rankings.len() >= 2 {
        rankings[0].comparison.confidence.value - rankings[1].comparison.confidence.value
    } else {
        rankings
            .first()
            .map(|r| r.comparison.confidence.value)
            .unwrap_or(0.0)
    };

    let is_definitive = top_margin > 0.15;

    let summary = generate_ranking_summary(&rankings, top_margin, is_definitive);

    RankingResult {
        rankings,
        top_margin,
        is_definitive,
        summary,
    }
}

fn generate_ranking_summary(
    rankings: &[CandidateRanking],
    top_margin: f64,
    is_definitive: bool,
) -> String {
    match rankings.len() {
        0 => "No candidates to rank.".into(),
        1 => {
            let best = &rankings[0];
            format!(
                "Single candidate '{}' scored {:.1}% confidence.",
                best.author_name,
                best.comparison.confidence.value * 100.0,
            )
        }
        _ => {
            let best = &rankings[0];
            let second = &rankings[1];
            if is_definitive {
                format!(
                    "'{}' is the strongest match ({:.1}% confidence), with a {:.1} percentage point \
                     margin over '{}' ({:.1}%). The ranking is considered definitive.",
                    best.author_name,
                    best.comparison.confidence.value * 100.0,
                    top_margin * 100.0,
                    second.author_name,
                    second.comparison.confidence.value * 100.0,
                )
            } else {
                format!(
                    "'{}' ({:.1}%) and '{}' ({:.1}%) are closely ranked (margin: {:.1} pp). \
                     Additional evidence may be needed to distinguish between them.",
                    best.author_name,
                    best.comparison.confidence.value * 100.0,
                    second.author_name,
                    second.comparison.confidence.value * 100.0,
                    top_margin * 100.0,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis;

    fn sample_text() -> &'static str {
        "The quick brown fox jumps over the lazy dog. This is a sample text with \
         multiple sentences. It contains various words of different lengths. \
         Furthermore, the discourse markers are present. I believe that hedging \
         language is important. However, we should also consider the alternative \
         perspectives on this matter."
    }

    fn make_profile(name: &str, text: &str) -> AuthorProfile {
        let analysis = analysis::analyze_text(text).unwrap();
        crate::identity::profile::build(name, &[analysis]).unwrap()
    }

    #[test]
    fn test_ranking_single_candidate() {
        let text = sample_text();
        let analysis = analysis::analyze_text(text).unwrap();
        let profile = make_profile("Alice", text);

        let result = rank_candidates(&analysis, &[profile]);
        assert_eq!(result.rankings.len(), 1);
        assert_eq!(result.rankings[0].rank, 1);
        assert_eq!(result.rankings[0].author_name, "Alice");
        // Self-comparison should yield non-trivial confidence
        // (text-length penalty reduces score significantly for short samples)
        assert!(
            result.rankings[0].comparison.confidence.value > 0.2,
            "Self-comparison should yield non-trivial confidence, got {:.3}",
            result.rankings[0].comparison.confidence.value
        );
    }

    #[test]
    fn test_ranking_multiple_candidates() {
        let query = sample_text();
        let analysis = analysis::analyze_text(query).unwrap();

        let alice = make_profile("Alice", query);
        let bob = make_profile(
            "Bob",
            "yo what up. this is totally different writing style. \
             like, super casual and stuff. can't believe it. \
             we're just chilling here. isn't that cool? yeah!",
        );

        let result = rank_candidates(&analysis, &[alice, bob]);
        assert_eq!(result.rankings.len(), 2);
        // Alice (same text) should rank higher
        assert_eq!(result.rankings[0].author_name, "Alice");
        assert!(result.top_margin > 0.0);
    }

    #[test]
    fn test_ranking_empty_candidates() {
        let analysis = analysis::analyze_text(sample_text()).unwrap();
        let result = rank_candidates(&analysis, &[]);
        assert!(result.rankings.is_empty());
        assert!(!result.is_definitive);
    }

    #[test]
    fn test_ranking_relative_scores() {
        let query = sample_text();
        let analysis = analysis::analyze_text(query).unwrap();
        let alice = make_profile("Alice", query);
        let bob = make_profile("Bob", "Short. Different. Style.");

        let result = rank_candidates(&analysis, &[alice, bob]);

        // Best candidate should have relative_score = 1.0
        assert!(
            (result.rankings[0].relative_score - 1.0).abs() < 0.01,
            "Best candidate should have relative_score ~1.0"
        );
        // Second should be less
        assert!(result.rankings[1].relative_score <= 1.0);
    }

    #[test]
    fn test_ranking_includes_explanations() {
        let analysis = analysis::analyze_text(sample_text()).unwrap();
        let alice = make_profile("Alice", sample_text());

        let result = rank_candidates(&analysis, &[alice]);
        assert!(!result.rankings[0].explanation.narrative.is_empty());
        assert!(!result.rankings[0].explanation.ranked_features.is_empty());
    }

    #[test]
    fn test_ranking_summary_mentions_candidates() {
        let analysis = analysis::analyze_text(sample_text()).unwrap();
        let alice = make_profile("Alice", sample_text());
        let bob = make_profile("Bob", "Different style text entirely. Very formal language used throughout.");

        let result = rank_candidates(&analysis, &[alice, bob]);
        assert!(
            result.summary.contains("Alice"),
            "Summary should mention best candidate: {}",
            result.summary
        );
    }
}
