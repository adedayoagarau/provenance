//! Register classifier for text analysis.
//!
//! Identifies what type of text is being analyzed (academic, literary, technical, etc.)
//! so that baselines can be adjusted accordingly. A low-perplexity legal brief should
//! not be flagged as anomalous — that's expected for its register.

use serde::{Deserialize, Serialize};

use super::AnalysisResult;

/// Top-level register categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Register {
    Academic(AcademicSubtype),
    Literary(LiterarySubtype),
    Technical(TechnicalSubtype),
    Journalistic(JournalisticSubtype),
    Professional(ProfessionalSubtype),
    Casual(CasualSubtype),
    Educational(EducationalSubtype),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcademicSubtype {
    Legal,
    Scientific,
    Humanities,
    SocialScience,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LiterarySubtype {
    LiteraryFiction,
    GenreFiction,
    Poetry,
    CreativeNonfiction,
    Memoir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TechnicalSubtype {
    Documentation,
    ApiReference,
    Specification,
    Whitepaper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JournalisticSubtype {
    News,
    Feature,
    Editorial,
    Opinion,
    Investigative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfessionalSubtype {
    BusinessCorrespondence,
    Report,
    Proposal,
    MarketingCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CasualSubtype {
    BlogPost,
    SocialMedia,
    PersonalEssay,
    ForumPost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EducationalSubtype {
    Textbook,
    StudyGuide,
    Instructional,
}

/// Register classification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterClassification {
    /// Primary register classification.
    pub primary: Register,
    /// Confidence in the classification (0.0–1.0).
    pub confidence: f64,
    /// Human-readable label.
    pub label: String,
    /// Feature scores that drove the classification.
    pub feature_scores: RegisterFeatures,
}

/// Features extracted for register classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFeatures {
    /// Higher = more formal vocabulary.
    pub formality_score: f64,
    /// Average words per sentence.
    pub avg_sentence_length: f64,
    /// Fraction of passive voice constructions.
    pub passive_voice_ratio: f64,
    /// Technical/specialized term density (approximated).
    pub technical_term_density: f64,
    /// Personal pronoun frequency (I, me, my, we, us, our).
    pub personal_pronoun_freq: f64,
    /// Question frequency.
    pub question_ratio: f64,
    /// Contraction frequency.
    pub contraction_ratio: f64,
    /// Discourse marker density.
    pub discourse_marker_ratio: f64,
    /// Flesch-Kincaid grade level.
    pub reading_grade: f64,
    /// Hedge word ratio.
    pub hedge_ratio: f64,
    /// Average word length in characters.
    pub avg_word_length: f64,
}

impl Register {
    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Register::Academic(AcademicSubtype::Legal) => "Academic / Legal",
            Register::Academic(AcademicSubtype::Scientific) => "Academic / Scientific",
            Register::Academic(AcademicSubtype::Humanities) => "Academic / Humanities",
            Register::Academic(AcademicSubtype::SocialScience) => "Academic / Social Science",
            Register::Academic(AcademicSubtype::General) => "Academic / General",
            Register::Literary(LiterarySubtype::LiteraryFiction) => "Literary / Fiction",
            Register::Literary(LiterarySubtype::GenreFiction) => "Literary / Genre Fiction",
            Register::Literary(LiterarySubtype::Poetry) => "Literary / Poetry",
            Register::Literary(LiterarySubtype::CreativeNonfiction) => "Literary / Creative Nonfiction",
            Register::Literary(LiterarySubtype::Memoir) => "Literary / Memoir",
            Register::Technical(TechnicalSubtype::Documentation) => "Technical / Documentation",
            Register::Technical(TechnicalSubtype::ApiReference) => "Technical / API Reference",
            Register::Technical(TechnicalSubtype::Specification) => "Technical / Specification",
            Register::Technical(TechnicalSubtype::Whitepaper) => "Technical / Whitepaper",
            Register::Journalistic(JournalisticSubtype::News) => "Journalistic / News",
            Register::Journalistic(JournalisticSubtype::Feature) => "Journalistic / Feature",
            Register::Journalistic(JournalisticSubtype::Editorial) => "Journalistic / Editorial",
            Register::Journalistic(JournalisticSubtype::Opinion) => "Journalistic / Opinion",
            Register::Journalistic(JournalisticSubtype::Investigative) => "Journalistic / Investigative",
            Register::Professional(ProfessionalSubtype::BusinessCorrespondence) => "Professional / Business",
            Register::Professional(ProfessionalSubtype::Report) => "Professional / Report",
            Register::Professional(ProfessionalSubtype::Proposal) => "Professional / Proposal",
            Register::Professional(ProfessionalSubtype::MarketingCopy) => "Professional / Marketing",
            Register::Casual(CasualSubtype::BlogPost) => "Casual / Blog Post",
            Register::Casual(CasualSubtype::SocialMedia) => "Casual / Social Media",
            Register::Casual(CasualSubtype::PersonalEssay) => "Casual / Personal Essay",
            Register::Casual(CasualSubtype::ForumPost) => "Casual / Forum Post",
            Register::Educational(EducationalSubtype::Textbook) => "Educational / Textbook",
            Register::Educational(EducationalSubtype::StudyGuide) => "Educational / Study Guide",
            Register::Educational(EducationalSubtype::Instructional) => "Educational / Instructional",
        }
    }

    /// Broad category for baseline lookup.
    pub fn broad_category(&self) -> &'static str {
        match self {
            Register::Academic(_) => "academic",
            Register::Literary(_) => "literary",
            Register::Technical(_) => "technical",
            Register::Journalistic(_) => "journalistic",
            Register::Professional(_) => "professional",
            Register::Casual(_) => "casual",
            Register::Educational(_) => "educational",
        }
    }
}

/// Classify the register of analyzed text.
pub fn classify(analysis: &AnalysisResult) -> RegisterClassification {
    let features = extract_features(analysis);

    // Score each broad category
    let mut scores: Vec<(Register, f64)> = Vec::new();

    scores.push((score_academic(&features), score_academic_val(&features)));
    scores.push((score_literary(&features), score_literary_val(&features)));
    scores.push((score_technical(&features), score_technical_val(&features)));
    scores.push((score_journalistic(&features), score_journalistic_val(&features)));
    scores.push((score_professional(&features), score_professional_val(&features)));
    scores.push((score_casual(&features), score_casual_val(&features)));
    scores.push((score_educational(&features), score_educational_val(&features)));

    // Pick highest scoring
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (best_register, best_score) = scores[0].clone();
    let second_score = scores[1].1;

    // Confidence = separation between best and second best
    let total: f64 = scores.iter().map(|(_, s)| s).sum();
    let confidence = if total > 0.0 {
        ((best_score - second_score) / total + 0.5).min(0.95).max(0.3)
    } else {
        0.3
    };

    RegisterClassification {
        label: best_register.label().to_string(),
        primary: best_register,
        confidence,
        feature_scores: features,
    }
}

fn extract_features(analysis: &AnalysisResult) -> RegisterFeatures {
    // Personal pronoun frequency from function word profile
    let personal_pronouns = ["i", "me", "my", "mine", "myself",
                             "we", "us", "our", "ours", "ourselves"];
    let personal_pronoun_freq: f64 = personal_pronouns
        .iter()
        .filter_map(|p| analysis.function_words.frequencies.get(*p))
        .sum();

    // Formality score: combines multiple signals
    // High formality = long sentences, passive voice, low contractions, low personal pronouns
    let formality_score = (analysis.syntactic.avg_words_per_sentence / 30.0).min(1.0) * 0.25
        + analysis.syntactic.passive_voice_ratio * 0.25
        + (1.0 - analysis.stylometric.contraction_ratio * 100.0).max(0.0).min(1.0) * 0.25
        + (1.0 - (personal_pronoun_freq / 50.0).min(1.0)) * 0.25;

    // Technical term density: approximated by avg word length + low personal pronouns
    let technical_term_density =
        ((analysis.lexical.avg_word_length - 4.0) / 3.0).max(0.0).min(1.0);

    RegisterFeatures {
        formality_score,
        avg_sentence_length: analysis.syntactic.avg_words_per_sentence,
        passive_voice_ratio: analysis.syntactic.passive_voice_ratio,
        technical_term_density,
        personal_pronoun_freq,
        question_ratio: analysis.syntactic.interrogative_ratio,
        contraction_ratio: analysis.stylometric.contraction_ratio,
        discourse_marker_ratio: analysis.semantic.discourse_marker_ratio,
        reading_grade: analysis.semantic.flesch_kincaid_grade,
        hedge_ratio: analysis.stylometric.hedge_word_ratio,
        avg_word_length: analysis.lexical.avg_word_length,
    }
}

// --- Scoring functions for each register ---
// Each returns (Register variant, score)

fn score_academic(f: &RegisterFeatures) -> Register {
    // Subtype discrimination
    if f.reading_grade > 14.0 && f.passive_voice_ratio > 0.15 {
        Register::Academic(AcademicSubtype::Legal)
    } else if f.technical_term_density > 0.5 && f.personal_pronoun_freq < 10.0 {
        Register::Academic(AcademicSubtype::Scientific)
    } else if f.discourse_marker_ratio > 0.015 && f.hedge_ratio > 0.01 {
        Register::Academic(AcademicSubtype::Humanities)
    } else {
        Register::Academic(AcademicSubtype::General)
    }
}

fn score_academic_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // Long sentences
    if f.avg_sentence_length > 18.0 { score += 2.0; }
    if f.avg_sentence_length > 24.0 { score += 2.0; }
    // High passive voice — THE strongest academic signal
    // Technical docs rarely exceed 0.15; academic regularly exceeds 0.20
    if f.passive_voice_ratio > 0.10 { score += 3.0; }
    if f.passive_voice_ratio > 0.20 { score += 3.0; }
    if f.passive_voice_ratio > 0.35 { score += 2.0; }
    // Low personal pronouns
    if f.personal_pronoun_freq < 15.0 { score += 1.0; }
    // High reading grade
    if f.reading_grade > 12.0 { score += 2.0; }
    if f.reading_grade > 16.0 { score += 1.5; }
    // High formality
    if f.formality_score > 0.50 { score += 1.5; }
    // Discourse markers — academic uses "furthermore", "however" extensively
    if f.discourse_marker_ratio > 0.006 { score += 2.0; }
    if f.discourse_marker_ratio > 0.012 { score += 1.5; }
    // Long words (but less weight than technical since both have this)
    if f.avg_word_length > 5.0 { score += 0.5; }
    score
}

fn score_literary(_f: &RegisterFeatures) -> Register {
    Register::Literary(LiterarySubtype::LiteraryFiction)
}

fn score_literary_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // Varied sentence lengths (not too uniform)
    if f.avg_sentence_length > 12.0 && f.avg_sentence_length < 22.0 { score += 1.5; }
    // Low passive voice
    if f.passive_voice_ratio < 0.08 { score += 1.5; }
    // Personal pronouns present
    if f.personal_pronoun_freq > 15.0 { score += 2.0; }
    if f.personal_pronoun_freq > 30.0 { score += 1.0; }
    // Some contractions
    if f.contraction_ratio > 0.005 { score += 1.5; }
    // Moderate reading grade
    if f.reading_grade > 6.0 && f.reading_grade < 13.0 { score += 1.5; }
    // Low technical density
    if f.technical_term_density < 0.3 { score += 1.0; }
    // Low discourse markers
    if f.discourse_marker_ratio < 0.01 { score += 1.0; }
    score
}

fn score_technical(_f: &RegisterFeatures) -> Register {
    Register::Technical(TechnicalSubtype::Documentation)
}

fn score_technical_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // High technical term density — strongest technical signal
    if f.technical_term_density > 0.4 { score += 3.0; }
    if f.technical_term_density > 0.6 { score += 1.5; }
    // Short to medium sentences — technical docs are concise
    if f.avg_sentence_length < 22.0 { score += 2.0; }
    if f.avg_sentence_length < 18.0 { score += 1.0; }
    // Low personal pronouns
    if f.personal_pronoun_freq < 10.0 { score += 1.5; }
    // Low contractions
    if f.contraction_ratio < 0.003 { score += 1.5; }
    // Moderate reading grade (not as high as academic)
    if f.reading_grade > 8.0 && f.reading_grade < 16.0 { score += 1.5; }
    // Low hedge words (technical writing is assertive)
    if f.hedge_ratio < 0.005 { score += 1.0; }
    // Long words — technical jargon
    if f.avg_word_length > 5.2 { score += 2.0; }
    if f.avg_word_length > 5.5 { score += 1.0; }
    // Low discourse markers — technical docs don't use "however", "furthermore"
    if f.discourse_marker_ratio < 0.006 { score += 2.0; }
    // PENALTY: Very high passive voice is NOT technical (it's academic)
    if f.passive_voice_ratio > 0.20 { score -= 3.0; }
    score
}

fn score_journalistic(_f: &RegisterFeatures) -> Register {
    Register::Journalistic(JournalisticSubtype::News)
}

fn score_journalistic_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // Medium sentence length
    if f.avg_sentence_length > 15.0 && f.avg_sentence_length < 25.0 { score += 2.0; }
    // Low passive voice
    if f.passive_voice_ratio < 0.10 { score += 1.0; }
    // Low personal pronouns (third person reporting)
    if f.personal_pronoun_freq < 10.0 { score += 1.5; }
    // Few contractions
    if f.contraction_ratio < 0.008 { score += 1.0; }
    // Moderate reading grade (AP style aims for ~8th grade)
    if f.reading_grade > 7.0 && f.reading_grade < 13.0 { score += 2.0; }
    // Moderate formality
    if f.formality_score > 0.4 && f.formality_score < 0.7 { score += 1.5; }
    score
}

fn score_professional(_f: &RegisterFeatures) -> Register {
    Register::Professional(ProfessionalSubtype::Report)
}

fn score_professional_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // Medium-high formality
    if f.formality_score > 0.5 { score += 1.5; }
    // Medium sentence length
    if f.avg_sentence_length > 15.0 && f.avg_sentence_length < 25.0 { score += 1.5; }
    // Some passive voice
    if f.passive_voice_ratio > 0.05 && f.passive_voice_ratio < 0.15 { score += 1.5; }
    // Low personal pronouns (except "we")
    if f.personal_pronoun_freq < 20.0 && f.personal_pronoun_freq > 5.0 { score += 1.0; }
    // Few contractions
    if f.contraction_ratio < 0.005 { score += 1.0; }
    // Moderate reading grade
    if f.reading_grade > 10.0 && f.reading_grade < 15.0 { score += 1.5; }
    score
}

fn score_casual(_f: &RegisterFeatures) -> Register {
    Register::Casual(CasualSubtype::BlogPost)
}

fn score_casual_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // Short sentences
    if f.avg_sentence_length < 18.0 { score += 1.5; }
    // High personal pronouns
    if f.personal_pronoun_freq > 20.0 { score += 2.0; }
    if f.personal_pronoun_freq > 35.0 { score += 1.0; }
    // High contractions
    if f.contraction_ratio > 0.01 { score += 2.0; }
    if f.contraction_ratio > 0.02 { score += 1.0; }
    // Low formality
    if f.formality_score < 0.4 { score += 2.0; }
    // Low reading grade
    if f.reading_grade < 10.0 { score += 1.5; }
    // Questions present
    if f.question_ratio > 0.05 { score += 1.0; }
    score
}

fn score_educational(_f: &RegisterFeatures) -> Register {
    Register::Educational(EducationalSubtype::Textbook)
}

fn score_educational_val(f: &RegisterFeatures) -> f64 {
    let mut score = 0.0;
    // Medium sentence length
    if f.avg_sentence_length > 15.0 && f.avg_sentence_length < 22.0 { score += 1.5; }
    // Some questions (pedagogical)
    if f.question_ratio > 0.03 { score += 1.5; }
    // Moderate formality
    if f.formality_score > 0.4 && f.formality_score < 0.7 { score += 1.5; }
    // Low passive voice
    if f.passive_voice_ratio < 0.10 { score += 1.0; }
    // Moderate reading grade
    if f.reading_grade > 8.0 && f.reading_grade < 14.0 { score += 1.5; }
    // Discourse markers (explanatory)
    if f.discourse_marker_ratio > 0.01 { score += 1.0; }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create an AnalysisResult with specific overrides for testing
    fn classify_text(text: &str) -> RegisterClassification {
        let analysis = super::super::analyze_text(text).unwrap();
        classify(&analysis)
    }

    #[test]
    fn test_register_classification_academic() {
        let text = "The court held that the defendant's actions constituted a breach of \
                    fiduciary duty. Furthermore, the appellate court affirmed the lower \
                    court's decision, noting that the evidence was sufficient to establish \
                    liability. The doctrine of respondeat superior was applied in this case, \
                    whereby the employer was held vicariously liable for the tortious acts \
                    committed by the employee within the scope of employment. The plaintiff's \
                    claim for damages was substantiated by the expert testimony presented \
                    during the proceedings. It was determined that the standard of care had \
                    been violated, resulting in demonstrable harm to the aggrieved party. \
                    The jurisprudence established in prior decisions was instrumental in \
                    shaping the court's analysis of the contractual obligations at issue.";
        let result = classify_text(text);
        assert!(
            matches!(result.primary, Register::Academic(_)),
            "Expected Academic, got {:?}",
            result.primary
        );
    }

    #[test]
    fn test_register_classification_casual() {
        let text = "So I was thinking about this the other day, and honestly? It's wild. \
                    Like, I can't believe we're still talking about this stuff in 2025. \
                    But here's the thing — nobody really knows what they're doing, right? \
                    We're all just figuring it out as we go. I've been trying to get better \
                    at it myself, and it's not easy. Sometimes you just gotta take a step \
                    back and think about what really matters. You know what I mean? \
                    Anyway, that's my take on it. Let me know what you think in the comments. \
                    Don't forget to subscribe if you haven't already!";
        let result = classify_text(text);
        assert!(
            matches!(result.primary, Register::Casual(_)),
            "Expected Casual, got {:?}",
            result.primary
        );
    }

    #[test]
    fn test_register_classification_technical() {
        let text = "The function accepts a configuration object containing the database \
                    connection parameters. Initialize the connection pool with the specified \
                    maximum connections parameter. The middleware component validates the \
                    authentication token against the authorization server. Configure the \
                    retry mechanism with exponential backoff and jitter to prevent \
                    thundering herd problems. The serialization layer transforms the \
                    internal representation into the protocol buffer format specified \
                    in the schema definition. Memory allocation follows the arena \
                    pattern to minimize fragmentation and improve cache locality. \
                    Error propagation utilizes the Result monad pattern with typed \
                    error variants for each failure category.";
        let result = classify_text(text);
        assert!(
            matches!(result.primary, Register::Technical(_)),
            "Expected Technical, got {:?}",
            result.primary
        );
    }

    #[test]
    fn test_register_features_extraction() {
        let text = "This is a simple test. It has short sentences. Nothing fancy here.";
        let analysis = super::super::analyze_text(text).unwrap();
        let features = extract_features(&analysis);
        assert!(features.avg_sentence_length > 0.0);
        assert!(features.formality_score >= 0.0 && features.formality_score <= 1.0);
    }

    #[test]
    fn test_register_confidence_bounds() {
        let text = "The defendant argued that the statute of limitations had expired. \
                    However, the court disagreed and ruled in favor of the plaintiff.";
        let result = classify_text(text);
        assert!(result.confidence >= 0.3 && result.confidence <= 0.95);
    }
}
