//! Formatting consistency analysis for DOCX forensics.
//!
//! When text is pasted from an external source, it often carries formatting artifacts
//! that differ from the document's baseline style. This module detects those inconsistencies.

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

/// Baseline formatting established from the document's default styles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingBaseline {
    pub default_font: Option<String>,
    pub default_font_size: Option<u32>,
    pub default_line_spacing: Option<u32>,
    pub default_para_spacing_before: Option<u32>,
    pub default_para_spacing_after: Option<u32>,
    pub default_language: Option<String>,
}

/// Formatting properties of a single run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunFormatting {
    pub paragraph_index: usize,
    pub run_index: usize,
    pub font: Option<String>,
    pub font_size: Option<u32>,
    pub language: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub color: Option<String>,
}

/// A formatting anomaly region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingAnomaly {
    pub start_paragraph: usize,
    pub end_paragraph: usize,
    pub anomaly_type: FormattingAnomalyType,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormattingAnomalyType {
    NonBaselineFont,
    NonBaselineFontSize,
    InconsistentLanguage,
    InlineOverrides,
}

/// Complete formatting analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingAnalysis {
    pub baseline: FormattingBaseline,
    pub run_count: usize,
    pub runs_matching_baseline: usize,
    pub formatting_consistency_score: f64,
    pub anomalies: Vec<FormattingAnomaly>,
}

/// Extract default formatting baseline from styles.xml.
pub fn extract_baseline(styles_xml: &str) -> FormattingBaseline {
    let mut baseline = FormattingBaseline {
        default_font: None,
        default_font_size: None,
        default_line_spacing: None,
        default_para_spacing_before: None,
        default_para_spacing_after: None,
        default_language: None,
    };

    let mut reader = Reader::from_str(styles_xml);
    reader.config_mut().trim_text(true);

    let mut in_doc_defaults = false;
    let mut in_rpr_default = false;
    let mut in_ppr_default = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"docDefaults" => in_doc_defaults = true,
                    b"rPrDefault" => {
                        if in_doc_defaults {
                            in_rpr_default = true;
                        }
                    }
                    b"pPrDefault" => {
                        if in_doc_defaults {
                            in_ppr_default = true;
                        }
                    }
                    b"rFonts" if in_rpr_default => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            let key = std::str::from_utf8(local.as_ref())
                                .unwrap_or_default();
                            if key == "ascii" || key == "hAnsi" {
                                if baseline.default_font.is_none() {
                                    baseline.default_font = Some(
                                        String::from_utf8_lossy(&attr.value).to_string(),
                                    );
                                }
                            }
                        }
                    }
                    b"sz" if in_rpr_default => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            if local.as_ref() == b"val" {
                                baseline.default_font_size = String::from_utf8_lossy(&attr.value)
                                    .parse()
                                    .ok();
                            }
                        }
                    }
                    b"lang" if in_rpr_default => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            if local.as_ref() == b"val" {
                                baseline.default_language = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    b"spacing" if in_ppr_default => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            let key = std::str::from_utf8(local.as_ref())
                                .unwrap_or_default();
                            let val: Option<u32> =
                                String::from_utf8_lossy(&attr.value).parse().ok();
                            match key {
                                "line" => baseline.default_line_spacing = val,
                                "before" => baseline.default_para_spacing_before = val,
                                "after" => baseline.default_para_spacing_after = val,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.local_name().as_ref() {
                    b"docDefaults" => in_doc_defaults = false,
                    b"rPrDefault" => in_rpr_default = false,
                    b"pPrDefault" => in_ppr_default = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    baseline
}

/// Extract per-run formatting from document.xml.
pub fn extract_run_formatting(document_xml: &str) -> Vec<RunFormatting> {
    let mut runs = Vec::new();
    let mut reader = Reader::from_str(document_xml);
    reader.config_mut().trim_text(true);

    let mut para_index: usize = 0;
    let mut run_index: usize = 0;
    let mut in_paragraph = false;
    let mut in_run = false;
    let mut in_rpr = false;
    let mut current_run = RunFormatting {
        paragraph_index: 0,
        run_index: 0,
        font: None,
        font_size: None,
        language: None,
        bold: false,
        italic: false,
        color: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        if in_paragraph {
                            para_index += 1;
                        }
                        in_paragraph = true;
                        run_index = 0;
                    }
                    b"r" if in_paragraph => {
                        in_run = true;
                        current_run = RunFormatting {
                            paragraph_index: para_index,
                            run_index,
                            font: None,
                            font_size: None,
                            language: None,
                            bold: false,
                            italic: false,
                            color: None,
                        };
                    }
                    b"rPr" if in_run => {
                        in_rpr = true;
                    }
                    b"rFonts" if in_rpr => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            let key = std::str::from_utf8(local.as_ref())
                                .unwrap_or_default();
                            if key == "ascii" || key == "hAnsi" {
                                if current_run.font.is_none() {
                                    current_run.font = Some(
                                        String::from_utf8_lossy(&attr.value).to_string(),
                                    );
                                }
                            }
                        }
                    }
                    b"sz" if in_rpr => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            if local.as_ref() == b"val" {
                                current_run.font_size =
                                    String::from_utf8_lossy(&attr.value).parse().ok();
                            }
                        }
                    }
                    b"b" if in_rpr => {
                        current_run.bold = true;
                    }
                    b"i" if in_rpr => {
                        current_run.italic = true;
                    }
                    b"color" if in_rpr => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            if local.as_ref() == b"val" {
                                current_run.color = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    b"lang" if in_rpr => {
                        for attr in e.attributes().flatten() {
                            let local = attr.key.local_name();
                            if local.as_ref() == b"val" {
                                current_run.language = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.local_name().as_ref() {
                    b"p" => {
                        in_paragraph = false;
                        para_index += 1;
                    }
                    b"r" => {
                        if in_run {
                            runs.push(current_run.clone());
                            run_index += 1;
                            in_run = false;
                        }
                    }
                    b"rPr" => in_rpr = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    runs
}

/// Perform full formatting analysis.
pub fn analyze(
    document_xml: &str,
    styles_xml: Option<&str>,
) -> FormattingAnalysis {
    let baseline = styles_xml
        .map(|xml| extract_baseline(xml))
        .unwrap_or(FormattingBaseline {
            default_font: None,
            default_font_size: None,
            default_line_spacing: None,
            default_para_spacing_before: None,
            default_para_spacing_after: None,
            default_language: None,
        });

    let runs = extract_run_formatting(document_xml);
    let run_count = runs.len();

    if run_count == 0 {
        return FormattingAnalysis {
            baseline,
            run_count: 0,
            runs_matching_baseline: 0,
            formatting_consistency_score: 1.0,
            anomalies: Vec::new(),
        };
    }

    // If no baseline from styles, infer from most common values
    let effective_font = baseline.default_font.clone().or_else(|| {
        most_common_value(runs.iter().filter_map(|r| r.font.as_ref()))
    });
    let effective_size = baseline.default_font_size.or_else(|| {
        most_common_u32(runs.iter().filter_map(|r| r.font_size))
    });

    // Count runs matching baseline
    let mut matching = 0;
    let mut non_matching_runs: Vec<&RunFormatting> = Vec::new();

    for run in &runs {
        let font_ok = match (&run.font, &effective_font) {
            (Some(f), Some(b)) => f == b,
            (None, _) => true, // No override = uses baseline
            _ => true,
        };
        let size_ok = match (run.font_size, effective_size) {
            (Some(f), Some(b)) => f == b,
            (None, _) => true,
            _ => true,
        };

        if font_ok && size_ok {
            matching += 1;
        } else {
            non_matching_runs.push(run);
        }
    }

    let formatting_consistency_score = matching as f64 / run_count as f64;

    // Find anomaly clusters (contiguous non-baseline runs)
    let anomalies = find_anomaly_clusters(&non_matching_runs, &effective_font, effective_size);

    FormattingAnalysis {
        baseline: FormattingBaseline {
            default_font: effective_font,
            default_font_size: effective_size,
            ..baseline
        },
        run_count,
        runs_matching_baseline: matching,
        formatting_consistency_score,
        anomalies,
    }
}

fn find_anomaly_clusters(
    non_matching: &[&RunFormatting],
    baseline_font: &Option<String>,
    baseline_size: Option<u32>,
) -> Vec<FormattingAnomaly> {
    if non_matching.is_empty() {
        return Vec::new();
    }

    let mut anomalies = Vec::new();
    let mut i = 0;

    while i < non_matching.len() {
        let start_para = non_matching[i].paragraph_index;
        let mut end_para = start_para;
        let mut j = i + 1;

        // Extend cluster while paragraphs are contiguous or close
        while j < non_matching.len() && non_matching[j].paragraph_index <= end_para + 2 {
            end_para = non_matching[j].paragraph_index;
            j += 1;
        }

        // Determine anomaly type
        let has_font_diff = non_matching[i..j].iter().any(|r| {
            r.font.is_some() && r.font != *baseline_font
        });
        let has_size_diff = non_matching[i..j].iter().any(|r| {
            r.font_size.is_some() && r.font_size != baseline_size
        });

        let (anomaly_type, details) = if has_font_diff && has_size_diff {
            let fonts: Vec<_> = non_matching[i..j]
                .iter()
                .filter_map(|r| r.font.as_ref())
                .collect();
            (
                FormattingAnomalyType::NonBaselineFont,
                format!(
                    "Paragraphs {start_para}-{end_para} contain non-baseline font ({}) \
                     and font size, differing from document default ({})",
                    fonts.first().map(|f| f.as_str()).unwrap_or("unknown"),
                    baseline_font.as_deref().unwrap_or("unknown")
                ),
            )
        } else if has_font_diff {
            (
                FormattingAnomalyType::NonBaselineFont,
                format!(
                    "Paragraphs {start_para}-{end_para} use a different font than the document baseline"
                ),
            )
        } else if has_size_diff {
            (
                FormattingAnomalyType::NonBaselineFontSize,
                format!(
                    "Paragraphs {start_para}-{end_para} use a different font size than the document baseline"
                ),
            )
        } else {
            (
                FormattingAnomalyType::InlineOverrides,
                format!(
                    "Paragraphs {start_para}-{end_para} contain inline formatting overrides"
                ),
            )
        };

        anomalies.push(FormattingAnomaly {
            start_paragraph: start_para,
            end_paragraph: end_para,
            anomaly_type,
            details,
        });

        i = j;
    }

    anomalies
}

fn most_common_value<'a>(iter: impl Iterator<Item = &'a String>) -> Option<String> {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for v in iter {
        *counts.entry(v.as_str()).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(v, _)| v.to_string())
}

fn most_common_u32(iter: impl Iterator<Item = u32>) -> Option<u32> {
    let mut counts: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for v in iter {
        *counts.entry(v).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_extraction() {
        let styles = r#"<?xml version="1.0"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/>
        <w:sz w:val="22"/>
        <w:lang w:val="en-US"/>
      </w:rPr>
    </w:rPrDefault>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="160" w:line="259"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
</w:styles>"#;
        let baseline = extract_baseline(styles);
        assert_eq!(baseline.default_font.as_deref(), Some("Calibri"));
        assert_eq!(baseline.default_font_size, Some(22));
        assert_eq!(baseline.default_language.as_deref(), Some("en-US"));
        assert_eq!(baseline.default_para_spacing_after, Some(160));
        assert_eq!(baseline.default_line_spacing, Some(259));
    }

    #[test]
    fn test_formatting_consistency_clean() {
        // All runs use baseline formatting (no overrides)
        let doc = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:r><w:t>Paragraph one.</w:t></w:r></w:p>
  <w:p><w:r><w:t>Paragraph two.</w:t></w:r></w:p>
  <w:p><w:r><w:t>Paragraph three.</w:t></w:r></w:p>
</w:body></w:document>"#;
        let analysis = analyze(doc, None);
        assert!((analysis.formatting_consistency_score - 1.0).abs() < f64::EPSILON);
        assert!(analysis.anomalies.is_empty());
    }

    #[test]
    fn test_formatting_consistency_with_paste_artifacts() {
        let styles = r#"<?xml version="1.0"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr><w:rFonts w:ascii="Calibri"/><w:sz w:val="22"/></w:rPr>
    </w:rPrDefault>
  </w:docDefaults>
</w:styles>"#;

        let doc = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
  <w:p><w:r><w:t>Normal text.</w:t></w:r></w:p>
  <w:p><w:r><w:rPr><w:rFonts w:ascii="Times New Roman"/><w:sz w:val="24"/></w:rPr><w:t>Pasted text.</w:t></w:r></w:p>
  <w:p><w:r><w:rPr><w:rFonts w:ascii="Times New Roman"/><w:sz w:val="24"/></w:rPr><w:t>More pasted text.</w:t></w:r></w:p>
  <w:p><w:r><w:t>Normal again.</w:t></w:r></w:p>
</w:body></w:document>"#;

        let analysis = analyze(doc, Some(styles));
        assert!(analysis.formatting_consistency_score < 1.0);
        assert!(!analysis.anomalies.is_empty());
    }
}
