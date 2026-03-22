//! Provenance — Forensic authorship verification platform.
//!
//! Determines whether a piece of writing was authored by a specific individual
//! by analyzing deeply embedded stylistic, structural, and behavioral patterns.

pub mod forensics;
pub mod extraction;
pub mod analysis;
pub mod identity;
pub mod scoring;
pub mod crypto;
pub mod utils;
pub mod ml;
pub mod data;

use scoring::engine::OutputFormat;
use utils::errors::{ProvenanceError, Result};

/// Analyze a document, optionally comparing against an author profile.
pub fn analyze(file_path: &str, profile_path: Option<&str>) -> Result<String> {
    analyze_with_format(file_path, profile_path, OutputFormat::Text)
}

/// Analyze a document with a specific output format.
pub fn analyze_with_format(
    file_path: &str,
    profile_path: Option<&str>,
    format: OutputFormat,
) -> Result<String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    // Layer 1: File forensics
    let forensic_report = forensics::examine(file_path)?;

    // Layer 2: Text extraction (format-aware) + analysis
    let text = extraction::extract_text(file_path)?;
    let analysis_result = analysis::analyze_text(&text)?;

    // Layer 3: Identity comparison (if profile provided)
    let identity_result = if let Some(profile) = profile_path {
        let author_profile = identity::profile::load(profile)?;
        Some(identity::comparison::compare(&analysis_result, &author_profile))
    } else {
        None
    };

    // Layer 4: Register classification + baselines
    let register = analysis::register::classify(&analysis_result);
    let baseline_report = analysis::baselines::compare(&analysis_result, &register.primary);

    // Layer 5: DOCX deep forensics (if applicable)
    let docx_profile = if file_path.to_lowercase().ends_with(".docx") {
        build_docx_profile(path).ok()
    } else {
        None
    };

    // Layer 6: Scoring model
    let acs = scoring::acs::compute(
        docx_profile.as_ref(),
        identity_result.as_ref(),
        Some(&baseline_report),
    );
    let pii_score = scoring::pii::compute(
        forensic_report.metadata.document_metadata.as_ref().map(|_| &forensic_report.metadata),
        docx_profile.as_ref(),
    );
    let anomaly_report = scoring::anomalies::collect_anomalies(
        docx_profile.as_ref(),
        Some(&baseline_report),
    );

    // Generate full unified report
    let report = scoring::engine::score_full(
        &forensic_report,
        &analysis_result,
        identity_result.as_ref(),
        register,
        baseline_report,
        acs,
        pii_score,
        anomaly_report,
    );
    scoring::report::render_format(&report, format)
}

/// Build a DOCX construction profile for deep forensic analysis.
fn build_docx_profile(
    path: &std::path::Path,
) -> Result<forensics::docx::profile::DocumentConstructionProfile> {
    let archive = forensics::docx::DocxArchive::open(path)?;

    let rsid_analysis = forensics::docx::rsid::analyze(
        &archive.document_xml,
        archive.settings_xml.as_deref(),
    );
    let formatting_analysis = forensics::docx::formatting::analyze(
        &archive.document_xml,
        archive.styles_xml.as_deref(),
    );
    let structural_forensics = forensics::docx::structure::analyze(&archive.document_xml);

    let file_metadata = forensics::metadata::extract(path)?;
    let doc_metadata = file_metadata.document_metadata.as_ref();

    Ok(forensics::docx::profile::build_profile(
        doc_metadata,
        Some(&rsid_analysis),
        Some(&formatting_analysis),
        Some(&structural_forensics),
    ))
}

/// Build an author profile from verified writing samples.
pub fn build_profile(samples_dir: &str, name: &str) -> Result<String> {
    use rayon::prelude::*;

    let samples = extraction::collect_samples(samples_dir)?;
    if samples.is_empty() {
        return Err(ProvenanceError::ProfileError {
            reason: format!("No sample files found in '{samples_dir}'"),
        });
    }

    let analyses: Vec<_> = samples
        .par_iter()
        .map(|sample| {
            let text = extraction::extract_text(sample)?;
            analysis::analyze_text(&text)
        })
        .collect::<Result<Vec<_>>>()?;

    let profile = identity::profile::build(name, &analyses)?;
    let path = identity::profile::save(&profile)?;
    Ok(path)
}

/// Run file forensics only (no text analysis).
pub fn run_forensics(file_path: &str) -> Result<String> {
    run_forensics_with_format(file_path, OutputFormat::Text)
}

/// Run deep DOCX forensic analysis (RSID, formatting, structure, construction profile).
pub fn run_docx_forensics(file_path: &str, format: OutputFormat) -> Result<String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    let profile = build_docx_profile(path)?;

    match format {
        OutputFormat::Json => {
            Ok(serde_json::to_string_pretty(&profile)?)
        }
        _ => Ok(render_docx_profile_text(&profile)),
    }
}

fn render_docx_profile_text(
    profile: &forensics::docx::profile::DocumentConstructionProfile,
) -> String {
    let mut out = String::new();

    out.push_str("═══════════════════════════════════════════════════════\n");
    out.push_str("  DOCX FORENSIC ANALYSIS — Document Construction Profile\n");
    out.push_str("═══════════════════════════════════════════════════════\n\n");

    // Construction pattern
    let pattern_label = match profile.construction_pattern {
        forensics::docx::profile::ConstructionPattern::Organic => "ORGANIC",
        forensics::docx::profile::ConstructionPattern::BulkInsertion => "BULK INSERTION",
        forensics::docx::profile::ConstructionPattern::Hybrid => "HYBRID",
        forensics::docx::profile::ConstructionPattern::Insufficient => "INSUFFICIENT DATA",
    };
    out.push_str(&format!("Construction Pattern: {pattern_label}\n"));
    out.push_str(&format!(
        "Process Integrity Score: {:.0}/100\n\n",
        profile.process_integrity_score
    ));

    // Assessment
    out.push_str("ASSESSMENT\n");
    out.push_str("───────────────────────────────────────────────────────\n");
    out.push_str(&profile.assessment_text);
    out.push_str("\n\n");

    // Derived metrics
    out.push_str("DERIVED METRICS\n");
    out.push_str("───────────────────────────────────────────────────────\n");
    if let Some(vel) = profile.editing_velocity {
        out.push_str(&format!("  Editing velocity:    {vel:.1} words/minute\n"));
    }
    if let Some(sph) = profile.saves_per_hour {
        out.push_str(&format!("  Saves per hour:      {sph:.1}\n"));
    }
    if let Some(wps) = profile.words_per_save {
        out.push_str(&format!("  Words per save:      {wps:.0}\n"));
    }
    if let Some(span) = profile.creation_to_modification_hours {
        out.push_str(&format!("  Development span:    {span:.1} hours\n"));
    }
    out.push('\n');

    // RSID summary
    if let Some(ref rsid) = profile.rsid_analysis {
        out.push_str("RSID ANALYSIS\n");
        out.push_str("───────────────────────────────────────────────────────\n");
        out.push_str(&format!("  Unique editing sessions:  {}\n", rsid.unique_rsid_count));
        out.push_str(&format!("  Total paragraphs:         {}\n", rsid.total_paragraphs));
        out.push_str(&format!("  RSID diversity:           {:.3}\n", rsid.rsid_diversity));
        out.push_str(&format!("  Largest single block:     {} paragraphs\n", rsid.largest_block_size));
        out.push_str(&format!("  Revision scatter:         {:.1}%\n", rsid.revision_scatter * 100.0));
        if let Some(corr) = rsid.progression_correlation {
            out.push_str(&format!("  Progression correlation:  {corr:.3}\n"));
        }
        if !rsid.paste_events.is_empty() {
            out.push_str(&format!("  Paste events detected:    {}\n", rsid.paste_events.len()));
        }
        out.push('\n');
    }

    // Formatting summary
    if let Some(ref fmt) = profile.formatting_analysis {
        out.push_str("FORMATTING CONSISTENCY\n");
        out.push_str("───────────────────────────────────────────────────────\n");
        out.push_str(&format!(
            "  Consistency score:  {:.1}%\n",
            fmt.formatting_consistency_score * 100.0
        ));
        if let Some(ref font) = fmt.baseline.default_font {
            out.push_str(&format!("  Baseline font:      {font}\n"));
        }
        if let Some(size) = fmt.baseline.default_font_size {
            out.push_str(&format!("  Baseline size:      {} half-points\n", size));
        }
        if !fmt.anomalies.is_empty() {
            out.push_str(&format!("  Anomaly clusters:   {}\n", fmt.anomalies.len()));
        }
        out.push('\n');
    }

    // Structural summary
    if let Some(ref struc) = profile.structural_forensics {
        out.push_str("STRUCTURAL ANALYSIS\n");
        out.push_str("───────────────────────────────────────────────────────\n");
        out.push_str(&format!(
            "  Content paragraphs:     {}\n",
            struc.content_paragraphs
        ));
        out.push_str(&format!(
            "  Mean paragraph length:  {:.1} words\n",
            struc.mean_paragraph_length
        ));
        out.push_str(&format!(
            "  Length CV:              {:.3} (higher = more varied)\n",
            struc.paragraph_length_cv
        ));
        if !struc.section_word_counts.is_empty() {
            out.push_str(&format!(
                "  Sections:               {}\n",
                struc.section_word_counts.len()
            ));
            out.push_str(&format!(
                "  Section length CV:      {:.3}\n",
                struc.section_length_cv
            ));
        }
        out.push('\n');
    }

    // Anomalies
    if !profile.anomalies.is_empty() {
        out.push_str("ANOMALIES\n");
        out.push_str("───────────────────────────────────────────────────────\n");
        for anomaly in &profile.anomalies {
            let severity = match anomaly.severity {
                forensics::docx::profile::ForensicSeverity::Low => "LOW",
                forensics::docx::profile::ForensicSeverity::Medium => "MEDIUM",
                forensics::docx::profile::ForensicSeverity::High => "HIGH",
            };
            let location = anomaly
                .location
                .map(|(s, e)| format!(" [paragraphs {s}-{e}]"))
                .unwrap_or_default();
            out.push_str(&format!("  [{severity}]{location} {}\n", anomaly.description));
        }
    }

    out
}

/// Run file forensics with a specific output format.
pub fn run_forensics_with_format(file_path: &str, format: OutputFormat) -> Result<String> {
    // Delegate to full pipeline (forensics-only is now just analyze without profile)
    analyze_with_format(file_path, None, format)
}
