use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResult;
use crate::forensics::ForensicReport;
use crate::identity::comparison::ComparisonResult;

/// Unified score combining all analysis layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub forensic_report: ForensicReport,
    pub analysis_result: AnalysisResult,
    pub comparison: Option<ComparisonResult>,
    pub overall_confidence: Option<f64>,
}

/// Aggregate scores from all layers into a unified result.
pub fn score(
    forensic_report: &ForensicReport,
    analysis_result: &AnalysisResult,
    comparison: Option<&ComparisonResult>,
) -> UnifiedScore {
    let overall_confidence = comparison.map(|c| c.confidence.value);

    UnifiedScore {
        forensic_report: forensic_report.clone(),
        analysis_result: analysis_result.clone(),
        comparison: comparison.cloned(),
        overall_confidence,
    }
}
