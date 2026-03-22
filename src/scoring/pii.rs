//! Process Integrity Index (PII) — measures completeness of process evidence.
//!
//! PII tells the evaluator how much to trust the ACS. Higher PII means more
//! evidence is available for a reliable assessment.

use serde::{Deserialize, Serialize};

use crate::forensics::docx::profile::DocumentConstructionProfile;
use crate::forensics::metadata::FileMetadata;

/// Process Integrity Index result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIntegrityIndex {
    /// PII on 0-100 scale.
    pub score: f64,
    /// Interpretation level.
    pub level: IntegrityLevel,
    /// Human-readable guidance.
    pub guidance: String,
    /// Per-component breakdown.
    pub components: IntegrityComponents,
}

/// Interpretation levels for PII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityLevel {
    /// PII > 80: Strong process evidence.
    Strong,
    /// PII 50-80: Moderate process evidence.
    Moderate,
    /// PII 20-50: Limited process evidence.
    Limited,
    /// PII < 20: Insufficient process evidence.
    Insufficient,
}

impl IntegrityLevel {
    pub fn label(&self) -> &'static str {
        match self {
            IntegrityLevel::Strong => "Strong",
            IntegrityLevel::Moderate => "Moderate",
            IntegrityLevel::Limited => "Limited",
            IntegrityLevel::Insufficient => "Insufficient",
        }
    }
}

/// Component scores that make up PII.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityComponents {
    /// Metadata completeness (0-25).
    pub metadata_completeness: f64,
    /// RSID richness (0-30).
    pub rsid_richness: f64,
    /// Formatting baseline quality (0-20).
    pub formatting_quality: f64,
    /// Revision evidence (0-15).
    pub revision_evidence: f64,
    /// Additional process data (0-10).
    pub additional_evidence: f64,
}

/// Compute the Process Integrity Index.
pub fn compute(
    file_metadata: Option<&FileMetadata>,
    docx_profile: Option<&DocumentConstructionProfile>,
) -> ProcessIntegrityIndex {
    let mut components = IntegrityComponents {
        metadata_completeness: 0.0,
        rsid_richness: 0.0,
        formatting_quality: 0.0,
        revision_evidence: 0.0,
        additional_evidence: 0.0,
    };

    // Metadata completeness (up to 25 points)
    if let Some(meta) = file_metadata {
        if let Some(ref doc) = meta.document_metadata {
            if doc.author.is_some() { components.metadata_completeness += 5.0; }
            if doc.creation_date.is_some() { components.metadata_completeness += 5.0; }
            if doc.modification_date.is_some() { components.metadata_completeness += 5.0; }
            if doc.revision_count.is_some() { components.metadata_completeness += 5.0; }
            if doc.creator_tool.is_some() { components.metadata_completeness += 5.0; }
        }
    }

    // RSID richness (up to 30 points)
    if let Some(profile) = docx_profile {
        if let Some(ref rsid) = profile.rsid_analysis {
            if rsid.total_paragraphs >= 5 {
                components.rsid_richness += 8.0;
            }
            if rsid.unique_rsid_count >= 3 {
                components.rsid_richness += 7.0;
            }
            if rsid.rsid_diversity > 0.1 {
                components.rsid_richness += 8.0;
            }
            if !rsid.settings_rsids.is_empty() {
                components.rsid_richness += 7.0;
            }
        }
    }

    // Formatting baseline quality (up to 20 points)
    if let Some(profile) = docx_profile {
        if let Some(ref fmt) = profile.formatting_analysis {
            if fmt.baseline.default_font.is_some() {
                components.formatting_quality += 7.0;
            }
            if fmt.run_count > 0 {
                components.formatting_quality += 6.0;
            }
            if fmt.formatting_consistency_score > 0.0 {
                components.formatting_quality += 7.0;
            }
        }
    }

    // Revision evidence (up to 15 points)
    if let Some(meta) = file_metadata {
        if let Some(ref doc) = meta.document_metadata {
            if doc.revision_count.map(|r| r > 1).unwrap_or(false) {
                components.revision_evidence += 8.0;
            }
            if doc.custom.contains_key("TotalTime") {
                components.revision_evidence += 7.0;
            }
        }
    }

    // Additional evidence (up to 10 points)
    if let Some(profile) = docx_profile {
        if profile.editing_velocity.is_some() {
            components.additional_evidence += 5.0;
        }
        if profile.creation_to_modification_hours.is_some() {
            components.additional_evidence += 5.0;
        }
    }

    let score = (components.metadata_completeness
        + components.rsid_richness
        + components.formatting_quality
        + components.revision_evidence
        + components.additional_evidence)
        .min(100.0);

    let level = if score > 80.0 {
        IntegrityLevel::Strong
    } else if score > 50.0 {
        IntegrityLevel::Moderate
    } else if score > 20.0 {
        IntegrityLevel::Limited
    } else {
        IntegrityLevel::Insufficient
    };

    let guidance = match level {
        IntegrityLevel::Strong => {
            "Strong process evidence — high confidence in assessment".to_string()
        }
        IntegrityLevel::Moderate => {
            "Moderate process evidence — assessment is informative but not definitive".to_string()
        }
        IntegrityLevel::Limited => {
            "Limited process evidence — assessment should be interpreted cautiously".to_string()
        }
        IntegrityLevel::Insufficient => {
            "Insufficient process evidence — text analysis only, subject to known limitations"
                .to_string()
        }
    };

    ProcessIntegrityIndex {
        score,
        level,
        guidance,
        components,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::metadata::{DocumentMetadata, FileMetadata};
    use std::collections::HashMap;

    fn full_metadata() -> FileMetadata {
        let mut custom = HashMap::new();
        custom.insert("TotalTime".to_string(), "120".to_string());
        FileMetadata {
            file_name: "test.docx".into(),
            file_size: 50000,
            created_epoch: Some(1700000000),
            modified_epoch: Some(1700100000),
            accessed_epoch: None,
            is_readonly: false,
            document_metadata: Some(DocumentMetadata {
                title: Some("Test".into()),
                author: Some("Author".into()),
                subject: None,
                creator_tool: Some("Microsoft Word".into()),
                producer: None,
                creation_date: Some("2024-01-01".into()),
                modification_date: Some("2024-01-15".into()),
                revision_count: Some(15),
                page_count: Some(10),
                reported_word_count: Some(3000),
                custom,
            }),
        }
    }

    #[test]
    fn test_pii_complete_metadata() {
        let meta = full_metadata();
        let pii = compute(Some(&meta), None);
        // Should have high metadata + revision scores
        assert!(
            pii.score > 30.0,
            "Full metadata should yield PII > 30, got {}",
            pii.score
        );
        assert!(pii.components.metadata_completeness >= 20.0);
        assert!(pii.components.revision_evidence > 0.0);
    }

    #[test]
    fn test_pii_no_evidence() {
        let pii = compute(None, None);
        assert_eq!(pii.score, 0.0);
        assert_eq!(pii.level, IntegrityLevel::Insufficient);
    }

    #[test]
    fn test_pii_pdf_minimal() {
        let meta = FileMetadata {
            file_name: "paper.pdf".into(),
            file_size: 200000,
            created_epoch: Some(1700000000),
            modified_epoch: Some(1700100000),
            accessed_epoch: None,
            is_readonly: false,
            document_metadata: Some(DocumentMetadata {
                title: None,
                author: Some("Author".into()),
                subject: None,
                creator_tool: Some("LaTeX".into()),
                producer: Some("pdfTeX".into()),
                creation_date: Some("2024-01-01".into()),
                modification_date: None,
                revision_count: None,
                page_count: Some(20),
                reported_word_count: None,
                custom: HashMap::new(),
            }),
        };
        let pii = compute(Some(&meta), None);
        assert!(pii.score < 30.0, "PDF with minimal metadata should yield low PII");
    }

    #[test]
    fn test_pii_levels() {
        // Verify level boundaries
        assert_eq!(IntegrityLevel::Strong.label(), "Strong");
        assert_eq!(IntegrityLevel::Insufficient.label(), "Insufficient");
    }
}
