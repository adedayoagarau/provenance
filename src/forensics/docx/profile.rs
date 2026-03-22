use super::formatting::FormattingAnalysis;
use super::metadata::DocxMetadata;
use super::rsid::{AnomalySeverity, RsidAnalysis};
use super::structure::StructuralForensics;
use serde::{Deserialize, Serialize};

/// How the document appears to have been constructed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstructionPattern {
    /// Incremental writing across multiple sessions with revision
    Organic,
    /// Most or all content inserted in a single operation
    BulkInsertion,
    /// Mix of organic and bulk-inserted sections
    Hybrid,
    /// Not enough metadata to assess
    Insufficient,
}

/// A forensic anomaly with type, location, severity, and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicAnomaly {
    pub anomaly_type: String,
    pub location: Option<String>,
    pub severity: AnomalySeverity,
    pub description: String,
}

/// Complete document construction profile combining all forensic analyses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentConstructionProfile {
    pub metadata: DocxMetadata,
    pub rsid_analysis: RsidAnalysis,
    pub formatting_analysis: FormattingAnalysis,
    pub structural_forensics: StructuralForensics,
    pub construction_pattern: ConstructionPattern,
    pub anomalies: Vec<ForensicAnomaly>,
    pub process_integrity_score: f64,
    pub assessment_text: String,
}

impl DocumentConstructionProfile {
    /// Build a complete construction profile from all forensic analyses.
    pub fn build(
        metadata: DocxMetadata,
        rsid_analysis: RsidAnalysis,
        formatting_analysis: FormattingAnalysis,
        structural_forensics: StructuralForensics,
    ) -> Self {
        let construction_pattern = classify_construction(
            &metadata,
            &rsid_analysis,
            &formatting_analysis,
            &structural_forensics,
        );

        let anomalies = collect_anomalies(
            &metadata,
            &rsid_analysis,
            &formatting_analysis,
            &structural_forensics,
        );

        let process_integrity_score = compute_integrity_score(
            &metadata,
            &rsid_analysis,
            &formatting_analysis,
        );

        let assessment_text = generate_assessment(
            &metadata,
            &rsid_analysis,
            &construction_pattern,
            process_integrity_score,
        );

        DocumentConstructionProfile {
            metadata,
            rsid_analysis,
            formatting_analysis,
            structural_forensics,
            construction_pattern,
            anomalies,
            process_integrity_score,
            assessment_text,
        }
    }

    /// Render as a human-readable text report.
    pub fn render_text(&self) -> String {
        let mut r = String::new();

        r.push_str("═══════════════════════════════════════════════════\n");
        r.push_str("  PROVENANCE — Document Construction Profile\n");
        r.push_str("═══════════════════════════════════════════════════\n\n");

        // Metadata
        r.push_str("── Document Metadata ──\n");
        if let Some(ref author) = self.metadata.author {
            r.push_str(&format!("  Author:          {author}\n"));
        }
        if let Some(ref app) = self.metadata.application {
            r.push_str(&format!("  Application:     {app}\n"));
        }
        if let Some(ref created) = self.metadata.created {
            r.push_str(&format!("  Created:         {created}\n"));
        }
        if let Some(ref modified) = self.metadata.modified {
            r.push_str(&format!("  Modified:        {modified}\n"));
        }
        if let Some(rev) = self.metadata.revision_count {
            r.push_str(&format!("  Revisions:       {rev}\n"));
        }
        if let Some(time) = self.metadata.total_editing_time_minutes {
            r.push_str(&format!("  Editing Time:    {time} minutes\n"));
        }
        if let Some(words) = self.metadata.reported_word_count {
            r.push_str(&format!("  Word Count:      {words}\n"));
        }
        if let Some(velocity) = self.metadata.editing_velocity_wpm {
            r.push_str(&format!("  Editing Velocity: {velocity:.1} words/minute\n"));
        }
        r.push('\n');

        // RSID Analysis
        r.push_str("── RSID Analysis ──\n");
        r.push_str(&format!(
            "  Unique Sessions:    {}\n",
            self.rsid_analysis.unique_rsid_count
        ));
        r.push_str(&format!(
            "  Total Paragraphs:   {}\n",
            self.rsid_analysis.total_paragraphs
        ));
        r.push_str(&format!(
            "  RSID Diversity:     {:.3}\n",
            self.rsid_analysis.rsid_diversity
        ));
        r.push_str(&format!(
            "  Largest Block:      {} paragraphs\n",
            self.rsid_analysis.largest_block_size
        ));
        r.push_str(&format!(
            "  Revision Scatter:   {:.1}%\n",
            self.rsid_analysis.revision_scatter * 100.0
        ));
        if let Some(corr) = self.rsid_analysis.rsid_progression_correlation {
            r.push_str(&format!("  Progression Corr:   {corr:.3}\n"));
        }
        r.push('\n');

        // Formatting
        r.push_str("── Formatting Consistency ──\n");
        r.push_str(&format!(
            "  Consistency Score:  {:.1}%\n",
            self.formatting_analysis.formatting_consistency_score * 100.0
        ));
        r.push_str(&format!(
            "  Total Runs:         {}\n",
            self.formatting_analysis.total_runs
        ));
        if let Some(ref font) = self.formatting_analysis.baseline_font {
            r.push_str(&format!("  Baseline Font:      {font}\n"));
        }
        if !self.formatting_analysis.anomalies.is_empty() {
            r.push_str(&format!(
                "  Anomaly Clusters:   {}\n",
                self.formatting_analysis.anomalies.len()
            ));
        }
        r.push('\n');

        // Structure
        r.push_str("── Structural Analysis ──\n");
        r.push_str(&format!(
            "  Paragraphs:         {}\n",
            self.structural_forensics.total_paragraphs
        ));
        r.push_str(&format!(
            "  Avg Length:          {:.1} words\n",
            self.structural_forensics.mean_paragraph_length
        ));
        r.push_str(&format!(
            "  Length Variance CV:  {:.3}\n",
            self.structural_forensics.coefficient_of_variation
        ));
        if self.structural_forensics.uniform_paragraph_flag {
            r.push_str("  [!] Paragraph lengths are unusually uniform\n");
        }
        if self.structural_forensics.uniform_section_flag {
            r.push_str("  [!] Section lengths are unusually balanced\n");
        }
        r.push('\n');

        // Construction Pattern
        r.push_str("── Construction Assessment ──\n");
        r.push_str(&format!(
            "  Pattern:            {:?}\n",
            self.construction_pattern
        ));
        r.push_str(&format!(
            "  Integrity Score:    {:.0}/100\n",
            self.process_integrity_score * 100.0
        ));
        r.push('\n');

        // Assessment
        r.push_str("── Assessment ──\n");
        r.push_str(&format!("  {}\n", self.assessment_text));
        r.push('\n');

        // Anomalies
        if !self.anomalies.is_empty() {
            r.push_str("── Anomalies ──\n");
            for anomaly in &self.anomalies {
                let loc = anomaly
                    .location
                    .as_deref()
                    .unwrap_or("Document-wide");
                r.push_str(&format!(
                    "  [{:?}] ({loc}) {}\n",
                    anomaly.severity, anomaly.description
                ));
            }
            r.push('\n');
        }

        r.push_str("═══════════════════════════════════════════════════\n");
        r
    }

    /// Render as JSON.
    pub fn render_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Classify the construction pattern based on forensic evidence.
fn classify_construction(
    metadata: &DocxMetadata,
    rsid: &RsidAnalysis,
    formatting: &FormattingAnalysis,
    structure: &StructuralForensics,
) -> ConstructionPattern {
    // Check if we have enough data
    let has_rsids = rsid.unique_rsid_count > 0;
    let has_metadata = metadata.total_editing_time_minutes.is_some()
        || metadata.revision_count.is_some();

    if !has_rsids && !has_metadata {
        return ConstructionPattern::Insufficient;
    }

    if rsid.total_paragraphs < 3 {
        return ConstructionPattern::Insufficient;
    }

    // Organic indicators
    let high_diversity = rsid.rsid_diversity > 0.15;
    let high_scatter = rsid.revision_scatter > 0.30;
    let consistent_formatting = formatting.formatting_consistency_score > 0.8;
    let proportional_time = metadata
        .editing_velocity_wpm
        .map(|v| v < 50.0)
        .unwrap_or(true);
    let varied_paragraphs = structure.coefficient_of_variation > 0.3;

    // Bulk insertion indicators
    let low_diversity = rsid.rsid_diversity < 0.10;
    let large_block = rsid.largest_block_size as f64 > rsid.total_paragraphs as f64 * 0.5;
    let dominant_block = rsid.largest_block_size as f64 > rsid.total_paragraphs as f64 * 0.9;
    let fast_creation = metadata.editing_velocity_wpm.map(|v| v > 50.0).unwrap_or(false);

    // Check for hybrid: some sections organic, some bulk
    let has_paste_events = !rsid.paste_events.is_empty();
    let paste_coverage: usize = rsid
        .paste_events
        .iter()
        .map(|e| e.block_size)
        .sum();
    let paste_fraction = if rsid.total_paragraphs > 0 {
        paste_coverage as f64 / rsid.total_paragraphs as f64
    } else {
        0.0
    };

    // A single rsid dominating >90% of the document is strong bulk insertion signal
    if dominant_block && rsid.unique_rsid_count <= 2 {
        ConstructionPattern::BulkInsertion
    } else if low_diversity && large_block {
        ConstructionPattern::BulkInsertion
    } else if has_paste_events && paste_fraction > 0.1 && paste_fraction < 0.8 && high_diversity {
        ConstructionPattern::Hybrid
    } else if high_diversity && high_scatter && proportional_time && consistent_formatting {
        ConstructionPattern::Organic
    } else if high_diversity && (high_scatter || varied_paragraphs) {
        ConstructionPattern::Organic
    } else if fast_creation && !high_diversity {
        ConstructionPattern::BulkInsertion
    } else {
        // Default: look at the weight of evidence
        let organic_score = high_diversity as u8
            + high_scatter as u8
            + consistent_formatting as u8
            + proportional_time as u8
            + varied_paragraphs as u8;
        let bulk_score =
            low_diversity as u8 + large_block as u8 + fast_creation as u8;

        if organic_score >= 3 {
            ConstructionPattern::Organic
        } else if bulk_score >= 2 {
            ConstructionPattern::BulkInsertion
        } else if has_paste_events {
            ConstructionPattern::Hybrid
        } else {
            ConstructionPattern::Organic
        }
    }
}

/// Collect all anomalies from sub-analyses.
fn collect_anomalies(
    metadata: &DocxMetadata,
    rsid: &RsidAnalysis,
    formatting: &FormattingAnalysis,
    _structure: &StructuralForensics,
) -> Vec<ForensicAnomaly> {
    let mut anomalies = Vec::new();

    // Metadata flags
    if let Some(ref flag) = metadata.velocity_flag {
        anomalies.push(ForensicAnomaly {
            anomaly_type: "TemporalAnomaly".to_string(),
            location: None,
            severity: AnomalySeverity::Medium,
            description: flag.clone(),
        });
    }
    if let Some(ref flag) = metadata.single_save_flag {
        anomalies.push(ForensicAnomaly {
            anomaly_type: "TemporalAnomaly".to_string(),
            location: None,
            severity: AnomalySeverity::Medium,
            description: flag.clone(),
        });
    }
    if let Some(ref flag) = metadata.rapid_creation_flag {
        anomalies.push(ForensicAnomaly {
            anomaly_type: "TemporalAnomaly".to_string(),
            location: None,
            severity: AnomalySeverity::High,
            description: flag.clone(),
        });
    }

    // RSID anomalies
    for rsid_anomaly in &rsid.anomalies {
        let location = match (rsid_anomaly.start_paragraph, rsid_anomaly.end_paragraph) {
            (Some(s), Some(e)) => Some(format!("Paragraphs {s}-{e}")),
            _ => None,
        };
        anomalies.push(ForensicAnomaly {
            anomaly_type: rsid_anomaly.anomaly_type.clone(),
            location,
            severity: rsid_anomaly.severity.clone(),
            description: rsid_anomaly.description.clone(),
        });
    }

    // Formatting anomalies
    for fmt_anomaly in &formatting.anomalies {
        anomalies.push(ForensicAnomaly {
            anomaly_type: format!("{:?}", fmt_anomaly.anomaly_type),
            location: Some(format!(
                "Paragraphs {}-{}",
                fmt_anomaly.start_paragraph, fmt_anomaly.end_paragraph
            )),
            severity: AnomalySeverity::Medium,
            description: fmt_anomaly.details.clone(),
        });
    }

    anomalies
}

/// Compute process integrity score (0.0 - 1.0).
fn compute_integrity_score(
    metadata: &DocxMetadata,
    rsid: &RsidAnalysis,
    formatting: &FormattingAnalysis,
) -> f64 {
    let mut score = 0.0;
    let mut max_score = 0.0;

    // Metadata completeness (0.3 weight)
    max_score += 0.3;
    let mut meta_fields = 0u32;
    let mut meta_present = 0u32;

    meta_fields += 1;
    if metadata.author.is_some() {
        meta_present += 1;
    }
    meta_fields += 1;
    if metadata.created.is_some() {
        meta_present += 1;
    }
    meta_fields += 1;
    if metadata.modified.is_some() {
        meta_present += 1;
    }
    meta_fields += 1;
    if metadata.total_editing_time_minutes.is_some() {
        meta_present += 1;
    }
    meta_fields += 1;
    if metadata.revision_count.is_some() {
        meta_present += 1;
    }

    score += 0.3 * (meta_present as f64 / meta_fields as f64);

    // RSID richness (0.4 weight)
    max_score += 0.4;
    if rsid.unique_rsid_count > 0 {
        // More sessions = more data = higher integrity
        let session_score = (rsid.unique_rsid_count as f64 / 10.0).min(1.0);
        score += 0.4 * session_score;
    }

    // Formatting baseline (0.3 weight)
    max_score += 0.3;
    if formatting.total_runs > 0 {
        let fmt_score: f64 = if formatting.baseline_font.is_some() { 0.5 } else { 0.0 }
            + if formatting.total_runs > 5 { 0.5 } else { 0.25 };
        score += 0.3 * fmt_score.min(1.0);
    }

    if max_score > 0.0 {
        (score / max_score).min(1.0)
    } else {
        0.0
    }
}

/// Generate a human-readable assessment text.
fn generate_assessment(
    metadata: &DocxMetadata,
    rsid: &RsidAnalysis,
    pattern: &ConstructionPattern,
    integrity_score: f64,
) -> String {
    let sessions = rsid.unique_rsid_count;
    let editing_time = metadata
        .total_editing_time_minutes
        .map(|m| {
            if m >= 60 {
                format!("{} hours and {} minutes", m / 60, m % 60)
            } else {
                format!("{m} minutes")
            }
        });

    let time_phrase = editing_time
        .as_deref()
        .unwrap_or("an unknown amount of");

    match pattern {
        ConstructionPattern::Organic => {
            format!(
                "This document was constructed across approximately {sessions} editing session(s) \
                 with {time_phrase} of active editing time. RSID analysis shows progressive, \
                 non-uniform construction with {:.0}% revision scatter, consistent with \
                 incremental human writing.",
                rsid.revision_scatter * 100.0
            )
        }
        ConstructionPattern::BulkInsertion => {
            format!(
                "This document shows characteristics consistent with bulk text insertion. \
                 Only {sessions} unique editing session(s) were detected across {} paragraphs, \
                 with the largest contiguous block spanning {} paragraphs. \
                 The editing time was {time_phrase}.",
                rsid.total_paragraphs, rsid.largest_block_size
            )
        }
        ConstructionPattern::Hybrid => {
            let paste_count = rsid.paste_events.len();
            format!(
                "This document shows a mixed construction pattern. While {sessions} editing \
                 sessions were detected, {paste_count} region(s) show characteristics consistent \
                 with bulk text insertion. The remaining content shows incremental construction. \
                 The editing time was {time_phrase}.",
            )
        }
        ConstructionPattern::Insufficient => {
            format!(
                "Insufficient process evidence is available to assess document construction. \
                 Process integrity score is {:.0}/100. This assessment should be interpreted \
                 with significant caution.",
                integrity_score * 100.0
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construction_pattern_display() {
        assert_eq!(format!("{:?}", ConstructionPattern::Organic), "Organic");
        assert_eq!(
            format!("{:?}", ConstructionPattern::BulkInsertion),
            "BulkInsertion"
        );
    }
}
