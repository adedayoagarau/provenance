//! Provenance — Forensic authorship verification platform.
//!
//! Determines whether a piece of writing was authored by a specific individual
//! by analyzing deeply embedded stylistic, structural, and behavioral patterns.

pub mod forensics;
pub mod analysis;
pub mod identity;
pub mod scoring;
pub mod crypto;
pub mod utils;

use anyhow::Result;

/// Analyze a document, optionally comparing against an author profile.
pub async fn analyze(file_path: &str, profile_path: Option<&str>) -> Result<String> {
    // Layer 1: File forensics
    let forensic_report = forensics::examine(file_path)?;

    // Layer 2: Text analysis
    let text = forensics::extract_text(file_path)?;
    let analysis_result = analysis::analyze_text(&text)?;

    // Layer 3: Identity comparison (if profile provided)
    let identity_result = if let Some(profile) = profile_path {
        let author_profile = identity::profile::load(profile)?;
        Some(identity::comparison::compare(&analysis_result, &author_profile)?)
    } else {
        None
    };

    // Generate unified report
    let report = scoring::engine::score(&forensic_report, &analysis_result, identity_result.as_ref())?;
    scoring::report::render(&report)
}

/// Build an author profile from verified writing samples.
pub async fn build_profile(samples_dir: &str, name: &str) -> Result<String> {
    let samples = forensics::collect_samples(samples_dir)?;
    let mut analyses = Vec::new();

    for sample in &samples {
        let text = forensics::extract_text(sample)?;
        let result = analysis::analyze_text(&text)?;
        analyses.push(result);
    }

    let profile = identity::profile::build(name, &analyses)?;
    let path = identity::profile::save(&profile)?;
    Ok(path)
}

/// Run file forensics only (no text analysis).
pub async fn run_forensics(file_path: &str) -> Result<String> {
    let report = forensics::examine(file_path)?;
    Ok(format!("{report:#?}"))
}
