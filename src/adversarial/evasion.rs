//! Evasion testing framework — simulates adversarial attacks against Provenance signals.
//!
//! Tests whether stylometric, forensic, and process signals survive various
//! text-level and file-level manipulations. Each attack type has a severity
//! rating and expected impact on different signal categories.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An adversarial attack scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionScenario {
    /// Unique identifier for this scenario.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of the attack.
    pub description: String,
    /// Category of attack.
    pub attack_type: AttackType,
    /// Which signal categories this attack targets.
    pub targeted_signals: Vec<SignalCategory>,
    /// Expected difficulty for attacker (1-5, 5 = hardest to execute).
    pub attacker_difficulty: u8,
    /// Expected prevalence in the wild (1-5, 5 = most common).
    pub prevalence: u8,
}

/// Categories of adversarial attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    /// AI-generated text run through humanizer tools (Undetectable.ai, QuillBot, etc.).
    Humanizer,
    /// Manual post-editing of AI text (rewording, restructuring).
    ManualEditing,
    /// Prompt engineering to produce human-like AI output.
    PromptEngineering,
    /// Style transfer — rewriting text to match a target author's style.
    StyleTransfer,
    /// Paraphrasing — restating content with different wording.
    Paraphrasing,
    /// Back-translation — translate to another language and back.
    BackTranslation,
    /// Homoglyph substitution — replacing characters with lookalikes.
    HomoglyphSubstitution,
    /// Zero-width character injection.
    ZeroWidthInjection,
    /// Metadata stripping — removing forensic evidence from files.
    MetadataStripping,
    /// Document reconstruction — copy text into new document.
    DocumentReconstruction,
    /// Collaborative writing — mixing human and AI contributions.
    CollaborativeWriting,
    /// Template-based writing — human fills in AI-generated skeleton.
    TemplateFilling,
}

impl AttackType {
    pub fn label(&self) -> &'static str {
        match self {
            AttackType::Humanizer => "Humanizer Tool",
            AttackType::ManualEditing => "Manual Editing",
            AttackType::PromptEngineering => "Prompt Engineering",
            AttackType::StyleTransfer => "Style Transfer",
            AttackType::Paraphrasing => "Paraphrasing",
            AttackType::BackTranslation => "Back-Translation",
            AttackType::HomoglyphSubstitution => "Homoglyph Substitution",
            AttackType::ZeroWidthInjection => "Zero-Width Injection",
            AttackType::MetadataStripping => "Metadata Stripping",
            AttackType::DocumentReconstruction => "Document Reconstruction",
            AttackType::CollaborativeWriting => "Collaborative Writing",
            AttackType::TemplateFilling => "Template Filling",
        }
    }

    /// Whether this attack targets text-level signals.
    pub fn targets_text(&self) -> bool {
        matches!(
            self,
            AttackType::Humanizer
                | AttackType::ManualEditing
                | AttackType::PromptEngineering
                | AttackType::StyleTransfer
                | AttackType::Paraphrasing
                | AttackType::BackTranslation
                | AttackType::HomoglyphSubstitution
                | AttackType::ZeroWidthInjection
        )
    }

    /// Whether this attack targets file-level forensic signals.
    pub fn targets_forensics(&self) -> bool {
        matches!(
            self,
            AttackType::MetadataStripping | AttackType::DocumentReconstruction
        )
    }

    /// Whether this attack targets process-level signals.
    pub fn targets_process(&self) -> bool {
        matches!(
            self,
            AttackType::DocumentReconstruction | AttackType::CollaborativeWriting
        )
    }
}

/// Signal categories that can be impacted by attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalCategory {
    /// Vocabulary richness, function words, n-grams.
    Stylometric,
    /// Sentence structure, passive voice, discourse markers.
    Syntactic,
    /// Readability indices, register classification.
    Semantic,
    /// RSID analysis, formatting consistency.
    DocxForensic,
    /// Metadata, timestamps, tool fingerprints.
    FileMetadata,
    /// Keystroke timing, paste detection, session data.
    ProcessCapture,
    /// Perplexity scoring (future).
    Perplexity,
}

impl SignalCategory {
    pub fn label(&self) -> &'static str {
        match self {
            SignalCategory::Stylometric => "Stylometric",
            SignalCategory::Syntactic => "Syntactic",
            SignalCategory::Semantic => "Semantic",
            SignalCategory::DocxForensic => "DOCX Forensic",
            SignalCategory::FileMetadata => "File Metadata",
            SignalCategory::ProcessCapture => "Process Capture",
            SignalCategory::Perplexity => "Perplexity",
        }
    }
}

/// Result of running an evasion test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionTestResult {
    /// The scenario that was tested.
    pub scenario: EvasionScenario,
    /// Per-signal results: signal category → (before_score, after_score, survived).
    pub signal_results: Vec<SignalTestResult>,
    /// Overall robustness score (0.0-1.0, 1.0 = fully robust).
    pub robustness_score: f64,
    /// Which signals survived the attack.
    pub surviving_signals: Vec<SignalCategory>,
    /// Which signals were defeated.
    pub defeated_signals: Vec<SignalCategory>,
    /// Assessment text.
    pub assessment: String,
}

/// Result for a single signal against an attack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTestResult {
    /// Signal category tested.
    pub signal: SignalCategory,
    /// Score before attack (0.0-1.0).
    pub score_before: f64,
    /// Score after attack (0.0-1.0).
    pub score_after: f64,
    /// Whether the signal survived (maintained detection ability).
    pub survived: bool,
    /// Degradation percentage.
    pub degradation_pct: f64,
}

/// The full evasion test catalog.
pub fn standard_scenarios() -> Vec<EvasionScenario> {
    vec![
        EvasionScenario {
            id: "HUM-001".into(),
            name: "Undetectable.ai humanizer".into(),
            description: "AI text processed through Undetectable.ai to evade text-level detectors".into(),
            attack_type: AttackType::Humanizer,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic, SignalCategory::Perplexity],
            attacker_difficulty: 1,
            prevalence: 5,
        },
        EvasionScenario {
            id: "HUM-002".into(),
            name: "QuillBot paraphraser".into(),
            description: "AI text rewritten through QuillBot paraphrasing modes".into(),
            attack_type: AttackType::Humanizer,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic],
            attacker_difficulty: 1,
            prevalence: 5,
        },
        EvasionScenario {
            id: "HUM-003".into(),
            name: "GPTZero bypass humanizer".into(),
            description: "AI text processed through tools specifically targeting GPTZero".into(),
            attack_type: AttackType::Humanizer,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Perplexity],
            attacker_difficulty: 1,
            prevalence: 4,
        },
        EvasionScenario {
            id: "MAN-001".into(),
            name: "Light manual editing".into(),
            description: "AI text with 10-20% of sentences manually rewritten".into(),
            attack_type: AttackType::ManualEditing,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic],
            attacker_difficulty: 3,
            prevalence: 4,
        },
        EvasionScenario {
            id: "MAN-002".into(),
            name: "Heavy manual editing".into(),
            description: "AI text with 50%+ sentences manually rewritten and restructured".into(),
            attack_type: AttackType::ManualEditing,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic, SignalCategory::Semantic],
            attacker_difficulty: 4,
            prevalence: 2,
        },
        EvasionScenario {
            id: "PRO-001".into(),
            name: "Register-matched prompt".into(),
            description: "AI prompted to produce text matching specific academic/literary register".into(),
            attack_type: AttackType::PromptEngineering,
            targeted_signals: vec![SignalCategory::Semantic, SignalCategory::Syntactic],
            attacker_difficulty: 2,
            prevalence: 4,
        },
        EvasionScenario {
            id: "STY-001".into(),
            name: "Author style mimicry".into(),
            description: "AI fine-tuned or prompted with author samples to match writing style".into(),
            attack_type: AttackType::StyleTransfer,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic, SignalCategory::Semantic],
            attacker_difficulty: 4,
            prevalence: 2,
        },
        EvasionScenario {
            id: "BTR-001".into(),
            name: "Back-translation (EN→FR→EN)".into(),
            description: "AI text translated to French and back to English to alter surface patterns".into(),
            attack_type: AttackType::BackTranslation,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic],
            attacker_difficulty: 1,
            prevalence: 3,
        },
        EvasionScenario {
            id: "HOM-001".into(),
            name: "Homoglyph substitution".into(),
            description: "Replace ASCII characters with visually identical Unicode homoglyphs".into(),
            attack_type: AttackType::HomoglyphSubstitution,
            targeted_signals: vec![SignalCategory::Stylometric],
            attacker_difficulty: 2,
            prevalence: 2,
        },
        EvasionScenario {
            id: "ZWC-001".into(),
            name: "Zero-width character injection".into(),
            description: "Insert zero-width spaces/joiners between words/characters".into(),
            attack_type: AttackType::ZeroWidthInjection,
            targeted_signals: vec![SignalCategory::Stylometric],
            attacker_difficulty: 2,
            prevalence: 2,
        },
        EvasionScenario {
            id: "MET-001".into(),
            name: "Metadata stripping".into(),
            description: "Remove all document metadata, RSID data, and revision history".into(),
            attack_type: AttackType::MetadataStripping,
            targeted_signals: vec![SignalCategory::DocxForensic, SignalCategory::FileMetadata],
            attacker_difficulty: 2,
            prevalence: 3,
        },
        EvasionScenario {
            id: "REC-001".into(),
            name: "Document reconstruction".into(),
            description: "Copy-paste AI text into fresh Word document to create organic-looking RSIDs".into(),
            attack_type: AttackType::DocumentReconstruction,
            targeted_signals: vec![SignalCategory::DocxForensic, SignalCategory::ProcessCapture],
            attacker_difficulty: 2,
            prevalence: 4,
        },
        EvasionScenario {
            id: "COL-001".into(),
            name: "Collaborative human+AI".into(),
            description: "Human writes outline and transitions, AI fills in body paragraphs".into(),
            attack_type: AttackType::CollaborativeWriting,
            targeted_signals: vec![SignalCategory::Stylometric, SignalCategory::Syntactic, SignalCategory::ProcessCapture],
            attacker_difficulty: 3,
            prevalence: 5,
        },
        EvasionScenario {
            id: "TPL-001".into(),
            name: "AI skeleton + human fill".into(),
            description: "AI generates structure/outline, human rewrites all content".into(),
            attack_type: AttackType::TemplateFilling,
            targeted_signals: vec![SignalCategory::Semantic],
            attacker_difficulty: 3,
            prevalence: 3,
        },
    ]
}

/// Compute expected signal robustness for a given attack type.
///
/// This is a theoretical/heuristic assessment based on the signal layer architecture.
/// Real measurements require running actual evasion experiments (Phase 12.12).
pub fn assess_robustness(attack: &AttackType) -> HashMap<SignalCategory, f64> {
    let mut robustness = HashMap::new();

    match attack {
        AttackType::Humanizer | AttackType::Paraphrasing => {
            // Text-level signals degraded; forensic/process signals robust
            robustness.insert(SignalCategory::Stylometric, 0.3);
            robustness.insert(SignalCategory::Syntactic, 0.4);
            robustness.insert(SignalCategory::Semantic, 0.5);
            robustness.insert(SignalCategory::DocxForensic, 1.0);
            robustness.insert(SignalCategory::FileMetadata, 1.0);
            robustness.insert(SignalCategory::ProcessCapture, 1.0);
            robustness.insert(SignalCategory::Perplexity, 0.4);
        }
        AttackType::ManualEditing => {
            robustness.insert(SignalCategory::Stylometric, 0.5);
            robustness.insert(SignalCategory::Syntactic, 0.5);
            robustness.insert(SignalCategory::Semantic, 0.6);
            robustness.insert(SignalCategory::DocxForensic, 0.9);
            robustness.insert(SignalCategory::FileMetadata, 0.9);
            robustness.insert(SignalCategory::ProcessCapture, 0.8);
            robustness.insert(SignalCategory::Perplexity, 0.6);
        }
        AttackType::PromptEngineering | AttackType::StyleTransfer => {
            robustness.insert(SignalCategory::Stylometric, 0.4);
            robustness.insert(SignalCategory::Syntactic, 0.4);
            robustness.insert(SignalCategory::Semantic, 0.3);
            robustness.insert(SignalCategory::DocxForensic, 1.0);
            robustness.insert(SignalCategory::FileMetadata, 1.0);
            robustness.insert(SignalCategory::ProcessCapture, 1.0);
            robustness.insert(SignalCategory::Perplexity, 0.3);
        }
        AttackType::BackTranslation => {
            robustness.insert(SignalCategory::Stylometric, 0.2);
            robustness.insert(SignalCategory::Syntactic, 0.3);
            robustness.insert(SignalCategory::Semantic, 0.6);
            robustness.insert(SignalCategory::DocxForensic, 1.0);
            robustness.insert(SignalCategory::FileMetadata, 1.0);
            robustness.insert(SignalCategory::ProcessCapture, 1.0);
            robustness.insert(SignalCategory::Perplexity, 0.3);
        }
        AttackType::HomoglyphSubstitution | AttackType::ZeroWidthInjection => {
            // Detected by existing sanitization (hidden content detection)
            robustness.insert(SignalCategory::Stylometric, 0.9);
            robustness.insert(SignalCategory::Syntactic, 1.0);
            robustness.insert(SignalCategory::Semantic, 1.0);
            robustness.insert(SignalCategory::DocxForensic, 1.0);
            robustness.insert(SignalCategory::FileMetadata, 1.0);
            robustness.insert(SignalCategory::ProcessCapture, 1.0);
            robustness.insert(SignalCategory::Perplexity, 0.9);
        }
        AttackType::MetadataStripping => {
            robustness.insert(SignalCategory::Stylometric, 1.0);
            robustness.insert(SignalCategory::Syntactic, 1.0);
            robustness.insert(SignalCategory::Semantic, 1.0);
            robustness.insert(SignalCategory::DocxForensic, 0.0); // Completely stripped
            robustness.insert(SignalCategory::FileMetadata, 0.0);
            robustness.insert(SignalCategory::ProcessCapture, 1.0);
            robustness.insert(SignalCategory::Perplexity, 1.0);
        }
        AttackType::DocumentReconstruction => {
            robustness.insert(SignalCategory::Stylometric, 1.0);
            robustness.insert(SignalCategory::Syntactic, 1.0);
            robustness.insert(SignalCategory::Semantic, 1.0);
            robustness.insert(SignalCategory::DocxForensic, 0.6); // RSIDs show bulk paste
            robustness.insert(SignalCategory::FileMetadata, 0.7);
            robustness.insert(SignalCategory::ProcessCapture, 0.9);
            robustness.insert(SignalCategory::Perplexity, 1.0);
        }
        AttackType::CollaborativeWriting => {
            robustness.insert(SignalCategory::Stylometric, 0.6);
            robustness.insert(SignalCategory::Syntactic, 0.6);
            robustness.insert(SignalCategory::Semantic, 0.5);
            robustness.insert(SignalCategory::DocxForensic, 0.7);
            robustness.insert(SignalCategory::FileMetadata, 0.8);
            robustness.insert(SignalCategory::ProcessCapture, 0.7);
            robustness.insert(SignalCategory::Perplexity, 0.5);
        }
        AttackType::TemplateFilling => {
            robustness.insert(SignalCategory::Stylometric, 0.7);
            robustness.insert(SignalCategory::Syntactic, 0.7);
            robustness.insert(SignalCategory::Semantic, 0.5);
            robustness.insert(SignalCategory::DocxForensic, 0.8);
            robustness.insert(SignalCategory::FileMetadata, 0.9);
            robustness.insert(SignalCategory::ProcessCapture, 0.8);
            robustness.insert(SignalCategory::Perplexity, 0.6);
        }
    }

    robustness
}

/// Generate a full robustness matrix across all scenarios.
pub fn robustness_matrix() -> RobustnessMatrix {
    let scenarios = standard_scenarios();
    let mut entries = Vec::new();

    for scenario in &scenarios {
        let robustness = assess_robustness(&scenario.attack_type);

        let overall = if robustness.is_empty() {
            0.0
        } else {
            robustness.values().sum::<f64>() / robustness.len() as f64
        };

        let surviving: Vec<SignalCategory> = robustness
            .iter()
            .filter(|(_, &v)| v >= 0.5)
            .map(|(k, _)| *k)
            .collect();

        let defeated: Vec<SignalCategory> = robustness
            .iter()
            .filter(|(_, &v)| v < 0.5)
            .map(|(k, _)| *k)
            .collect();

        entries.push(RobustnessEntry {
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            attack_type: scenario.attack_type,
            signal_robustness: robustness,
            overall_robustness: overall,
            surviving_signals: surviving,
            defeated_signals: defeated,
        });
    }

    let overall_avg = if entries.is_empty() {
        0.0
    } else {
        entries.iter().map(|e| e.overall_robustness).sum::<f64>() / entries.len() as f64
    };

    RobustnessMatrix {
        entries,
        overall_score: overall_avg,
        scenario_count: scenarios.len(),
    }
}

/// A full robustness assessment across all attack scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustnessMatrix {
    pub entries: Vec<RobustnessEntry>,
    pub overall_score: f64,
    pub scenario_count: usize,
}

/// One row in the robustness matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustnessEntry {
    pub scenario_id: String,
    pub scenario_name: String,
    pub attack_type: AttackType,
    pub signal_robustness: HashMap<SignalCategory, f64>,
    pub overall_robustness: f64,
    pub surviving_signals: Vec<SignalCategory>,
    pub defeated_signals: Vec<SignalCategory>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_scenarios_not_empty() {
        let scenarios = standard_scenarios();
        assert!(scenarios.len() >= 10, "Should have at least 10 evasion scenarios");
    }

    #[test]
    fn test_attack_type_properties() {
        assert!(AttackType::Humanizer.targets_text());
        assert!(!AttackType::Humanizer.targets_forensics());
        assert!(AttackType::MetadataStripping.targets_forensics());
        assert!(!AttackType::MetadataStripping.targets_text());
        assert!(AttackType::DocumentReconstruction.targets_forensics());
        assert!(AttackType::DocumentReconstruction.targets_process());
    }

    #[test]
    fn test_robustness_matrix() {
        let matrix = robustness_matrix();
        assert!(matrix.scenario_count >= 10);
        assert!(matrix.overall_score > 0.0 && matrix.overall_score < 1.0);

        // Forensic signals should survive text-level attacks
        for entry in &matrix.entries {
            if entry.attack_type.targets_text() && !entry.attack_type.targets_forensics() {
                let forensic_score = entry
                    .signal_robustness
                    .get(&SignalCategory::DocxForensic)
                    .copied()
                    .unwrap_or(0.0);
                assert!(
                    forensic_score >= 0.9,
                    "DOCX forensic should survive text attacks, got {} for {}",
                    forensic_score,
                    entry.scenario_name
                );
            }
        }
    }

    #[test]
    fn test_metadata_stripping_defeats_forensics() {
        let robustness = assess_robustness(&AttackType::MetadataStripping);
        assert_eq!(robustness[&SignalCategory::DocxForensic], 0.0);
        assert_eq!(robustness[&SignalCategory::FileMetadata], 0.0);
        // But text-level signals survive
        assert_eq!(robustness[&SignalCategory::Stylometric], 1.0);
    }
}
