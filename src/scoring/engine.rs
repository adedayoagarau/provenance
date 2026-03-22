use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResult;
use crate::analysis::register::RegisterClassification;
use crate::analysis::baselines::RegisterBaselineReport;
use crate::forensics::ForensicReport;
use crate::identity::comparison::ComparisonResult;
use crate::identity::explainability::ExplainedDecision;
use crate::scoring::acs::AuthorshipConfidenceScore;
use crate::scoring::pii::ProcessIntegrityIndex;
use crate::scoring::anomalies::AnomalyReport;
use crate::scoring::content_design::EvaluatorReport;

/// Unified score combining all analysis layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub forensic_report: ForensicReport,
    pub analysis_result: AnalysisResult,
    pub comparison: Option<ComparisonResult>,
    pub overall_confidence: Option<f64>,
    pub audit: AuditTrail,
    /// Register classification for the analyzed text.
    pub register: Option<RegisterClassification>,
    /// Register baseline comparison.
    pub baseline_report: Option<RegisterBaselineReport>,
    /// Authorship Confidence Score.
    pub acs: Option<AuthorshipConfidenceScore>,
    /// Process Integrity Index.
    pub pii: Option<ProcessIntegrityIndex>,
    /// Anomaly flags.
    pub anomaly_report: Option<AnomalyReport>,
    /// Evaluator-facing report content.
    pub evaluator_report: Option<EvaluatorReport>,
    /// Feature importance explanation (when comparison is available).
    pub explained_decision: Option<ExplainedDecision>,
}

/// Audit trail for reproducibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    /// Software version
    pub software_version: String,
    /// Input file SHA-256 hash
    pub input_file_hash: String,
    /// Input file path
    pub input_file_path: String,
    /// Analysis timestamp (ISO 8601)
    pub timestamp: String,
    /// Configuration used (serialized)
    pub config_summary: String,
    /// Feature set used
    pub feature_set: String,
    /// Number of features extracted
    pub feature_count: usize,
}

impl AuditTrail {
    /// Create an audit trail from analysis context.
    pub fn new(file_path: &str, file_hash: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            input_file_hash: file_hash.to_string(),
            input_file_path: file_path.to_string(),
            timestamp: format_epoch(now),
            config_summary: "default".to_string(),
            feature_set: "standard".to_string(),
            feature_count: 0,
        }
    }
}

/// Format epoch seconds to ISO 8601 (approximate, no chrono dep).
fn format_epoch(secs: u64) -> String {
    // Simple UTC date formatting without chrono
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Calculate year/month/day from days since epoch
    let (year, month, day) = days_to_date(days);

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Simplified date calculation
    let mut remaining = days as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let months = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for &m_days in &months {
        if remaining < m_days {
            break;
        }
        remaining -= m_days;
        month += 1;
    }

    (year as u64, month, remaining as u64 + 1)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Aggregate scores from all layers into a unified result.
pub fn score(
    forensic_report: &ForensicReport,
    analysis_result: &AnalysisResult,
    comparison: Option<&ComparisonResult>,
) -> UnifiedScore {
    let overall_confidence = comparison.map(|c| c.confidence.value);

    let audit = AuditTrail::new(
        &forensic_report.metadata.file_name,
        &forensic_report.integrity.sha256,
    );

    UnifiedScore {
        forensic_report: forensic_report.clone(),
        analysis_result: analysis_result.clone(),
        comparison: comparison.cloned(),
        overall_confidence,
        audit,
        register: None,
        baseline_report: None,
        acs: None,
        pii: None,
        anomaly_report: None,
        evaluator_report: None,
        explained_decision: None,
    }
}

/// Aggregate scores from all layers, including Phase 13-17 components.
pub fn score_full(
    forensic_report: &ForensicReport,
    analysis_result: &AnalysisResult,
    comparison: Option<&ComparisonResult>,
    register: RegisterClassification,
    baseline_report: RegisterBaselineReport,
    acs: AuthorshipConfidenceScore,
    pii_score: ProcessIntegrityIndex,
    anomaly_report: AnomalyReport,
) -> UnifiedScore {
    let overall_confidence = comparison.map(|c| c.confidence.value);

    let audit = AuditTrail::new(
        &forensic_report.metadata.file_name,
        &forensic_report.integrity.sha256,
    );

    let evaluator_report = crate::scoring::content_design::generate(
        &acs,
        &pii_score,
        &anomaly_report,
        &register,
    );

    // Generate explained decision when comparison is available
    let explained_decision = comparison.map(|c| {
        crate::identity::explainability::explain(c, analysis_result)
    });

    UnifiedScore {
        forensic_report: forensic_report.clone(),
        analysis_result: analysis_result.clone(),
        comparison: comparison.cloned(),
        overall_confidence,
        audit,
        register: Some(register),
        baseline_report: Some(baseline_report),
        acs: Some(acs),
        pii: Some(pii_score),
        anomaly_report: Some(anomaly_report),
        evaluator_report: Some(evaluator_report),
        explained_decision,
    }
}

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Html,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "html" => Ok(Self::Html),
            _ => Err(format!("Unknown format '{s}'. Use: text, json, html")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_epoch() {
        let s = format_epoch(0);
        assert_eq!(s, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_format_epoch_recent() {
        // 2024-01-15 roughly
        let s = format_epoch(1705276800);
        assert!(s.starts_with("2024-01-"));
    }

    #[test]
    fn test_output_format_parse() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("html".parse::<OutputFormat>().unwrap(), OutputFormat::Html);
        assert!("xml".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_audit_trail() {
        let audit = AuditTrail::new("test.txt", "abc123");
        assert_eq!(audit.input_file_path, "test.txt");
        assert_eq!(audit.input_file_hash, "abc123");
        assert!(!audit.timestamp.is_empty());
        assert_eq!(audit.software_version, env!("CARGO_PKG_VERSION"));
    }
}
