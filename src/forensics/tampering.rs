//! Tampering detection for document forensics.
//!
//! Detects signs of document manipulation including:
//! - Date/time inconsistencies between filesystem and embedded metadata
//! - Tool fingerprint analysis (identifying creating software)
//! - Hidden content detection (white text, very small fonts, etc.)

use serde::{Deserialize, Serialize};

use super::format::FormatInfo;
use super::metadata::FileMetadata;

/// Result of tampering analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TamperingReport {
    /// Overall risk score (0.0 = clean, 1.0 = highly suspicious)
    pub risk_score: f64,
    /// Individual findings
    pub findings: Vec<TamperingFinding>,
    /// Tool fingerprint analysis
    pub tool_fingerprint: Option<ToolFingerprint>,
}

/// A single tampering indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TamperingFinding {
    /// Severity: low, medium, high
    pub severity: Severity,
    /// Category of finding
    pub category: FindingCategory,
    /// Human-readable description
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingCategory {
    DateInconsistency,
    ToolAnomaly,
    HiddenContent,
    FormatMismatch,
    MetadataAnomaly,
}

/// Tool fingerprint identifying the software used to create/modify the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFingerprint {
    /// Primary tool name (e.g., "Microsoft Word", "LibreOffice")
    pub tool_name: String,
    /// Tool version (if detectable)
    pub tool_version: Option<String>,
    /// Platform (if detectable)
    pub platform: Option<String>,
    /// Confidence in the identification (0.0–1.0)
    pub confidence: f64,
}

/// Analyze a document for signs of tampering.
pub fn analyze(
    metadata: &FileMetadata,
    format: &FormatInfo,
    text: &str,
) -> TamperingReport {
    let mut findings = Vec::new();

    // 1. Date inconsistency checks
    check_date_inconsistencies(metadata, &mut findings);

    // 2. Format mismatch
    if !format.extension_matches {
        if let Some(ref warning) = format.mismatch_warning {
            findings.push(TamperingFinding {
                severity: Severity::High,
                category: FindingCategory::FormatMismatch,
                description: warning.clone(),
            });
        }
    }

    // 3. Hidden content detection
    check_hidden_content(text, &mut findings);

    // 4. Metadata anomalies
    check_metadata_anomalies(metadata, &mut findings);

    // 5. Tool fingerprint
    let tool_fingerprint = identify_tool(metadata);

    // Compute risk score
    let risk_score = compute_risk_score(&findings);

    TamperingReport {
        risk_score,
        findings,
        tool_fingerprint,
    }
}

/// Check for date/time inconsistencies.
fn check_date_inconsistencies(metadata: &FileMetadata, findings: &mut Vec<TamperingFinding>) {
    // Check: created date after modified date
    if let (Some(created), Some(modified)) = (metadata.created_epoch, metadata.modified_epoch) {
        if created > modified + 60 {
            // Allow 60s tolerance
            findings.push(TamperingFinding {
                severity: Severity::Medium,
                category: FindingCategory::DateInconsistency,
                description: format!(
                    "File creation date ({created}) is after modification date ({modified}). \
                     This can indicate the file was copied or the dates were manipulated."
                ),
            });
        }
    }

    // Check: document metadata dates vs filesystem dates
    if let Some(ref doc_meta) = metadata.document_metadata {
        if let (Some(ref doc_created), Some(fs_modified)) =
            (&doc_meta.creation_date, metadata.modified_epoch)
        {
            // Parse ISO date from document metadata
            if let Some(doc_epoch) = parse_date_approximate(doc_created) {
                // Document claims to be created after filesystem modification
                if doc_epoch > fs_modified + 86400 {
                    // 1 day tolerance
                    findings.push(TamperingFinding {
                        severity: Severity::Medium,
                        category: FindingCategory::DateInconsistency,
                        description: format!(
                            "Document internal creation date '{doc_created}' is after \
                             filesystem modification date. Metadata may have been altered."
                        ),
                    });
                }

                // Document claims to be created far in the future
                let now_approx = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if doc_epoch > now_approx + 86400 {
                    findings.push(TamperingFinding {
                        severity: Severity::High,
                        category: FindingCategory::DateInconsistency,
                        description: format!(
                            "Document creation date '{doc_created}' is in the future."
                        ),
                    });
                }
            }
        }
    }
}

/// Check for hidden content indicators.
fn check_hidden_content(text: &str, findings: &mut Vec<TamperingFinding>) {
    // Check for excessive whitespace that might hide content
    let chars: Vec<char> = text.chars().collect();
    if chars.len() > 100 {
        let invisible_count = chars
            .iter()
            .filter(|c| {
                c.is_whitespace()
                    && **c != ' '
                    && **c != '\n'
                    && **c != '\t'
                    && **c != '\r'
            })
            .count();

        let invisible_ratio = invisible_count as f64 / chars.len() as f64;
        if invisible_ratio > 0.05 {
            findings.push(TamperingFinding {
                severity: Severity::Medium,
                category: FindingCategory::HiddenContent,
                description: format!(
                    "Text contains {:.1}% unusual whitespace characters, \
                     which may indicate hidden content.",
                    invisible_ratio * 100.0
                ),
            });
        }
    }

    // Check for zero-width characters
    let zero_width_count = text
        .chars()
        .filter(|c| {
            matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{00AD}'
            )
        })
        .count();

    if zero_width_count > 3 {
        findings.push(TamperingFinding {
            severity: Severity::Medium,
            category: FindingCategory::HiddenContent,
            description: format!(
                "Text contains {zero_width_count} zero-width characters, \
                 which may be used for watermarking or to hide content."
            ),
        });
    }
}

/// Check for metadata anomalies.
fn check_metadata_anomalies(metadata: &FileMetadata, findings: &mut Vec<TamperingFinding>) {
    if let Some(ref doc_meta) = metadata.document_metadata {
        // Very high revision count for small document
        if let Some(revisions) = doc_meta.revision_count {
            if revisions > 100 && metadata.file_size < 50_000 {
                findings.push(TamperingFinding {
                    severity: Severity::Low,
                    category: FindingCategory::MetadataAnomaly,
                    description: format!(
                        "Document has {revisions} revisions but is only {} bytes. \
                         This is unusual and may indicate metadata manipulation.",
                        metadata.file_size
                    ),
                });
            }
        }

        // Word count mismatch (if reported)
        // This is a placeholder — actual comparison would need extracted text word count
    }
}

/// Identify the tool used to create/modify the document.
fn identify_tool(metadata: &FileMetadata) -> Option<ToolFingerprint> {
    let doc_meta = metadata.document_metadata.as_ref()?;

    let tool_str = doc_meta
        .creator_tool
        .as_deref()
        .or(doc_meta.producer.as_deref())?;

    let (name, version, platform) = parse_tool_string(tool_str);

    Some(ToolFingerprint {
        tool_name: name,
        tool_version: version,
        platform,
        confidence: 0.8,
    })
}

/// Parse tool identification string into components.
fn parse_tool_string(tool: &str) -> (String, Option<String>, Option<String>) {
    let tool_lower = tool.to_lowercase();

    // Microsoft Office
    if tool_lower.contains("microsoft") || tool_lower.contains("word") {
        let version = extract_version(tool);
        let platform = if tool_lower.contains("mac") {
            Some("macOS".to_string())
        } else if tool_lower.contains("windows") {
            Some("Windows".to_string())
        } else {
            None
        };
        return ("Microsoft Word".to_string(), version, platform);
    }

    // LibreOffice
    if tool_lower.contains("libreoffice") || tool_lower.contains("libre office") {
        return ("LibreOffice".to_string(), extract_version(tool), None);
    }

    // Google Docs
    if tool_lower.contains("google") {
        return ("Google Docs".to_string(), None, Some("Web".to_string()));
    }

    // LaTeX
    if tool_lower.contains("latex") || tool_lower.contains("pdftex") || tool_lower.contains("xetex") {
        return ("LaTeX".to_string(), extract_version(tool), None);
    }

    // Generic
    (tool.to_string(), extract_version(tool), None)
}

/// Extract version number from a tool string.
fn extract_version(s: &str) -> Option<String> {
    let re = regex::Regex::new(r"(\d+\.\d+(?:\.\d+)?)").ok()?;
    re.find(s).map(|m| m.as_str().to_string())
}

/// Parse a date string to approximate epoch seconds.
fn parse_date_approximate(date_str: &str) -> Option<u64> {
    // Handle ISO 8601 dates like "2024-01-15T10:30:00Z"
    // Also PDF date format: "D:20240115103000"
    let cleaned = date_str
        .trim_start_matches("D:")
        .replace('T', " ")
        .replace('Z', "");

    // Try to extract year-month-day
    let parts: Vec<&str> = cleaned.split(|c: char| !c.is_ascii_digit()).collect();
    if parts.len() >= 3 {
        let year: u64 = parts[0].parse().ok()?;
        let month: u64 = parts[1].parse().ok()?;
        let day: u64 = parts[2].parse().ok()?;

        if year < 1970 || year > 2100 || month < 1 || month > 12 || day < 1 || day > 31 {
            return None;
        }

        // Rough epoch calculation (ignoring leap years for approximation)
        let epoch = (year - 1970) * 365 * 86400
            + (month - 1) * 30 * 86400
            + (day - 1) * 86400;
        return Some(epoch);
    }

    // Handle YYYYMMDD format (PDF dates without separators)
    if cleaned.len() >= 8 {
        let year: u64 = cleaned[0..4].parse().ok()?;
        let month: u64 = cleaned[4..6].parse().ok()?;
        let day: u64 = cleaned[6..8].parse().ok()?;

        if year >= 1970 && year <= 2100 && month >= 1 && month <= 12 && day >= 1 && day <= 31 {
            let epoch = (year - 1970) * 365 * 86400
                + (month - 1) * 30 * 86400
                + (day - 1) * 86400;
            return Some(epoch);
        }
    }

    None
}

/// Compute overall risk score from findings.
fn compute_risk_score(findings: &[TamperingFinding]) -> f64 {
    if findings.is_empty() {
        return 0.0;
    }

    let mut score: f64 = 0.0;
    for f in findings {
        score += match f.severity {
            Severity::Low => 0.1,
            Severity::Medium => 0.3,
            Severity::High => 0.5,
        };
    }

    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_document() {
        let metadata = FileMetadata {
            file_name: "clean.txt".into(),
            file_size: 1000,
            created_epoch: Some(1700000000),
            modified_epoch: Some(1700001000),
            accessed_epoch: Some(1700002000),
            is_readonly: false,
            document_metadata: None,
        };

        let format = FormatInfo {
            extension: "txt".into(),
            detected_type: super::super::format::FileType::PlainText,
            encoding: "UTF-8".into(),
            magic_bytes: "".into(),
            extension_matches: true,
            mismatch_warning: None,
        };

        let report = analyze(&metadata, &format, "Normal text content here.");
        assert_eq!(report.risk_score, 0.0);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_date_inconsistency() {
        let metadata = FileMetadata {
            file_name: "suspicious.txt".into(),
            file_size: 500,
            created_epoch: Some(1700002000), // Created AFTER modified
            modified_epoch: Some(1700000000),
            accessed_epoch: Some(1700003000),
            is_readonly: false,
            document_metadata: None,
        };

        let format = FormatInfo {
            extension: "txt".into(),
            detected_type: super::super::format::FileType::PlainText,
            encoding: "UTF-8".into(),
            magic_bytes: "".into(),
            extension_matches: true,
            mismatch_warning: None,
        };

        let report = analyze(&metadata, &format, "content");
        assert!(report.risk_score > 0.0);
        assert!(report.findings.iter().any(|f| f.category == FindingCategory::DateInconsistency));
    }

    #[test]
    fn test_zero_width_detection() {
        let text_with_hidden = "Normal text\u{200B}\u{200B}\u{200B}\u{200B} more text";
        let metadata = FileMetadata {
            file_name: "test.txt".into(),
            file_size: 100,
            created_epoch: Some(1700000000),
            modified_epoch: Some(1700001000),
            accessed_epoch: None,
            is_readonly: false,
            document_metadata: None,
        };
        let format = FormatInfo {
            extension: "txt".into(),
            detected_type: super::super::format::FileType::PlainText,
            encoding: "UTF-8".into(),
            magic_bytes: "".into(),
            extension_matches: true,
            mismatch_warning: None,
        };

        let report = analyze(&metadata, &format, text_with_hidden);
        assert!(report.findings.iter().any(|f| f.category == FindingCategory::HiddenContent));
    }

    #[test]
    fn test_parse_date() {
        // ISO 8601
        assert!(parse_date_approximate("2024-01-15T10:30:00Z").is_some());
        // PDF format
        assert!(parse_date_approximate("D:20240115103000").is_some());
        // Invalid
        assert!(parse_date_approximate("not a date").is_none());
    }

    #[test]
    fn test_tool_identification() {
        let (name, version, _) = parse_tool_string("Microsoft Word 16.0.1");
        assert_eq!(name, "Microsoft Word");
        assert_eq!(version, Some("16.0.1".into()));

        let (name, _, platform) = parse_tool_string("Google Docs");
        assert_eq!(name, "Google Docs");
        assert_eq!(platform, Some("Web".into()));
    }

    #[test]
    fn test_risk_score() {
        assert_eq!(compute_risk_score(&[]), 0.0);

        let findings = vec![TamperingFinding {
            severity: Severity::High,
            category: FindingCategory::FormatMismatch,
            description: "test".into(),
        }];
        assert!((compute_risk_score(&findings) - 0.5).abs() < f64::EPSILON);
    }
}
