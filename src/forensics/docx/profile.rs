//! Document Construction Profile — combines all DOCX forensic findings.
//!
//! Classifies documents as Organic, BulkInsertion, Hybrid, or Insufficient
//! and generates a human-readable assessment.

use serde::{Deserialize, Serialize};

use super::formatting::FormattingAnalysis;
use super::rsid::RsidAnalysis;
use super::structure::StructuralForensics;
use crate::forensics::metadata::DocumentMetadata;

/// How the document was constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstructionPattern {
    /// High RSID diversity, proportional editing time, consistent formatting, revision scatter > 30%.
    Organic,
    /// Low RSID diversity, largest block > 50% of paragraphs, disproportionately low editing time.
    BulkInsertion,
    /// Mixed signals — some sections organic, some bulk.
    Hybrid,
    /// Insufficient metadata (PDF-to-docx, very short, non-Word editor).
    Insufficient,
}

/// A forensic anomaly from any sub-analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicAnomaly {
    pub anomaly_type: ForensicAnomalyType,
    pub location: Option<(usize, usize)>,
    pub severity: ForensicSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForensicAnomalyType {
    Rsid,
    Formatting,
    Structural,
    Metadata,
    Temporal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForensicSeverity {
    Low,
    Medium,
    High,
}

/// Complete document construction profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentConstructionProfile {
    pub metadata: Option<DocumentMetadata>,
    pub rsid_analysis: Option<RsidAnalysis>,
    pub formatting_analysis: Option<FormattingAnalysis>,
    pub structural_forensics: Option<StructuralForensics>,
    pub construction_pattern: ConstructionPattern,
    pub anomalies: Vec<ForensicAnomaly>,
    pub process_integrity_score: f64,
    pub assessment_text: String,

    // Derived metadata metrics
    pub editing_velocity: Option<f64>,
    pub saves_per_hour: Option<f64>,
    pub creation_to_modification_hours: Option<f64>,
    pub words_per_save: Option<f64>,
}

/// Build a complete document construction profile from all sub-analyses.
pub fn build_profile(
    metadata: Option<&DocumentMetadata>,
    rsid_analysis: Option<&RsidAnalysis>,
    formatting_analysis: Option<&FormattingAnalysis>,
    structural_forensics: Option<&StructuralForensics>,
) -> DocumentConstructionProfile {
    // Compute derived metadata metrics
    let total_time_minutes = metadata
        .and_then(|m| m.custom.get("TotalTime"))
        .and_then(|t| t.parse::<f64>().ok());

    let word_count = metadata
        .and_then(|m| m.reported_word_count)
        .map(|w| w as f64);

    let revision_count = metadata
        .and_then(|m| m.revision_count)
        .map(|r| r as f64);

    let editing_velocity = match (word_count, total_time_minutes) {
        (Some(w), Some(t)) if t > 0.0 => Some(w / t),
        _ => None,
    };

    let saves_per_hour = match (revision_count, total_time_minutes) {
        (Some(r), Some(t)) if t > 0.0 => Some(r / (t / 60.0)),
        _ => None,
    };

    let words_per_save = match (word_count, revision_count) {
        (Some(w), Some(r)) if r > 0.0 => Some(w / r),
        _ => None,
    };

    let creation_to_modification_hours = compute_time_span(metadata);

    // Collect anomalies from all sub-analyses
    let mut anomalies = Vec::new();

    if let Some(rsid) = rsid_analysis {
        for a in &rsid.anomalies {
            anomalies.push(ForensicAnomaly {
                anomaly_type: ForensicAnomalyType::Rsid,
                location: match (a.start_paragraph, a.end_paragraph) {
                    (Some(s), Some(e)) => Some((s, e)),
                    _ => None,
                },
                severity: match a.severity {
                    super::rsid::AnomalySeverity::Low => ForensicSeverity::Low,
                    super::rsid::AnomalySeverity::Medium => ForensicSeverity::Medium,
                    super::rsid::AnomalySeverity::High => ForensicSeverity::High,
                },
                description: a.description.clone(),
            });
        }
    }

    if let Some(fmt) = formatting_analysis {
        for a in &fmt.anomalies {
            anomalies.push(ForensicAnomaly {
                anomaly_type: ForensicAnomalyType::Formatting,
                location: Some((a.start_paragraph, a.end_paragraph)),
                severity: ForensicSeverity::Medium,
                description: a.details.clone(),
            });
        }
    }

    // Metadata anomalies
    if let Some(vel) = editing_velocity {
        if vel > 50.0 {
            anomalies.push(ForensicAnomaly {
                anomaly_type: ForensicAnomalyType::Metadata,
                location: None,
                severity: ForensicSeverity::High,
                description: format!(
                    "Editing velocity ({vel:.0} words/minute) exceeds 50 wpm sustained, \
                     suggesting bulk text insertion rather than original composition"
                ),
            });
        }
    }

    if let Some(wps) = words_per_save {
        if wps > 2000.0 {
            anomalies.push(ForensicAnomaly {
                anomaly_type: ForensicAnomalyType::Metadata,
                location: None,
                severity: ForensicSeverity::Medium,
                description: format!(
                    "High words-per-save ratio ({wps:.0}): document has very few saves \
                     relative to its word count"
                ),
            });
        }
    }

    if let (Some(wc), Some(span)) = (word_count, creation_to_modification_hours) {
        if wc > 5000.0 && span < 0.5 {
            anomalies.push(ForensicAnomaly {
                anomaly_type: ForensicAnomalyType::Temporal,
                location: None,
                severity: ForensicSeverity::High,
                description: format!(
                    "{:.0}-word document with creation-to-modification span of {:.0} minutes \
                     suggests bulk content insertion",
                    wc,
                    span * 60.0
                ),
            });
        }
    }

    // Classify construction pattern
    let construction_pattern = classify_pattern(
        rsid_analysis,
        formatting_analysis,
        editing_velocity,
        total_time_minutes,
    );

    // Compute process integrity score
    let process_integrity_score = compute_integrity_score(
        metadata,
        rsid_analysis,
        formatting_analysis,
    );

    // Generate assessment text
    let assessment_text = generate_assessment(
        &construction_pattern,
        rsid_analysis,
        metadata,
        total_time_minutes,
        creation_to_modification_hours,
    );

    DocumentConstructionProfile {
        metadata: metadata.cloned(),
        rsid_analysis: rsid_analysis.cloned(),
        formatting_analysis: formatting_analysis.cloned(),
        structural_forensics: structural_forensics.cloned(),
        construction_pattern,
        anomalies,
        process_integrity_score,
        assessment_text,
        editing_velocity,
        saves_per_hour,
        creation_to_modification_hours,
        words_per_save,
    }
}

fn classify_pattern(
    rsid: Option<&RsidAnalysis>,
    formatting: Option<&FormattingAnalysis>,
    editing_velocity: Option<f64>,
    _total_time: Option<f64>,
) -> ConstructionPattern {
    let Some(rsid) = rsid else {
        return ConstructionPattern::Insufficient;
    };

    if rsid.total_paragraphs < 5 {
        return ConstructionPattern::Insufficient;
    }

    let high_diversity = rsid.rsid_diversity > 0.15;
    let high_scatter = rsid.revision_scatter > 0.30;
    let formatting_ok = formatting
        .map(|f| f.formatting_consistency_score > 0.8)
        .unwrap_or(true);
    let velocity_ok = editing_velocity.map(|v| v < 50.0).unwrap_or(true);

    let largest_block_ratio = rsid.largest_block_size as f64 / rsid.total_paragraphs as f64;
    let low_diversity = rsid.rsid_diversity <= 0.05;
    let huge_block = largest_block_ratio > 0.5;

    if high_diversity && high_scatter && formatting_ok && velocity_ok {
        ConstructionPattern::Organic
    } else if low_diversity && huge_block {
        ConstructionPattern::BulkInsertion
    } else if !high_diversity || !formatting_ok || !velocity_ok {
        // Mixed signals
        if rsid.anomalies.is_empty() && formatting_ok {
            ConstructionPattern::Organic
        } else {
            ConstructionPattern::Hybrid
        }
    } else {
        ConstructionPattern::Organic
    }
}

fn compute_integrity_score(
    metadata: Option<&DocumentMetadata>,
    rsid: Option<&RsidAnalysis>,
    formatting: Option<&FormattingAnalysis>,
) -> f64 {
    let mut score: f64 = 0.0;
    let mut max_score: f64 = 0.0;

    // Metadata completeness (up to 30 points)
    max_score += 30.0;
    if let Some(meta) = metadata {
        if meta.author.is_some() {
            score += 5.0;
        }
        if meta.creation_date.is_some() {
            score += 5.0;
        }
        if meta.modification_date.is_some() {
            score += 5.0;
        }
        if meta.revision_count.is_some() {
            score += 5.0;
        }
        if meta.creator_tool.is_some() {
            score += 5.0;
        }
        if meta.custom.contains_key("TotalTime") {
            score += 5.0;
        }
    }

    // RSID richness (up to 40 points)
    max_score += 40.0;
    if let Some(rsid) = rsid {
        if rsid.total_paragraphs >= 5 {
            score += 10.0; // Has enough paragraphs
        }
        if rsid.unique_rsid_count >= 3 {
            score += 10.0; // Multiple editing sessions
        }
        if rsid.rsid_diversity > 0.1 {
            score += 10.0; // Good diversity
        }
        if !rsid.settings_rsids.is_empty() {
            score += 10.0; // Settings RSIDs present
        }
    }

    // Formatting baseline (up to 30 points)
    max_score += 30.0;
    if let Some(fmt) = formatting {
        if fmt.baseline.default_font.is_some() {
            score += 10.0;
        }
        if fmt.run_count > 0 {
            score += 10.0;
        }
        if fmt.formatting_consistency_score > 0.0 {
            score += 10.0;
        }
    }

    if max_score > 0.0 {
        (score / max_score * 100.0).min(100.0)
    } else {
        0.0
    }
}

fn compute_time_span(metadata: Option<&DocumentMetadata>) -> Option<f64> {
    let meta = metadata?;
    let created = meta.creation_date.as_ref()?;
    let modified = meta.modification_date.as_ref()?;

    let created_epoch = parse_iso_approximate(created)?;
    let modified_epoch = parse_iso_approximate(modified)?;

    if modified_epoch > created_epoch {
        Some((modified_epoch - created_epoch) as f64 / 3600.0)
    } else {
        Some(0.0)
    }
}

fn parse_iso_approximate(date_str: &str) -> Option<u64> {
    let parts: Vec<&str> = date_str
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() >= 3 {
        let year: u64 = parts[0].parse().ok()?;
        let month: u64 = parts[1].parse().ok()?;
        let day: u64 = parts[2].parse().ok()?;

        if year < 1970 || year > 2100 {
            return None;
        }

        let mut epoch = (year - 1970) * 365 * 86400 + (month - 1) * 30 * 86400 + (day - 1) * 86400;

        if parts.len() >= 4 {
            let hour: u64 = parts[3].parse().ok()?;
            epoch += hour * 3600;
        }
        if parts.len() >= 5 {
            let min: u64 = parts[4].parse().ok()?;
            epoch += min * 60;
        }

        Some(epoch)
    } else {
        None
    }
}

fn generate_assessment(
    pattern: &ConstructionPattern,
    rsid: Option<&RsidAnalysis>,
    _metadata: Option<&DocumentMetadata>,
    total_time: Option<f64>,
    time_span_hours: Option<f64>,
) -> String {
    let sessions = rsid.map(|r| r.unique_rsid_count).unwrap_or(0);
    let time_desc = total_time
        .map(|t| {
            if t >= 60.0 {
                format!("{:.1} hours", t / 60.0)
            } else {
                format!("{:.0} minutes", t)
            }
        })
        .unwrap_or_else(|| "unknown duration".to_string());

    let span_desc = time_span_hours
        .map(|h| {
            if h >= 24.0 {
                format!("{:.0} days", h / 24.0)
            } else if h >= 1.0 {
                format!("{:.1} hours", h)
            } else {
                format!("{:.0} minutes", h * 60.0)
            }
        })
        .unwrap_or_else(|| "unknown span".to_string());

    match pattern {
        ConstructionPattern::Organic => {
            format!(
                "This document was constructed across approximately {sessions} editing sessions \
                 over {span_desc}, with {time_desc} of active editing time. \
                 RSID analysis shows progressive, non-uniform construction with substantial \
                 revision scatter, consistent with incremental human composition."
            )
        }
        ConstructionPattern::BulkInsertion => {
            let block_info = rsid
                .map(|r| {
                    format!(
                        "The largest contiguous block spans {} of {} paragraphs.",
                        r.largest_block_size, r.total_paragraphs
                    )
                })
                .unwrap_or_default();
            format!(
                "This document shows patterns consistent with bulk text insertion. \
                 {block_info} \
                 Limited RSID diversity ({sessions} unique editing sessions) and \
                 {time_desc} of recorded editing time suggest the content may not have been \
                 composed incrementally within this document."
            )
        }
        ConstructionPattern::Hybrid => {
            format!(
                "This document shows mixed construction patterns across {sessions} editing sessions \
                 over {span_desc}. Some sections show incremental development while others \
                 contain characteristics worth discussing with the writer. \
                 See the anomaly details for specific regions of interest."
            )
        }
        ConstructionPattern::Insufficient => {
            format!(
                "Insufficient process evidence is available for this document to make a \
                 reliable construction assessment. This may be due to the document being very \
                 short, created by a non-standard editor, or converted from another format."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::docx::rsid;
    use std::collections::HashMap;

    fn make_metadata(
        revision_count: Option<u32>,
        word_count: Option<u32>,
        total_time: Option<&str>,
    ) -> DocumentMetadata {
        let mut custom = HashMap::new();
        if let Some(t) = total_time {
            custom.insert("TotalTime".to_string(), t.to_string());
        }
        DocumentMetadata {
            title: None,
            author: Some("Test Author".into()),
            subject: None,
            creator_tool: Some("Microsoft Office Word".into()),
            producer: None,
            creation_date: Some("2024-01-01T09:00:00Z".into()),
            modification_date: Some("2024-01-15T17:00:00Z".into()),
            revision_count,
            page_count: None,
            reported_word_count: word_count,
            custom,
        }
    }

    #[test]
    fn test_organic_classification() {
        // Build an RSID analysis that looks organic
        let mut para_rsids = Vec::new();
        let rsid_values = [
            "00A11111", "00B22222", "00C33333", "00D44444", "00E55555",
            "00F66666", "00A77777", "00B88888", "00C99999", "00DAAAAA",
        ];
        for (i, rsid) in rsid_values.iter().enumerate() {
            para_rsids.push(rsid::ParagraphRsid {
                paragraph_index: i,
                rsid_r: Some(rsid.to_string()),
                rsid_r_default: None,
                rsid_p: None,
            });
        }

        let doc_xml = build_test_xml(&rsid_values);
        let rsid_analysis = rsid::analyze(&doc_xml, None);
        let metadata = make_metadata(Some(15), Some(3000), Some("180"));

        let profile = build_profile(
            Some(&metadata),
            Some(&rsid_analysis),
            None,
            None,
        );

        assert_eq!(profile.construction_pattern, ConstructionPattern::Organic);
        assert!(profile.process_integrity_score > 30.0);
        assert!(profile.assessment_text.contains("editing sessions"));
    }

    #[test]
    fn test_bulk_insertion_classification() {
        let rsid_values: Vec<&str> = vec!["00A11111"; 20];
        let doc_xml = build_test_xml(&rsid_values);
        let rsid_analysis = rsid::analyze(&doc_xml, None);
        let metadata = make_metadata(Some(1), Some(5000), Some("5"));

        let profile = build_profile(
            Some(&metadata),
            Some(&rsid_analysis),
            None,
            None,
        );

        assert_eq!(profile.construction_pattern, ConstructionPattern::BulkInsertion);
        assert!(profile.assessment_text.contains("bulk text insertion"));
    }

    #[test]
    fn test_insufficient_short_doc() {
        let rsid_values = ["00A11111", "00B22222"];
        let doc_xml = build_test_xml(&rsid_values);
        let rsid_analysis = rsid::analyze(&doc_xml, None);

        let profile = build_profile(None, Some(&rsid_analysis), None, None);
        assert_eq!(profile.construction_pattern, ConstructionPattern::Insufficient);
    }

    #[test]
    fn test_editing_velocity_anomaly() {
        let rsid_values: Vec<&str> = (0..10)
            .map(|i| match i % 3 {
                0 => "00A11111",
                1 => "00B22222",
                _ => "00C33333",
            })
            .collect();
        let doc_xml = build_test_xml(&rsid_values);
        let rsid_analysis = rsid::analyze(&doc_xml, None);
        // 10000 words in 5 minutes = 2000 wpm
        let metadata = make_metadata(Some(2), Some(10000), Some("5"));

        let profile = build_profile(
            Some(&metadata),
            Some(&rsid_analysis),
            None,
            None,
        );

        assert!(profile.editing_velocity.unwrap() > 50.0);
        assert!(profile
            .anomalies
            .iter()
            .any(|a| a.anomaly_type == ForensicAnomalyType::Metadata));
    }

    fn build_test_xml(rsid_values: &[&str]) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>"#,
        );
        for rsid in rsid_values {
            xml.push_str(&format!(
                r#"<w:p w:rsidR="{rsid}"><w:r><w:t>Some text content here.</w:t></w:r></w:p>"#,
            ));
        }
        xml.push_str("</w:body></w:document>");
        xml
    }
}
