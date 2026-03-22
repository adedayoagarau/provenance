pub mod parser;
pub mod metadata;
pub mod rsid;
pub mod formatting;
pub mod structure;
pub mod profile;

use crate::utils::errors::Result;
use crate::scoring::engine::OutputFormat;
use std::path::Path;

pub use profile::DocumentConstructionProfile;

/// Run full DOCX forensic analysis and return a DocumentConstructionProfile.
pub fn analyze_docx(path: &Path) -> Result<DocumentConstructionProfile> {
    // Step 1: Extract all XML parts from the DOCX ZIP archive
    let contents = parser::extract_archive_contents(path)?;

    // Step 2: Extract metadata
    let docx_metadata = metadata::DocxMetadata::extract(&contents);

    // Step 3: Parse document paragraphs for RSID and formatting analysis
    let paragraphs = contents
        .document_xml
        .as_deref()
        .map(parser::parse_document_paragraphs)
        .unwrap_or_default();

    // Step 4: RSID analysis
    let rsid_analysis = rsid::RsidAnalysis::analyze(
        &paragraphs,
        contents.settings_xml.as_deref(),
    );

    // Step 5: Parse style defaults for formatting baseline
    let style_defaults = contents
        .styles_xml
        .as_deref()
        .map(parser::parse_style_defaults)
        .unwrap_or_default();

    // Step 6: Formatting consistency analysis
    let formatting_analysis = formatting::FormattingAnalysis::analyze(&paragraphs, &style_defaults);

    // Step 7: Structural analysis
    let structural_forensics = structure::StructuralForensics::analyze(&paragraphs);

    // Step 8: Build construction profile
    let profile = DocumentConstructionProfile::build(
        docx_metadata,
        rsid_analysis,
        formatting_analysis,
        structural_forensics,
    );

    Ok(profile)
}

/// Run DOCX forensic analysis and render in the requested format.
pub fn analyze_docx_formatted(path: &Path, format: OutputFormat) -> Result<String> {
    let profile = analyze_docx(path)?;

    match format {
        OutputFormat::Text => Ok(profile.render_text()),
        OutputFormat::Json => profile.render_json().map_err(|e| {
            crate::utils::errors::ProvenanceError::AnalysisError {
                reason: format!("JSON serialization failed: {e}"),
            }
        }),
        OutputFormat::Html => Ok(render_html(&profile)),
    }
}

/// Render profile as a standalone HTML report.
fn render_html(profile: &DocumentConstructionProfile) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html><head>\n");
    html.push_str("<meta charset='utf-8'>\n");
    html.push_str("<title>Provenance — Document Construction Profile</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; max-width: 800px; margin: 40px auto; padding: 0 20px; color: #333; }\n");
    html.push_str("h1 { color: #1a1a1a; border-bottom: 2px solid #333; padding-bottom: 10px; }\n");
    html.push_str("h2 { color: #444; margin-top: 30px; }\n");
    html.push_str("table { border-collapse: collapse; width: 100%; margin: 10px 0; }\n");
    html.push_str("td, th { padding: 8px 12px; border: 1px solid #ddd; text-align: left; }\n");
    html.push_str("th { background: #f5f5f5; }\n");
    html.push_str(".score { font-size: 1.4em; font-weight: bold; }\n");
    html.push_str(".anomaly { background: #fff3cd; padding: 10px; margin: 5px 0; border-left: 4px solid #ffc107; }\n");
    html.push_str(".anomaly.high { border-left-color: #dc3545; background: #f8d7da; }\n");
    html.push_str(".assessment { background: #e8f4fd; padding: 15px; border-radius: 5px; margin: 15px 0; }\n");
    html.push_str("</style>\n</head><body>\n");

    html.push_str("<h1>Document Construction Profile</h1>\n");

    // Metadata
    html.push_str("<h2>Document Metadata</h2>\n<table>\n");
    if let Some(ref author) = profile.metadata.author {
        html.push_str(&format!("<tr><th>Author</th><td>{author}</td></tr>\n"));
    }
    if let Some(ref app) = profile.metadata.application {
        html.push_str(&format!("<tr><th>Application</th><td>{app}</td></tr>\n"));
    }
    if let Some(ref created) = profile.metadata.created {
        html.push_str(&format!("<tr><th>Created</th><td>{created}</td></tr>\n"));
    }
    if let Some(rev) = profile.metadata.revision_count {
        html.push_str(&format!("<tr><th>Revisions</th><td>{rev}</td></tr>\n"));
    }
    if let Some(time) = profile.metadata.total_editing_time_minutes {
        html.push_str(&format!("<tr><th>Editing Time</th><td>{time} minutes</td></tr>\n"));
    }
    if let Some(words) = profile.metadata.reported_word_count {
        html.push_str(&format!("<tr><th>Word Count</th><td>{words}</td></tr>\n"));
    }
    html.push_str("</table>\n");

    // RSID Analysis
    html.push_str("<h2>RSID Analysis</h2>\n<table>\n");
    html.push_str(&format!(
        "<tr><th>Unique Sessions</th><td>{}</td></tr>\n",
        profile.rsid_analysis.unique_rsid_count
    ));
    html.push_str(&format!(
        "<tr><th>RSID Diversity</th><td>{:.3}</td></tr>\n",
        profile.rsid_analysis.rsid_diversity
    ));
    html.push_str(&format!(
        "<tr><th>Largest Block</th><td>{} paragraphs</td></tr>\n",
        profile.rsid_analysis.largest_block_size
    ));
    html.push_str(&format!(
        "<tr><th>Revision Scatter</th><td>{:.1}%</td></tr>\n",
        profile.rsid_analysis.revision_scatter * 100.0
    ));
    html.push_str("</table>\n");

    // Construction Pattern
    html.push_str("<h2>Construction Assessment</h2>\n");
    html.push_str(&format!(
        "<p>Pattern: <strong>{:?}</strong></p>\n",
        profile.construction_pattern
    ));
    html.push_str(&format!(
        "<p class='score'>Process Integrity Score: {:.0}/100</p>\n",
        profile.process_integrity_score * 100.0
    ));
    html.push_str(&format!(
        "<div class='assessment'>{}</div>\n",
        profile.assessment_text
    ));

    // Anomalies
    if !profile.anomalies.is_empty() {
        html.push_str("<h2>Anomalies</h2>\n");
        for anomaly in &profile.anomalies {
            let class = match anomaly.severity {
                crate::forensics::docx::rsid::AnomalySeverity::High => "anomaly high",
                _ => "anomaly",
            };
            let loc = anomaly.location.as_deref().unwrap_or("Document-wide");
            html.push_str(&format!(
                "<div class='{class}'><strong>[{:?}]</strong> ({loc}) {}</div>\n",
                anomaly.severity, anomaly.description
            ));
        }
    }

    html.push_str("</body></html>\n");
    html
}
