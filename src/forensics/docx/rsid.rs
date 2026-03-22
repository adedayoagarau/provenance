//! RSID (Revision Save ID) extraction and analysis for DOCX forensics.
//!
//! Every time a Word document is saved, a unique RSID is assigned to modified elements.
//! Analyzing RSID distribution reveals how a document was constructed — incrementally
//! (organic writing) or in bulk (paste from external source).

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RSID data for a single paragraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphRsid {
    pub paragraph_index: usize,
    /// RSID when paragraph was first added.
    pub rsid_r: Option<String>,
    /// Default RSID for runs in the paragraph.
    pub rsid_r_default: Option<String>,
    /// RSID when paragraph properties were last modified.
    pub rsid_p: Option<String>,
}

/// RSID data for a single run within a paragraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRsid {
    pub paragraph_index: usize,
    pub run_index: usize,
    /// RSID when run was created.
    pub rsid_r: Option<String>,
    /// RSID when run properties were modified.
    pub rsid_r_pr: Option<String>,
}

/// A contiguous block of paragraphs sharing the same RSID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsidBlock {
    pub rsid: String,
    pub start_paragraph: usize,
    pub end_paragraph: usize,
    pub block_length: usize,
}

/// A detected paste event (anomalous large single-rsid block).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasteEvent {
    pub start_paragraph: usize,
    pub end_paragraph: usize,
    pub rsid: String,
    pub block_size: usize,
    pub is_anomalous: bool,
}

/// An RSID-based anomaly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsidAnomaly {
    pub anomaly_type: RsidAnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub start_paragraph: Option<usize>,
    pub end_paragraph: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RsidAnomalyType {
    LowDiversity,
    LargeBlock,
    NoProgression,
    LowScatter,
    SessionCountMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
}

/// Complete RSID analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsidAnalysis {
    /// Per-paragraph RSID data.
    pub paragraph_rsids: Vec<ParagraphRsid>,
    /// Per-run RSID data.
    pub run_rsids: Vec<RunRsid>,
    /// All RSIDs from settings.xml.
    pub settings_rsids: Vec<String>,
    /// Maps RSID → paragraph indices.
    pub rsid_to_paragraphs: HashMap<String, Vec<usize>>,

    // Metrics
    /// unique_rsids / total_paragraphs
    pub rsid_diversity: f64,
    /// Number of unique RSIDs.
    pub unique_rsid_count: usize,
    /// Total paragraphs analyzed.
    pub total_paragraphs: usize,
    /// Contiguous blocks of same-RSID paragraphs, sorted by length desc.
    pub rsid_blocks: Vec<RsidBlock>,
    /// Largest contiguous block size.
    pub largest_block_size: usize,
    /// Spearman rank correlation of paragraph position vs RSID value.
    pub progression_correlation: Option<f64>,
    /// Percentage of paragraphs whose RSID differs from neighbors.
    pub revision_scatter: f64,
    /// Detected paste events.
    pub paste_events: Vec<PasteEvent>,
    /// Flagged anomalies.
    pub anomalies: Vec<RsidAnomaly>,
}

/// Extract RSIDs from document.xml content.
pub fn extract_rsids(document_xml: &str) -> (Vec<ParagraphRsid>, Vec<RunRsid>) {
    let mut paragraph_rsids = Vec::new();
    let mut run_rsids = Vec::new();

    let mut reader = Reader::from_str(document_xml);
    reader.config_mut().trim_text(true);

    let mut para_index: usize = 0;
    let mut run_index: usize = 0;
    let mut in_paragraph = false;
    let mut current_para_rsid = ParagraphRsid {
        paragraph_index: 0,
        rsid_r: None,
        rsid_r_default: None,
        rsid_p: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"p" => {
                        in_paragraph = true;
                        run_index = 0;
                        current_para_rsid = ParagraphRsid {
                            paragraph_index: para_index,
                            rsid_r: None,
                            rsid_r_default: None,
                            rsid_p: None,
                        };
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            let key = std::str::from_utf8(local.as_ref())
                                .unwrap_or_default();
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            match key {
                                "rsidR" => current_para_rsid.rsid_r = Some(val),
                                "rsidRDefault" => {
                                    current_para_rsid.rsid_r_default = Some(val)
                                }
                                "rsidP" => current_para_rsid.rsid_p = Some(val),
                                _ => {}
                            }
                        }
                    }
                    b"r" if in_paragraph => {
                        let mut rsid_r = None;
                        let mut rsid_r_pr = None;
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            let key = std::str::from_utf8(local.as_ref())
                                .unwrap_or_default();
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            match key {
                                "rsidR" => rsid_r = Some(val),
                                "rsidRPr" => rsid_r_pr = Some(val),
                                _ => {}
                            }
                        }
                        if rsid_r.is_some() || rsid_r_pr.is_some() {
                            run_rsids.push(RunRsid {
                                paragraph_index: para_index,
                                run_index,
                                rsid_r,
                                rsid_r_pr,
                            });
                        }
                        run_index += 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.local_name().as_ref() == b"p" && in_paragraph {
                    paragraph_rsids.push(current_para_rsid.clone());
                    para_index += 1;
                    in_paragraph = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    (paragraph_rsids, run_rsids)
}

/// Extract RSIDs from settings.xml (the <w:rsids> element).
pub fn extract_settings_rsids(settings_xml: &str) -> Vec<String> {
    let mut rsids = Vec::new();
    let mut reader = Reader::from_str(settings_xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                // Look for <w:rsid w:val="XXXXXXXX"/> or <w:rsidRoot w:val="..."/>
                if local_name.as_ref() == b"rsid" || local_name.as_ref() == b"rsidRoot" {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            if !rsids.contains(&val) {
                                rsids.push(val);
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    rsids
}

/// Perform full RSID analysis on document.xml and optional settings.xml.
pub fn analyze(document_xml: &str, settings_xml: Option<&str>) -> RsidAnalysis {
    let (paragraph_rsids, run_rsids) = extract_rsids(document_xml);
    let settings_rsids = settings_xml
        .map(|xml| extract_settings_rsids(xml))
        .unwrap_or_default();

    let total_paragraphs = paragraph_rsids.len();

    // Build RSID → paragraphs map (using the effective RSID for each paragraph)
    let mut rsid_to_paragraphs: HashMap<String, Vec<usize>> = HashMap::new();
    let effective_rsids: Vec<Option<&str>> = paragraph_rsids
        .iter()
        .map(|p| {
            p.rsid_r
                .as_deref()
                .or(p.rsid_r_default.as_deref())
        })
        .collect();

    for (i, rsid) in effective_rsids.iter().enumerate() {
        if let Some(r) = rsid {
            rsid_to_paragraphs
                .entry(r.to_string())
                .or_default()
                .push(i);
        }
    }

    let unique_rsid_count = rsid_to_paragraphs.len();
    let rsid_diversity = if total_paragraphs > 0 {
        unique_rsid_count as f64 / total_paragraphs as f64
    } else {
        0.0
    };

    // RSID clustering: find contiguous blocks
    let rsid_blocks = compute_rsid_blocks(&effective_rsids);
    let largest_block_size = rsid_blocks.first().map(|b| b.block_length).unwrap_or(0);

    // RSID progression: Spearman correlation
    let progression_correlation = compute_progression_correlation(&effective_rsids);

    // Revision scatter
    let revision_scatter = compute_revision_scatter(&effective_rsids);

    // Paste event detection
    let avg_block_size = if !rsid_blocks.is_empty() {
        rsid_blocks.iter().map(|b| b.block_length).sum::<usize>() as f64
            / rsid_blocks.len() as f64
    } else {
        0.0
    };

    let paste_events: Vec<PasteEvent> = rsid_blocks
        .iter()
        .filter(|b| b.block_length >= 5)
        .map(|b| {
            let is_anomalous =
                b.block_length as f64 > avg_block_size * 2.0 && b.block_length >= 10;
            PasteEvent {
                start_paragraph: b.start_paragraph,
                end_paragraph: b.end_paragraph,
                rsid: b.rsid.clone(),
                block_size: b.block_length,
                is_anomalous,
            }
        })
        .collect();

    // Anomaly detection
    let mut anomalies = Vec::new();

    if total_paragraphs >= 5 {
        // Low diversity
        if rsid_diversity < 0.05 {
            anomalies.push(RsidAnomaly {
                anomaly_type: RsidAnomalyType::LowDiversity,
                severity: AnomalySeverity::High,
                description: format!(
                    "Very low RSID diversity ({:.3}): {unique_rsid_count} unique RSIDs across \
                     {total_paragraphs} paragraphs suggests bulk document creation",
                    rsid_diversity
                ),
                start_paragraph: None,
                end_paragraph: None,
            });
        } else if rsid_diversity < 0.15 {
            anomalies.push(RsidAnomaly {
                anomaly_type: RsidAnomalyType::LowDiversity,
                severity: AnomalySeverity::Medium,
                description: format!(
                    "Low RSID diversity ({:.3}): document may have been created in very few \
                     editing sessions",
                    rsid_diversity
                ),
                start_paragraph: None,
                end_paragraph: None,
            });
        }

        // Large blocks
        if largest_block_size > 15 {
            let severity = if largest_block_size as f64 > total_paragraphs as f64 * 0.5 {
                AnomalySeverity::High
            } else {
                AnomalySeverity::Medium
            };
            if let Some(block) = rsid_blocks.first() {
                anomalies.push(RsidAnomaly {
                    anomaly_type: RsidAnomalyType::LargeBlock,
                    severity,
                    description: format!(
                        "Large contiguous block of {largest_block_size} paragraphs \
                         (paragraphs {}-{}) share a single RSID, suggesting bulk text insertion",
                        block.start_paragraph, block.end_paragraph
                    ),
                    start_paragraph: Some(block.start_paragraph),
                    end_paragraph: Some(block.end_paragraph),
                });
            }
        }

        // Low scatter with low diversity = untouched bulk paste
        if revision_scatter < 0.10 && rsid_diversity < 0.10 {
            anomalies.push(RsidAnomaly {
                anomaly_type: RsidAnomalyType::LowScatter,
                severity: AnomalySeverity::High,
                description: format!(
                    "Very low revision scatter ({:.1}%) combined with low RSID diversity \
                     suggests document was created in a single session without revision",
                    revision_scatter * 100.0
                ),
                start_paragraph: None,
                end_paragraph: None,
            });
        }
    }

    // Session count vs settings rsids mismatch
    if !settings_rsids.is_empty() && unique_rsid_count > 0 {
        let ratio = unique_rsid_count as f64 / settings_rsids.len() as f64;
        if ratio < 0.3 || ratio > 3.0 {
            anomalies.push(RsidAnomaly {
                anomaly_type: RsidAnomalyType::SessionCountMismatch,
                severity: AnomalySeverity::Low,
                description: format!(
                    "Mismatch between document RSIDs ({unique_rsid_count}) and settings RSIDs \
                     ({}): unusual editing pattern or metadata inconsistency",
                    settings_rsids.len()
                ),
                start_paragraph: None,
                end_paragraph: None,
            });
        }
    }

    RsidAnalysis {
        paragraph_rsids,
        run_rsids,
        settings_rsids,
        rsid_to_paragraphs,
        rsid_diversity,
        unique_rsid_count,
        total_paragraphs,
        rsid_blocks,
        largest_block_size,
        progression_correlation,
        revision_scatter,
        paste_events,
        anomalies,
    }
}

/// Compute contiguous blocks of paragraphs sharing the same RSID.
/// Returns blocks sorted by length descending.
fn compute_rsid_blocks(effective_rsids: &[Option<&str>]) -> Vec<RsidBlock> {
    let mut blocks = Vec::new();

    if effective_rsids.is_empty() {
        return blocks;
    }

    let mut current_rsid: Option<&str> = None;
    let mut block_start: usize = 0;

    for (i, rsid) in effective_rsids.iter().enumerate() {
        match (current_rsid, rsid) {
            (Some(curr), Some(next)) if curr == *next => {
                // Continue current block
            }
            _ => {
                // Close previous block
                if let Some(curr) = current_rsid {
                    let len = i - block_start;
                    if len > 0 {
                        blocks.push(RsidBlock {
                            rsid: curr.to_string(),
                            start_paragraph: block_start,
                            end_paragraph: i - 1,
                            block_length: len,
                        });
                    }
                }
                current_rsid = rsid.as_deref();
                block_start = i;
            }
        }
    }

    // Close final block
    if let Some(curr) = current_rsid {
        let len = effective_rsids.len() - block_start;
        if len > 0 {
            blocks.push(RsidBlock {
                rsid: curr.to_string(),
                start_paragraph: block_start,
                end_paragraph: effective_rsids.len() - 1,
                block_length: len,
            });
        }
    }

    blocks.sort_by(|a, b| b.block_length.cmp(&a.block_length));
    blocks
}

/// Compute Spearman rank correlation between paragraph position and RSID numeric value.
fn compute_progression_correlation(effective_rsids: &[Option<&str>]) -> Option<f64> {
    let pairs: Vec<(f64, f64)> = effective_rsids
        .iter()
        .enumerate()
        .filter_map(|(i, rsid)| {
            rsid.and_then(|r| {
                u64::from_str_radix(r, 16)
                    .ok()
                    .map(|v| (i as f64, v as f64))
            })
        })
        .collect();

    if pairs.len() < 3 {
        return None;
    }

    // Convert to ranks
    let n = pairs.len();
    let pos_ranks = rank_values(&pairs.iter().map(|(p, _)| *p).collect::<Vec<_>>());
    let val_ranks = rank_values(&pairs.iter().map(|(_, v)| *v).collect::<Vec<_>>());

    // Spearman: 1 - (6 * sum(d^2)) / (n * (n^2 - 1))
    let mut sum_d_sq = 0.0;
    for i in 0..n {
        let d = pos_ranks[i] - val_ranks[i];
        sum_d_sq += d * d;
    }

    let n_f = n as f64;
    let rho = 1.0 - (6.0 * sum_d_sq) / (n_f * (n_f * n_f - 1.0));
    Some(rho)
}

/// Assign ranks to values (average rank for ties).
fn rank_values(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut indexed: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && (indexed[j].1 - indexed[i].1).abs() < f64::EPSILON {
            j += 1;
        }
        let avg_rank = (i + j + 1) as f64 / 2.0;
        for k in i..j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j;
    }

    ranks
}

/// Compute revision scatter: fraction of paragraphs whose RSID differs from at least one neighbor.
fn compute_revision_scatter(effective_rsids: &[Option<&str>]) -> f64 {
    if effective_rsids.len() < 2 {
        return 0.0;
    }

    let mut differ_count = 0;
    for i in 0..effective_rsids.len() {
        let current = effective_rsids[i];
        let differs = if i > 0 && current != effective_rsids[i - 1] {
            true
        } else if i + 1 < effective_rsids.len() && current != effective_rsids[i + 1] {
            true
        } else {
            false
        };
        if differs {
            differ_count += 1;
        }
    }

    differ_count as f64 / effective_rsids.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_document_xml(paragraphs: &[(Option<&str>, Option<&str>)]) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>"#,
        );
        for (rsid_r, rsid_default) in paragraphs {
            xml.push_str("<w:p");
            if let Some(r) = rsid_r {
                xml.push_str(&format!(r#" w:rsidR="{r}""#));
            }
            if let Some(d) = rsid_default {
                xml.push_str(&format!(r#" w:rsidRDefault="{d}""#));
            }
            xml.push_str("><w:r><w:t>Text</w:t></w:r></w:p>");
        }
        xml.push_str("</w:body></w:document>");
        xml
    }

    #[test]
    fn test_rsid_extraction_basic() {
        let xml = make_document_xml(&[
            (Some("00A11111"), Some("00A11111")),
            (Some("00B22222"), Some("00B22222")),
            (Some("00C33333"), Some("00C33333")),
        ]);
        let (paras, _runs) = extract_rsids(&xml);
        assert_eq!(paras.len(), 3);
        assert_eq!(paras[0].rsid_r.as_deref(), Some("00A11111"));
        assert_eq!(paras[1].rsid_r.as_deref(), Some("00B22222"));
        assert_eq!(paras[2].rsid_r.as_deref(), Some("00C33333"));
    }

    #[test]
    fn test_rsid_diversity_high() {
        // Each paragraph has unique RSID = high diversity
        let xml = make_document_xml(&[
            (Some("00A11111"), None),
            (Some("00B22222"), None),
            (Some("00C33333"), None),
            (Some("00D44444"), None),
            (Some("00E55555"), None),
        ]);
        let analysis = analyze(&xml, None);
        assert_eq!(analysis.unique_rsid_count, 5);
        assert!((analysis.rsid_diversity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rsid_diversity_low_bulk() {
        // All paragraphs share one RSID = bulk paste
        let xml = make_document_xml(&[
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
        ]);
        let analysis = analyze(&xml, None);
        assert_eq!(analysis.unique_rsid_count, 1);
        assert!(analysis.rsid_diversity < 0.15);
        assert_eq!(analysis.largest_block_size, 10);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == RsidAnomalyType::LowDiversity));
    }

    #[test]
    fn test_rsid_clustering() {
        let xml = make_document_xml(&[
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00B22222"), None),
            (Some("00C33333"), None),
        ]);
        let analysis = analyze(&xml, None);
        // Largest block should be the first 3 paragraphs
        assert_eq!(analysis.rsid_blocks[0].block_length, 3);
        assert_eq!(analysis.rsid_blocks[0].rsid, "00A11111");
        assert_eq!(analysis.rsid_blocks[0].start_paragraph, 0);
        assert_eq!(analysis.rsid_blocks[0].end_paragraph, 2);
    }

    #[test]
    fn test_revision_scatter_high() {
        // Alternating RSIDs = high scatter
        let xml = make_document_xml(&[
            (Some("00A11111"), None),
            (Some("00B22222"), None),
            (Some("00A11111"), None),
            (Some("00B22222"), None),
            (Some("00A11111"), None),
        ]);
        let analysis = analyze(&xml, None);
        assert!(analysis.revision_scatter > 0.8);
    }

    #[test]
    fn test_revision_scatter_low() {
        // All same = zero scatter
        let xml = make_document_xml(&[
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
            (Some("00A11111"), None),
        ]);
        let analysis = analyze(&xml, None);
        assert!(analysis.revision_scatter < 0.01);
    }

    #[test]
    fn test_settings_rsids_extraction() {
        let settings = r#"<?xml version="1.0"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:rsids>
    <w:rsidRoot w:val="00A11111"/>
    <w:rsid w:val="00B22222"/>
    <w:rsid w:val="00C33333"/>
    <w:rsid w:val="00D44444"/>
  </w:rsids>
</w:settings>"#;
        let rsids = extract_settings_rsids(settings);
        assert_eq!(rsids.len(), 4);
        assert!(rsids.contains(&"00A11111".to_string()));
        assert!(rsids.contains(&"00D44444".to_string()));
    }

    #[test]
    fn test_paste_event_detection() {
        // 20 paragraphs with same RSID surrounded by different ones
        let mut paras: Vec<(Option<&str>, Option<&str>)> = Vec::new();
        paras.push((Some("00A11111"), None));
        paras.push((Some("00A11111"), None));
        for _ in 0..20 {
            paras.push((Some("00B22222"), None));
        }
        paras.push((Some("00C33333"), None));
        paras.push((Some("00C33333"), None));

        let xml = make_document_xml(&paras);
        let analysis = analyze(&xml, None);

        // Should detect the 20-paragraph block as a paste event
        assert!(!analysis.paste_events.is_empty());
        let paste = analysis
            .paste_events
            .iter()
            .find(|p| p.rsid == "00B22222")
            .expect("Should find paste event for 00B22222");
        assert_eq!(paste.block_size, 20);
        assert!(paste.is_anomalous);
    }

    #[test]
    fn test_progression_correlation() {
        // Sequential RSIDs should give positive correlation
        let xml = make_document_xml(&[
            (Some("00000001"), None),
            (Some("00000002"), None),
            (Some("00000003"), None),
            (Some("00000004"), None),
            (Some("00000005"), None),
        ]);
        let analysis = analyze(&xml, None);
        assert!(analysis.progression_correlation.unwrap() > 0.9);
    }
}
