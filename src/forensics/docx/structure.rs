use super::parser::ParsedParagraph;
use serde::{Deserialize, Serialize};

/// Structural forensic analysis of a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralForensics {
    /// Total paragraphs in document
    pub total_paragraphs: usize,
    /// Paragraph word counts in order
    pub paragraph_word_counts: Vec<usize>,
    /// Mean paragraph length (words)
    pub mean_paragraph_length: f64,
    /// Median paragraph length (words)
    pub median_paragraph_length: f64,
    /// Standard deviation of paragraph lengths
    pub stddev_paragraph_length: f64,
    /// Coefficient of variation (stddev / mean)
    pub coefficient_of_variation: f64,
    /// Number of empty paragraphs
    pub empty_paragraph_count: usize,
    /// Runs of consecutive empty paragraphs: (start_index, length)
    pub empty_paragraph_runs: Vec<(usize, usize)>,
    /// Section word counts (text between headings/long gaps)
    pub section_word_counts: Vec<usize>,
    /// Section balance score: stddev of section lengths / mean (lower = more uniform)
    pub section_balance_score: f64,
    /// Whether paragraph length distribution appears suspiciously uniform
    pub uniform_paragraph_flag: bool,
    /// Whether section lengths appear suspiciously balanced
    pub uniform_section_flag: bool,
}

impl StructuralForensics {
    /// Analyze the structural properties of parsed paragraphs.
    pub fn analyze(paragraphs: &[ParsedParagraph]) -> Self {
        let paragraph_word_counts: Vec<usize> = paragraphs
            .iter()
            .map(|p| p.word_count())
            .collect();

        let non_empty: Vec<f64> = paragraph_word_counts
            .iter()
            .filter(|&&c| c > 0)
            .map(|&c| c as f64)
            .collect();

        let total_paragraphs = paragraphs.len();
        let empty_paragraph_count = paragraph_word_counts.iter().filter(|&&c| c == 0).count();

        let (mean, median, stddev, cv) = if non_empty.is_empty() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let mean = non_empty.iter().sum::<f64>() / non_empty.len() as f64;
            let median = compute_median(&non_empty);
            let variance = non_empty.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / non_empty.len() as f64;
            let stddev = variance.sqrt();
            let cv = if mean > 0.0 { stddev / mean } else { 0.0 };
            (mean, median, stddev, cv)
        };

        // Detect runs of empty paragraphs
        let empty_paragraph_runs = detect_empty_runs(&paragraph_word_counts);

        // Compute section word counts
        let section_word_counts = compute_section_counts(&paragraph_word_counts);
        let section_balance_score = compute_balance_score(&section_word_counts);

        // Flag suspiciously uniform paragraph lengths
        // AI-generated text tends to have CV < 0.3 (very uniform)
        let uniform_paragraph_flag = cv < 0.3 && non_empty.len() > 10;

        // Flag suspiciously balanced sections
        let uniform_section_flag = section_balance_score < 0.25
            && section_word_counts.len() > 3
            && section_word_counts.iter().all(|&c| c > 50);

        StructuralForensics {
            total_paragraphs,
            paragraph_word_counts,
            mean_paragraph_length: mean,
            median_paragraph_length: median,
            stddev_paragraph_length: stddev,
            coefficient_of_variation: cv,
            empty_paragraph_count,
            empty_paragraph_runs,
            section_word_counts,
            section_balance_score,
            uniform_paragraph_flag,
            uniform_section_flag,
        }
    }
}

fn compute_median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Detect runs of consecutive empty paragraphs.
fn detect_empty_runs(word_counts: &[usize]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < word_counts.len() {
        if word_counts[i] == 0 {
            let start = i;
            while i < word_counts.len() && word_counts[i] == 0 {
                i += 1;
            }
            let length = i - start;
            if length >= 2 {
                runs.push((start, length));
            }
        } else {
            i += 1;
        }
    }
    runs
}

/// Compute section word counts by splitting at empty-paragraph boundaries
/// or every ~5 paragraphs if no clear section breaks.
fn compute_section_counts(word_counts: &[usize]) -> Vec<usize> {
    let mut sections = Vec::new();
    let mut current_section_words = 0usize;
    let mut consecutive_empty = 0usize;

    for &count in word_counts {
        if count == 0 {
            consecutive_empty += 1;
            if consecutive_empty >= 2 && current_section_words > 0 {
                sections.push(current_section_words);
                current_section_words = 0;
            }
        } else {
            consecutive_empty = 0;
            current_section_words += count;
        }
    }

    if current_section_words > 0 {
        sections.push(current_section_words);
    }

    // If we only got one section, split by rough paragraph groupings
    if sections.len() <= 1 && word_counts.len() > 10 {
        sections.clear();
        let chunk_size = (word_counts.len() / 4).max(3);
        for chunk in word_counts.chunks(chunk_size) {
            let sum: usize = chunk.iter().sum();
            if sum > 0 {
                sections.push(sum);
            }
        }
    }

    sections
}

/// Compute balance score: CV of section word counts.
fn compute_balance_score(section_counts: &[usize]) -> f64 {
    if section_counts.len() < 2 {
        return 0.0;
    }
    let vals: Vec<f64> = section_counts.iter().map(|&c| c as f64).collect();
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    variance.sqrt() / mean
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::docx::parser::{ParsedParagraph, ParsedRun};

    fn make_para_with_words(word_count: usize) -> ParsedParagraph {
        let text = if word_count == 0 {
            String::new()
        } else {
            (0..word_count).map(|_| "word").collect::<Vec<_>>().join(" ")
        };
        ParsedParagraph {
            runs: vec![ParsedRun {
                text,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_structural_basic() {
        let paragraphs = vec![
            make_para_with_words(50),
            make_para_with_words(30),
            make_para_with_words(0),
            make_para_with_words(45),
            make_para_with_words(60),
        ];
        let analysis = StructuralForensics::analyze(&paragraphs);
        assert_eq!(analysis.total_paragraphs, 5);
        assert_eq!(analysis.empty_paragraph_count, 1);
        assert!(analysis.mean_paragraph_length > 0.0);
    }

    #[test]
    fn test_uniform_paragraph_detection() {
        // Very uniform paragraph lengths (AI-like)
        let paragraphs: Vec<ParsedParagraph> = (0..20)
            .map(|_| make_para_with_words(50))
            .collect();
        let analysis = StructuralForensics::analyze(&paragraphs);
        assert!(analysis.coefficient_of_variation < 0.3);
        assert!(analysis.uniform_paragraph_flag);
    }

    #[test]
    fn test_varied_paragraph_lengths() {
        // Varied paragraph lengths (human-like)
        let lengths = [10, 45, 120, 8, 67, 200, 15, 90, 5, 150, 30, 75];
        let paragraphs: Vec<ParsedParagraph> = lengths
            .iter()
            .map(|&n| make_para_with_words(n))
            .collect();
        let analysis = StructuralForensics::analyze(&paragraphs);
        assert!(analysis.coefficient_of_variation > 0.3);
        assert!(!analysis.uniform_paragraph_flag);
    }

    #[test]
    fn test_empty_paragraph_runs() {
        let paragraphs = vec![
            make_para_with_words(50),
            make_para_with_words(0),
            make_para_with_words(0),
            make_para_with_words(0),
            make_para_with_words(50),
        ];
        let analysis = StructuralForensics::analyze(&paragraphs);
        assert_eq!(analysis.empty_paragraph_runs.len(), 1);
        assert_eq!(analysis.empty_paragraph_runs[0], (1, 3));
    }
}
