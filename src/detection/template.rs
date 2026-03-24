//! Template and boilerplate detection for false positive reduction.
//!
//! Legal briefs, medical forms, corporate boilerplate, and academic templates
//! have inherently low burstiness and high structural regularity — patterns
//! that overlap with AI indicators. This module detects template sections
//! and ensures only non-template content is scored.
//!
//! When template content is detected, the detection pipeline scores only
//! the non-template portions of the text.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Template detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateResult {
    /// Whether significant template/boilerplate content was detected.
    pub detected: bool,
    /// Fraction of text identified as template (0.0–1.0).
    pub template_fraction: f64,
    /// Indices of paragraphs identified as template content.
    pub template_paragraphs: Vec<usize>,
    /// The non-template text (for re-scoring).
    pub original_text: String,
    /// Non-template text extracted for scoring.
    pub scorable_text: String,
    /// Types of templates detected.
    pub template_types: Vec<TemplateType>,
}

/// Types of detected templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateType {
    LegalBoilerplate,
    AcademicHeader,
    CorporateTemplate,
    MedicalForm,
    EmailSignature,
    Disclaimer,
    Citation,
    TableOfContents,
}

/// Legal boilerplate phrases.
const LEGAL_PHRASES: &[&str] = &[
    "hereby", "herein", "hereinafter", "whereas", "notwithstanding",
    "pursuant to", "in accordance with", "subject to", "shall be deemed",
    "for the purposes of", "without prejudice", "in witness whereof",
    "duly authorized", "binding upon", "to the fullest extent",
    "governing law", "entire agreement", "severability",
    "force majeure", "indemnify and hold harmless", "liquidated damages",
    "in lieu of", "to the extent permitted by law",
    "the foregoing notwithstanding", "as set forth herein",
];

/// Academic template phrases.
const ACADEMIC_PHRASES: &[&str] = &[
    "abstract:", "keywords:", "introduction", "methodology",
    "literature review", "references", "bibliography", "acknowledgments",
    "submitted in partial fulfillment", "hereby declare",
    "table of contents", "list of figures", "list of tables",
    "appendix", "running head:", "doi:", "issn:", "isbn:",
];

/// Corporate/email boilerplate phrases.
const CORPORATE_PHRASES: &[&str] = &[
    "confidentiality notice", "this email and any attachments",
    "if you are not the intended recipient", "please notify the sender",
    "do not copy or distribute", "views expressed",
    "unsubscribe", "privacy policy", "terms of service",
    "all rights reserved", "copyright ©", "® registered trademark",
    "proprietary and confidential", "authorized use only",
];

/// Analyze text for template/boilerplate content.
pub fn analyze(text: &str) -> TemplateResult {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    if paragraphs.is_empty() {
        return TemplateResult {
            detected: false,
            template_fraction: 0.0,
            template_paragraphs: Vec::new(),
            original_text: text.to_string(),
            scorable_text: text.to_string(),
            template_types: Vec::new(),
        };
    }

    let mut template_paragraphs: Vec<usize> = Vec::new();
    let mut template_types: Vec<TemplateType> = Vec::new();
    let mut template_type_set: HashSet<&str> = HashSet::new();
    let total_chars: usize = paragraphs.iter().map(|p| p.len()).sum();

    for (i, para) in paragraphs.iter().enumerate() {
        let lower = para.to_lowercase();

        // Check legal boilerplate
        let legal_hits = LEGAL_PHRASES.iter().filter(|&&p| lower.contains(p)).count();
        if legal_hits >= 2 {
            template_paragraphs.push(i);
            if !template_type_set.contains("legal") {
                template_types.push(TemplateType::LegalBoilerplate);
                template_type_set.insert("legal");
            }
            continue;
        }

        // Check academic headers/template
        let academic_hits = ACADEMIC_PHRASES.iter().filter(|&&p| lower.contains(p)).count();
        if academic_hits >= 1 && para.len() < 200 {
            template_paragraphs.push(i);
            if !template_type_set.contains("academic") {
                template_types.push(TemplateType::AcademicHeader);
                template_type_set.insert("academic");
            }
            continue;
        }

        // Check corporate/email boilerplate
        let corporate_hits = CORPORATE_PHRASES.iter().filter(|&&p| lower.contains(p)).count();
        if corporate_hits >= 1 {
            template_paragraphs.push(i);
            if !template_type_set.contains("corporate") {
                template_types.push(TemplateType::CorporateTemplate);
                template_type_set.insert("corporate");
            }
            continue;
        }

        // Check disclaimers (short paragraphs with specific patterns)
        if para.len() < 300 && (lower.contains("disclaimer") || lower.contains("notice:")) {
            template_paragraphs.push(i);
            if !template_type_set.contains("disclaimer") {
                template_types.push(TemplateType::Disclaimer);
                template_type_set.insert("disclaimer");
            }
            continue;
        }

        // Check citation blocks (lines starting with numbers/brackets, short)
        if is_citation_block(para) {
            template_paragraphs.push(i);
            if !template_type_set.contains("citation") {
                template_types.push(TemplateType::Citation);
                template_type_set.insert("citation");
            }
            continue;
        }

        // Check email signatures (short, after main content)
        if i >= paragraphs.len().saturating_sub(3) && is_email_signature(para) {
            template_paragraphs.push(i);
            if !template_type_set.contains("email") {
                template_types.push(TemplateType::EmailSignature);
                template_type_set.insert("email");
            }
        }
    }

    // Build scorable text (non-template paragraphs only)
    let scorable_paragraphs: Vec<&str> = paragraphs
        .iter()
        .enumerate()
        .filter(|(i, _)| !template_paragraphs.contains(i))
        .map(|(_, p)| *p)
        .collect();

    let scorable_text = scorable_paragraphs.join("\n\n");

    let template_chars: usize = template_paragraphs
        .iter()
        .map(|&i| paragraphs[i].len())
        .sum();
    let template_fraction = if total_chars > 0 {
        template_chars as f64 / total_chars as f64
    } else {
        0.0
    };

    let detected = template_fraction > 0.10; // More than 10% template content

    TemplateResult {
        detected,
        template_fraction,
        template_paragraphs,
        original_text: text.to_string(),
        scorable_text,
        template_types,
    }
}

/// Check if a paragraph looks like a citation block.
fn is_citation_block(para: &str) -> bool {
    let lines: Vec<&str> = para.lines().collect();
    if lines.is_empty() {
        return false;
    }

    let citation_lines = lines.iter().filter(|line| {
        let trimmed = line.trim();
        // [1] Author... or 1. Author...
        trimmed.starts_with('[')
            || (trimmed.len() > 2
                && trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
                && trimmed.contains('.'))
    }).count();

    citation_lines as f64 / lines.len() as f64 > 0.5 && lines.len() >= 3
}

/// Check if a paragraph looks like an email signature.
fn is_email_signature(para: &str) -> bool {
    let lines: Vec<&str> = para.lines().collect();
    if lines.len() > 8 || lines.is_empty() {
        return false;
    }

    let word_count: usize = lines.iter().map(|l| l.split_whitespace().count()).sum();
    if word_count > 50 {
        return false;
    }

    // Check for signature markers
    let lower = para.to_lowercase();
    lower.contains("regards,")
        || lower.contains("sincerely,")
        || lower.contains("best,")
        || lower.contains("thank you,")
        || lower.contains("sent from my")
        || para.starts_with("--")
        || para.starts_with("___")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_template() {
        let text = "This is a normal essay about climate change. \
                     The earth's temperature has been rising steadily.\n\n\
                     Scientists have documented the effects across many ecosystems. \
                     The data shows clear trends in multiple indicators.";
        let result = analyze(text);
        assert!(!result.detected);
        assert!(result.template_fraction < 0.1);
    }

    #[test]
    fn test_legal_boilerplate() {
        let text = "Normal analysis content here about the topic.\n\n\
                     Pursuant to the terms herein, the parties hereby agree that \
                     notwithstanding any prior agreements, this shall be deemed \
                     the entire agreement in accordance with governing law.\n\n\
                     More original content follows.";
        let result = analyze(text);
        assert!(!result.template_paragraphs.is_empty());
        assert!(result.template_types.iter().any(|t| matches!(t, TemplateType::LegalBoilerplate)));
    }

    #[test]
    fn test_email_signature() {
        let text = "Here is the main content of my message. I wanted to discuss the project.\n\n\
                     The timeline looks good and we should proceed.\n\n\
                     Best regards,\nJohn Smith\nSenior Engineer";
        let result = analyze(text);
        assert!(!result.template_paragraphs.is_empty());
    }

    #[test]
    fn test_scorable_text_extraction() {
        let text = "Abstract: This paper examines climate change.\n\n\
                     The main body of the paper discusses findings in detail. \
                     Our research shows significant temperature increases across \
                     all measured regions over the past fifty years.\n\n\
                     References\n1. Smith et al. 2020\n2. Jones 2019\n3. Lee 2021";
        let result = analyze(text);
        // Scorable text should exclude abstract header and references
        assert!(result.scorable_text.contains("main body"));
    }
}
