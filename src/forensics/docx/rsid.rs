use super::parser::{self, ParsedParagraph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RSID data for a single paragraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphRsid {
    pub paragraph_index: usize,
    pub rsid_r: Option<String>,
    pub rsid_r_default: Option<String>,
    pub rsid_p: Option<String>,
}

/// RSID data for a single run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRsid {
    pub paragraph_index: usize,
    pub run_index: usize,
    pub rsid_r: Option<String>,
    pub rsid_r_pr: Option<String>,
}

/// A contiguous block of paragraphs sharing the same rsid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsidBlock {
    pub rsid: String,
    pub start_paragraph: usize,
    pub end_paragraph: usize,
    pub block_length: usize,
}

/// Detected paste event (anomalous rsid block).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasteEvent {
    pub start_paragraph: usize,
    pub end_paragraph: usize,
    pub rsid: String,
    pub block_size: usize,
    pub is_anomalous: bool,
}

/// An anomaly detected during RSID analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsidAnomaly {
    pub anomaly_type: String,
    pub severity: AnomalySeverity,
    pub description: String,
    pub start_paragraph: Option<usize>,
    pub end_paragraph: Option<usize>,
}

/// Severity levels for anomalies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
}

/// Complete RSID analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsidAnalysis {
    /// Ordered list of per-paragraph RSID data
    pub paragraph_rsids: Vec<ParagraphRsid>,
    /// Ordered list of per-run RSID data
    pub run_rsids: Vec<RunRsid>,
    /// Map from rsid value to paragraph indices where it appears
    pub rsid_to_paragraphs: HashMap<String, Vec<usize>>,
    /// All rsids from settings.xml
    pub settings_rsids: Vec<String>,

    // Metrics
    /// unique_rsids / total_paragraphs
    pub rsid_diversity: f64,
    /// Number of unique rsid values
    pub unique_rsid_count: usize,
    /// Total paragraphs analyzed
    pub total_paragraphs: usize,
    /// Contiguous blocks of paragraphs sharing the same rsid
    pub rsid_blocks: Vec<RsidBlock>,
    /// Largest single-rsid contiguous block
    pub largest_block_size: usize,
    /// Spearman rank correlation between paragraph position and rsid value
    pub rsid_progression_correlation: Option<f64>,
    /// Percentage of paragraphs whose rsid differs from neighbors
    pub revision_scatter: f64,
    /// Detected paste events
    pub paste_events: Vec<PasteEvent>,
    /// Detected anomalies
    pub anomalies: Vec<RsidAnomaly>,
}

impl RsidAnalysis {
    /// Perform full RSID analysis from parsed paragraphs and settings.
    pub fn analyze(
        paragraphs: &[ParsedParagraph],
        settings_xml: Option<&str>,
    ) -> Self {
        let settings_rsids = settings_xml
            .map(|xml| parser::extract_settings_rsids(xml))
            .unwrap_or_default();

        // Build paragraph and run rsid lists
        let mut paragraph_rsids = Vec::new();
        let mut run_rsids = Vec::new();
        let mut rsid_to_paragraphs: HashMap<String, Vec<usize>> = HashMap::new();

        for (para_idx, para) in paragraphs.iter().enumerate() {
            let p_rsid = ParagraphRsid {
                paragraph_index: para_idx,
                rsid_r: para.rsid_r.clone(),
                rsid_r_default: para.rsid_r_default.clone(),
                rsid_p: para.rsid_p.clone(),
            };

            if let Some(ref rsid) = para.rsid_r {
                rsid_to_paragraphs
                    .entry(rsid.clone())
                    .or_default()
                    .push(para_idx);
            }

            paragraph_rsids.push(p_rsid);

            for (run_idx, run) in para.runs.iter().enumerate() {
                run_rsids.push(RunRsid {
                    paragraph_index: para_idx,
                    run_index: run_idx,
                    rsid_r: run.rsid_r.clone(),
                    rsid_r_pr: run.rsid_r_pr.clone(),
                });
            }
        }

        let total_paragraphs = paragraphs.len();
        let unique_rsid_count = rsid_to_paragraphs.len();

        // RSID diversity
        let rsid_diversity = if total_paragraphs > 0 {
            unique_rsid_count as f64 / total_paragraphs as f64
        } else {
            0.0
        };

        // RSID clustering — find contiguous blocks
        let rsid_blocks = compute_rsid_blocks(paragraphs);
        let largest_block_size = rsid_blocks.iter().map(|b| b.block_length).max().unwrap_or(0);

        // RSID progression correlation
        let rsid_progression_correlation = compute_rsid_progression(paragraphs);

        // Revision scatter
        let revision_scatter = compute_revision_scatter(paragraphs);

        // Paste event detection
        let avg_block_size = if !rsid_blocks.is_empty() {
            rsid_blocks.iter().map(|b| b.block_length).sum::<usize>() as f64
                / rsid_blocks.len() as f64
        } else {
            0.0
        };

        let paste_events = detect_paste_events(&rsid_blocks, avg_block_size, paragraphs);

        // Generate anomalies
        let anomalies = generate_anomalies(
            rsid_diversity,
            largest_block_size,
            total_paragraphs,
            &paste_events,
            revision_scatter,
            unique_rsid_count,
            &settings_rsids,
        );

        RsidAnalysis {
            paragraph_rsids,
            run_rsids,
            rsid_to_paragraphs,
            settings_rsids,
            rsid_diversity,
            unique_rsid_count,
            total_paragraphs,
            rsid_blocks,
            largest_block_size,
            rsid_progression_correlation,
            revision_scatter,
            paste_events,
            anomalies,
        }
    }
}

/// Find contiguous blocks of paragraphs sharing the same rsid_r.
fn compute_rsid_blocks(paragraphs: &[ParsedParagraph]) -> Vec<RsidBlock> {
    let mut blocks = Vec::new();
    if paragraphs.is_empty() {
        return blocks;
    }

    let mut current_rsid: Option<&str> = None;
    let mut block_start = 0;

    for (i, para) in paragraphs.iter().enumerate() {
        let rsid = para.rsid_r.as_deref();
        match (current_rsid, rsid) {
            (Some(cur), Some(new)) if cur == new => {
                // Continue current block
            }
            (_, new_rsid) => {
                // End previous block if it had an rsid
                if let Some(cur) = current_rsid {
                    blocks.push(RsidBlock {
                        rsid: cur.to_string(),
                        start_paragraph: block_start,
                        end_paragraph: i - 1,
                        block_length: i - block_start,
                    });
                }
                current_rsid = new_rsid;
                block_start = i;
            }
        }
    }

    // Push final block
    if let Some(cur) = current_rsid {
        blocks.push(RsidBlock {
            rsid: cur.to_string(),
            start_paragraph: block_start,
            end_paragraph: paragraphs.len() - 1,
            block_length: paragraphs.len() - block_start,
        });
    }

    blocks
}

/// Compute Spearman rank correlation between paragraph position and rsid value.
fn compute_rsid_progression(paragraphs: &[ParsedParagraph]) -> Option<f64> {
    let values: Vec<(usize, u64)> = paragraphs
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            p.rsid_r.as_ref().and_then(|rsid| {
                u64::from_str_radix(rsid.trim_start_matches("0x").trim_start_matches("0X"), 16)
                    .ok()
                    .map(|v| (i, v))
            })
        })
        .collect();

    if values.len() < 3 {
        return None;
    }

    // Compute ranks for positions (already in order: 0, 1, 2, ...)
    let n = values.len() as f64;
    let position_ranks: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();

    // Compute ranks for rsid values
    let mut rsid_indexed: Vec<(usize, u64)> = values.iter().map(|&(i, v)| (i, v)).collect();
    rsid_indexed.sort_by_key(|&(_, v)| v);
    let mut rsid_ranks = vec![0.0f64; values.len()];
    for (rank, &(orig_idx, _)) in rsid_indexed.iter().enumerate() {
        // Find position in original values
        if let Some(pos) = values.iter().position(|&(i, _)| i == orig_idx) {
            rsid_ranks[pos] = rank as f64;
        }
    }

    // Spearman correlation = 1 - (6 * sum(d^2)) / (n * (n^2 - 1))
    let sum_d_squared: f64 = position_ranks
        .iter()
        .zip(rsid_ranks.iter())
        .map(|(p, r)| (p - r).powi(2))
        .sum();

    let denominator = n * (n * n - 1.0);
    if denominator == 0.0 {
        return None;
    }

    Some(1.0 - (6.0 * sum_d_squared) / denominator)
}

/// Compute revision scatter: percentage of paragraphs whose rsid differs from neighbors.
fn compute_revision_scatter(paragraphs: &[ParsedParagraph]) -> f64 {
    if paragraphs.len() <= 1 {
        return 0.0;
    }

    let mut different_count = 0usize;
    for i in 0..paragraphs.len() {
        let current = paragraphs[i].rsid_r.as_deref();
        let prev = if i > 0 {
            paragraphs[i - 1].rsid_r.as_deref()
        } else {
            None
        };
        let next = if i + 1 < paragraphs.len() {
            paragraphs[i + 1].rsid_r.as_deref()
        } else {
            None
        };

        let differs_from_prev = match (current, prev) {
            (Some(c), Some(p)) => c != p,
            _ => true,
        };
        let differs_from_next = match (current, next) {
            (Some(c), Some(n)) => c != n,
            _ => true,
        };

        if differs_from_prev || differs_from_next {
            different_count += 1;
        }
    }

    different_count as f64 / paragraphs.len() as f64
}

/// Detect probable paste events from rsid blocks.
fn detect_paste_events(
    blocks: &[RsidBlock],
    avg_block_size: f64,
    paragraphs: &[ParsedParagraph],
) -> Vec<PasteEvent> {
    let threshold = (avg_block_size * 3.0).max(15.0) as usize;
    let mut events = Vec::new();

    for block in blocks {
        let is_anomalous = block.block_length >= threshold;

        // Check if the block differs from surrounding paragraphs
        let differs_from_surroundings = if block.start_paragraph > 0 {
            let prev_rsid = paragraphs[block.start_paragraph - 1].rsid_r.as_deref();
            prev_rsid != Some(&block.rsid)
        } else {
            true
        };

        if is_anomalous && differs_from_surroundings {
            events.push(PasteEvent {
                start_paragraph: block.start_paragraph,
                end_paragraph: block.end_paragraph,
                rsid: block.rsid.clone(),
                block_size: block.block_length,
                is_anomalous,
            });
        }
    }

    events
}

/// Generate anomaly reports from RSID analysis metrics.
fn generate_anomalies(
    rsid_diversity: f64,
    largest_block_size: usize,
    total_paragraphs: usize,
    paste_events: &[PasteEvent],
    revision_scatter: f64,
    unique_rsid_count: usize,
    settings_rsids: &[String],
) -> Vec<RsidAnomaly> {
    let mut anomalies = Vec::new();

    // Low diversity anomaly
    if rsid_diversity < 0.05 && total_paragraphs > 5 {
        anomalies.push(RsidAnomaly {
            anomaly_type: "LowRsidDiversity".to_string(),
            severity: AnomalySeverity::High,
            description: format!(
                "RSID diversity is very low ({rsid_diversity:.3}), with only {unique_rsid_count} unique editing session(s) \
                 across {total_paragraphs} paragraphs. This pattern is consistent with bulk text insertion."
            ),
            start_paragraph: None,
            end_paragraph: None,
        });
    } else if rsid_diversity < 0.15 && total_paragraphs > 10 {
        anomalies.push(RsidAnomaly {
            anomaly_type: "LowRsidDiversity".to_string(),
            severity: AnomalySeverity::Medium,
            description: format!(
                "RSID diversity is below typical range ({rsid_diversity:.3}), with {unique_rsid_count} unique editing session(s) \
                 across {total_paragraphs} paragraphs."
            ),
            start_paragraph: None,
            end_paragraph: None,
        });
    }

    // Large block anomaly
    if largest_block_size > 15 {
        let severity = if largest_block_size as f64 > total_paragraphs as f64 * 0.5 {
            AnomalySeverity::High
        } else {
            AnomalySeverity::Medium
        };
        anomalies.push(RsidAnomaly {
            anomaly_type: "LargeRsidBlock".to_string(),
            severity,
            description: format!(
                "The largest contiguous block of paragraphs sharing a single editing session ID \
                 spans {largest_block_size} paragraphs. Blocks larger than 15 paragraphs may indicate \
                 bulk text insertion."
            ),
            start_paragraph: None,
            end_paragraph: None,
        });
    }

    // Paste event anomalies
    for event in paste_events {
        let severity = if event.block_size > 20 {
            AnomalySeverity::High
        } else {
            AnomalySeverity::Medium
        };
        anomalies.push(RsidAnomaly {
            anomaly_type: "PasteEvent".to_string(),
            severity,
            description: format!(
                "Paragraphs {}-{} share a single editing session ID with no subsequent revision, \
                 suggesting bulk text insertion ({} paragraphs).",
                event.start_paragraph, event.end_paragraph, event.block_size
            ),
            start_paragraph: Some(event.start_paragraph),
            end_paragraph: Some(event.end_paragraph),
        });
    }

    // Low revision scatter with uniform rsid
    if revision_scatter < 0.10 && rsid_diversity < 0.05 && total_paragraphs > 5 {
        anomalies.push(RsidAnomaly {
            anomaly_type: "UniformConstruction".to_string(),
            severity: AnomalySeverity::High,
            description: format!(
                "Revision scatter is very low ({:.1}%) with uniform editing session IDs. \
                 This pattern is consistent with a document created in a single operation.",
                revision_scatter * 100.0
            ),
            start_paragraph: None,
            end_paragraph: None,
        });
    }

    // Discrepancy between settings rsids and document rsids
    if !settings_rsids.is_empty() {
        let settings_count = settings_rsids.len();
        if unique_rsid_count > 0 {
            let ratio = settings_count as f64 / unique_rsid_count as f64;
            if ratio > 3.0 || ratio < 0.3 {
                anomalies.push(RsidAnomaly {
                    anomaly_type: "RsidCountDiscrepancy".to_string(),
                    severity: AnomalySeverity::Low,
                    description: format!(
                        "Settings.xml reports {settings_count} editing sessions while document.xml \
                         contains {unique_rsid_count} unique session IDs. This discrepancy may indicate \
                         an unusual editing pattern or metadata inconsistency."
                    ),
                    start_paragraph: None,
                    end_paragraph: None,
                });
            }
        }
    }

    anomalies
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::docx::parser::ParsedParagraph;

    fn make_para(rsid: &str) -> ParsedParagraph {
        ParsedParagraph {
            rsid_r: Some(rsid.to_string()),
            rsid_r_default: None,
            rsid_p: None,
            runs: vec![],
        }
    }

    #[test]
    fn test_rsid_diversity_high() {
        let paragraphs: Vec<ParsedParagraph> = (0..10)
            .map(|i| make_para(&format!("{:08X}", i * 1000)))
            .collect();
        let analysis = RsidAnalysis::analyze(&paragraphs, None);
        assert!(analysis.rsid_diversity >= 0.3, "diversity = {}", analysis.rsid_diversity);
    }

    #[test]
    fn test_rsid_diversity_low() {
        let paragraphs: Vec<ParsedParagraph> = (0..20)
            .map(|_| make_para("00112233"))
            .collect();
        let analysis = RsidAnalysis::analyze(&paragraphs, None);
        assert!(analysis.rsid_diversity < 0.1);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == "LowRsidDiversity"));
    }

    #[test]
    fn test_rsid_blocks() {
        let mut paragraphs = Vec::new();
        for _ in 0..5 {
            paragraphs.push(make_para("AAAA1111"));
        }
        for _ in 0..3 {
            paragraphs.push(make_para("BBBB2222"));
        }
        for _ in 0..2 {
            paragraphs.push(make_para("CCCC3333"));
        }
        let blocks = compute_rsid_blocks(&paragraphs);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_length, 5);
        assert_eq!(blocks[1].block_length, 3);
        assert_eq!(blocks[2].block_length, 2);
    }

    #[test]
    fn test_revision_scatter_low() {
        let paragraphs: Vec<ParsedParagraph> = (0..10)
            .map(|_| make_para("SAME_RSID"))
            .collect();
        let scatter = compute_revision_scatter(&paragraphs);
        // Only first and last differ from one neighbor
        assert!(scatter < 0.3);
    }

    #[test]
    fn test_revision_scatter_high() {
        let paragraphs: Vec<ParsedParagraph> = (0..10)
            .map(|i| make_para(&format!("{:08X}", i)))
            .collect();
        let scatter = compute_revision_scatter(&paragraphs);
        assert!(scatter > 0.5);
    }
}
