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

    // Generate unified report
    let report = scoring::engine::score(&forensic_report, &analysis_result, identity_result.as_ref());
    scoring::report::render_format(&report, format)
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

/// Run DOCX-specific forensic analysis.
pub fn run_docx_forensics(file_path: &str, format: OutputFormat) -> Result<String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }
    forensics::docx::analyze_docx_formatted(path, format)
}

/// Run file forensics with a specific output format.
pub fn run_forensics_with_format(file_path: &str, format: OutputFormat) -> Result<String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    let forensic_report = forensics::examine(file_path)?;
    let text = extraction::extract_text(file_path).unwrap_or_default();
    let analysis_result = analysis::analyze_text(&text)?;

    let report = scoring::engine::score(&forensic_report, &analysis_result, None);
    scoring::report::render_format(&report, format)
}
