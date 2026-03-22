//! Structural analysis for DOCX forensics.
//!
//! Analyzes paragraph and section length distributions to detect patterns
//! consistent with AI-generated content (uniform paragraph lengths) vs.
//! organic human writing (high variance).

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

/// Structural forensics results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralForensics {
    /// Per-paragraph word counts in document order.
    pub paragraph_word_counts: Vec<usize>,
    /// Total paragraphs (including empty).
    pub total_paragraphs: usize,
    /// Non-empty paragraphs.
    pub content_paragraphs: usize,
    /// Mean paragraph word count (non-empty paragraphs).
    pub mean_paragraph_length: f64,
    /// Median paragraph word count.
    pub median_paragraph_length: f64,
    /// Standard deviation of paragraph word counts.
    pub paragraph_length_stddev: f64,
    /// Coefficient of variation (stddev/mean) — higher = more varied = more human-like.
    pub paragraph_length_cv: f64,
    /// Per-section word counts (text between headings).
    pub section_word_counts: Vec<usize>,
    /// Coefficient of variation for section lengths.
    pub section_length_cv: f64,
    /// Count of empty paragraphs.
    pub empty_paragraph_count: usize,
    /// Indices of consecutive empty paragraph runs (potential paste boundaries).
    pub empty_paragraph_runs: Vec<(usize, usize)>,
}

/// Extract paragraph texts and heading markers from document.xml.
fn extract_paragraphs(document_xml: &str) -> Vec<ParagraphInfo> {
    let mut paragraphs = Vec::new();
    let mut reader = Reader::from_str(document_xml);
    reader.config_mut().trim_text(true);

    let mut in_paragraph = false;
    let mut in_run = false;
    let mut in_ppr = false;
    let mut current_text = String::new();
    let mut is_heading = false;
    let mut in_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        // Self-closing <w:p/> = empty paragraph
                        paragraphs.push(ParagraphInfo {
                            text: String::new(),
                            is_heading: false,
                        });
                    }
                    b"pStyle" if in_ppr => {
                        for attr in e.attributes().flatten() {
                            let local_attr = attr.key.local_name();
                            if local_attr.as_ref() == b"val" {
                                let val = String::from_utf8_lossy(&attr.value);
                                if val.starts_with("Heading") || val.starts_with("heading") {
                                    is_heading = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        in_paragraph = true;
                        current_text.clear();
                        is_heading = false;
                    }
                    b"pPr" if in_paragraph => {
                        in_ppr = true;
                    }
                    b"pStyle" if in_ppr => {
                        for attr in e.attributes().flatten() {
                            let local_attr = attr.key.local_name();
                            if local_attr.as_ref() == b"val" {
                                let val = String::from_utf8_lossy(&attr.value);
                                if val.starts_with("Heading") || val.starts_with("heading") {
                                    is_heading = true;
                                }
                            }
                        }
                    }
                    b"r" if in_paragraph => {
                        in_run = true;
                    }
                    b"t" if in_run => {
                        in_text = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    if let Ok(text) = e.unescape() {
                        current_text.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                match e.local_name().as_ref() {
                    b"p" => {
                        paragraphs.push(ParagraphInfo {
                            text: current_text.clone(),
                            is_heading,
                        });
                        in_paragraph = false;
                        in_ppr = false;
                    }
                    b"pPr" => in_ppr = false,
                    b"r" => in_run = false,
                    b"t" => in_text = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    paragraphs
}

struct ParagraphInfo {
    text: String,
    is_heading: bool,
}

/// Perform structural analysis on document.xml.
pub fn analyze(document_xml: &str) -> StructuralForensics {
    let paragraphs = extract_paragraphs(document_xml);
    let total_paragraphs = paragraphs.len();

    let paragraph_word_counts: Vec<usize> = paragraphs
        .iter()
        .map(|p| count_words(&p.text))
        .collect();

    let non_empty: Vec<f64> = paragraph_word_counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| c as f64)
        .collect();

    let content_paragraphs = non_empty.len();
    let empty_paragraph_count = total_paragraphs - content_paragraphs;

    let mean_paragraph_length = if non_empty.is_empty() {
        0.0
    } else {
        non_empty.iter().sum::<f64>() / non_empty.len() as f64
    };

    let median_paragraph_length = if non_empty.is_empty() {
        0.0
    } else {
        let mut sorted = non_empty.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    };

    let paragraph_length_stddev = if non_empty.len() < 2 {
        0.0
    } else {
        let variance: f64 = non_empty
            .iter()
            .map(|x| (x - mean_paragraph_length).powi(2))
            .sum::<f64>()
            / (non_empty.len() - 1) as f64;
        variance.sqrt()
    };

    let paragraph_length_cv = if mean_paragraph_length > 0.0 {
        paragraph_length_stddev / mean_paragraph_length
    } else {
        0.0
    };

    // Section analysis (split at headings)
    let mut section_word_counts = Vec::new();
    let mut current_section_words = 0usize;

    for (i, para) in paragraphs.iter().enumerate() {
        if para.is_heading && current_section_words > 0 {
            section_word_counts.push(current_section_words);
            current_section_words = 0;
        }
        current_section_words += paragraph_word_counts[i];
    }
    if current_section_words > 0 {
        section_word_counts.push(current_section_words);
    }

    let section_length_cv = compute_cv(&section_word_counts);

    // Empty paragraph runs
    let empty_paragraph_runs = find_empty_runs(&paragraph_word_counts);

    StructuralForensics {
        paragraph_word_counts,
        total_paragraphs,
        content_paragraphs,
        mean_paragraph_length,
        median_paragraph_length,
        paragraph_length_stddev,
        paragraph_length_cv,
        section_word_counts,
        section_length_cv,
        empty_paragraph_count,
        empty_paragraph_runs,
    }
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn compute_cv(counts: &[usize]) -> f64 {
    let non_zero: Vec<f64> = counts.iter().filter(|&&c| c > 0).map(|&c| c as f64).collect();
    if non_zero.len() < 2 {
        return 0.0;
    }
    let mean = non_zero.iter().sum::<f64>() / non_zero.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = non_zero
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / (non_zero.len() - 1) as f64;
    variance.sqrt() / mean
}

/// Find runs of consecutive empty paragraphs.
fn find_empty_runs(word_counts: &[usize]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < word_counts.len() {
        if word_counts[i] == 0 {
            let start = i;
            while i < word_counts.len() && word_counts[i] == 0 {
                i += 1;
            }
            let len = i - start;
            if len >= 2 {
                runs.push((start, i - 1));
            }
        } else {
            i += 1;
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(paragraphs: &[&str]) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>"#,
        );
        for text in paragraphs {
            if text.is_empty() {
                xml.push_str("<w:p/>");
            } else {
                xml.push_str(&format!(
                    "<w:p><w:r><w:t>{text}</w:t></w:r></w:p>"
                ));
            }
        }
        xml.push_str("</w:body></w:document>");
        xml
    }

    #[test]
    fn test_paragraph_word_counts() {
        let doc = make_doc(&["Hello world", "One two three four", "Single"]);
        let result = analyze(&doc);
        assert_eq!(result.paragraph_word_counts, vec![2, 4, 1]);
        assert_eq!(result.content_paragraphs, 3);
        assert_eq!(result.total_paragraphs, 3);
    }

    #[test]
    fn test_high_variance_human_like() {
        // Very different paragraph lengths = human-like
        let doc = make_doc(&[
            "Short.",
            "This is a much longer paragraph with many words that demonstrates human-style varying length in natural writing patterns and diverse thought development.",
            "Medium sized paragraph here.",
            "Another very long paragraph that goes on and on because humans sometimes write extended thoughts when they get into a flow state and keep developing ideas.",
            "Tiny.",
        ]);
        let result = analyze(&doc);
        assert!(result.paragraph_length_cv > 0.5);
    }

    #[test]
    fn test_low_variance_ai_like() {
        // Similar paragraph lengths = AI-like
        let doc = make_doc(&[
            "One two three four five six seven eight.",
            "Alpha beta gamma delta epsilon zeta eta theta.",
            "Words here form a sentence of medium length now.",
            "Another line with roughly the same word count here.",
        ]);
        let result = analyze(&doc);
        // CV should be relatively low for uniform lengths
        assert!(result.paragraph_length_cv < 0.5);
    }

    #[test]
    fn test_empty_paragraph_detection() {
        let doc = make_doc(&["Text", "", "", "More text", "", "", "", "End"]);
        let result = analyze(&doc);
        assert!(!result.empty_paragraph_runs.is_empty());
        // First run: indices 1-2, second run: indices 4-6
        assert_eq!(result.empty_paragraph_runs[0], (1, 2));
        assert_eq!(result.empty_paragraph_runs[1], (4, 6));
    }
}
