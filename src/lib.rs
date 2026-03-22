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

/// Batch-analyze a directory of documents.
pub fn batch_analyze(
    dir_path: &str,
    profile_path: Option<&str>,
    format: OutputFormat,
) -> Result<String> {
    use rayon::prelude::*;

    let files = extraction::collect_samples(dir_path)?;
    if files.is_empty() {
        return Err(ProvenanceError::ProfileError {
            reason: format!("No documents found in '{dir_path}'"),
        });
    }

    let profile = profile_path
        .map(identity::profile::load)
        .transpose()?;

    let results: Vec<BatchEntry> = files
        .par_iter()
        .map(|file| {
            let file_name = file.to_string();
            match run_batch_entry(&file_name, profile.as_ref()) {
                Ok(entry) => entry,
                Err(e) => BatchEntry {
                    file: file_name,
                    status: "error".into(),
                    error: Some(e.to_string()),
                    confidence: None,
                    register: None,
                    acs_score: None,
                    word_count: None,
                },
            }
        })
        .collect();

    let summary = BatchSummary {
        total_files: results.len(),
        successful: results.iter().filter(|r| r.status == "ok").count(),
        failed: results.iter().filter(|r| r.status == "error").count(),
        avg_confidence: {
            let confs: Vec<f64> = results.iter().filter_map(|r| r.confidence).collect();
            if confs.is_empty() {
                None
            } else {
                Some(confs.iter().sum::<f64>() / confs.len() as f64)
            }
        },
        entries: results,
    };

    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(&summary)?),
        _ => Ok(render_batch_text(&summary)),
    }
}

fn run_batch_entry(
    file_path: &str,
    profile: Option<&identity::profile::AuthorProfile>,
) -> Result<BatchEntry> {
    let _forensic_report = forensics::examine(file_path)?;
    let text = extraction::extract_text(file_path)?;
    let analysis_result = analysis::analyze_text(&text)?;

    let comparison = profile.map(|p| identity::comparison::compare(&analysis_result, p));
    let register = analysis::register::classify(&analysis_result);
    let baseline = analysis::baselines::compare(&analysis_result, &register.primary);
    let acs = scoring::acs::compute(None, comparison.as_ref(), Some(&baseline));

    Ok(BatchEntry {
        file: file_path.to_string(),
        status: "ok".into(),
        error: None,
        confidence: comparison.as_ref().map(|c| c.confidence.value),
        register: Some(register.label.clone()),
        acs_score: Some(acs.score),
        word_count: Some(analysis_result.lexical.total_words),
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BatchEntry {
    file: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    register: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acs_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    word_count: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BatchSummary {
    total_files: usize,
    successful: usize,
    failed: usize,
    avg_confidence: Option<f64>,
    entries: Vec<BatchEntry>,
}

fn render_batch_text(summary: &BatchSummary) -> String {
    let mut out = String::new();
    out.push_str("═══════════════════════════════════════════════════════\n");
    out.push_str("  BATCH ANALYSIS SUMMARY\n");
    out.push_str("═══════════════════════════════════════════════════════\n\n");
    out.push_str(&format!("  Total files: {}\n", summary.total_files));
    out.push_str(&format!("  Successful:  {}\n", summary.successful));
    out.push_str(&format!("  Failed:      {}\n", summary.failed));
    if let Some(avg) = summary.avg_confidence {
        out.push_str(&format!("  Avg confidence: {:.1}%\n", avg * 100.0));
    }
    out.push_str("\n── Results ──\n\n");

    for entry in &summary.entries {
        if entry.status == "ok" {
            let reg = entry.register.as_deref().unwrap_or("?");
            let acs = entry.acs_score.map(|s| format!("{:.0}", s)).unwrap_or_else(|| "?".into());
            let words = entry.word_count.map(|w| format!("{w}")).unwrap_or_else(|| "?".into());
            let conf = entry.confidence.map(|c| format!("{:.1}%", c * 100.0)).unwrap_or_else(|| "N/A".into());
            out.push_str(&format!(
                "  {} — {words} words, {reg}, ACS: {acs}, confidence: {conf}\n",
                entry.file
            ));
        } else {
            out.push_str(&format!(
                "  {} — ERROR: {}\n",
                entry.file,
                entry.error.as_deref().unwrap_or("unknown")
            ));
        }
    }

    out
}

/// Rank multiple author candidates against a document.
pub fn rank_candidates_with_format(
    file_path: &str,
    profile_paths: &[String],
    format: OutputFormat,
) -> Result<String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    if profile_paths.is_empty() {
        return Err(ProvenanceError::ProfileError {
            reason: "At least one profile is required for ranking".to_string(),
        });
    }

    // Extract text and analyze
    let text = extraction::extract_text(file_path)?;
    let analysis_result = analysis::analyze_text(&text)?;

    // Load all candidate profiles
    let candidates: Vec<identity::profile::AuthorProfile> = profile_paths
        .iter()
        .map(|p| identity::profile::load(p))
        .collect::<Result<Vec<_>>>()?;

    // Rank candidates
    let ranking = identity::ranking::rank_candidates(&analysis_result, &candidates);

    // Render output
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(&ranking)?),
        OutputFormat::Html => Ok(render_ranking_html(&ranking)),
        OutputFormat::Text => Ok(render_ranking_text(&ranking)),
    }
}

fn render_ranking_text(ranking: &identity::ranking::RankingResult) -> String {
    let mut out = String::new();

    out.push_str("═══════════════════════════════════════════════════════\n");
    out.push_str("  AUTHORSHIP RANKING — Candidate Comparison\n");
    out.push_str("═══════════════════════════════════════════════════════\n\n");

    out.push_str(&format!("  {}\n\n", ranking.summary));

    out.push_str(&format!(
        "  Top margin:    {:.1} pp\n",
        ranking.top_margin * 100.0
    ));
    out.push_str(&format!(
        "  Definitive:    {}\n\n",
        if ranking.is_definitive { "Yes" } else { "No — additional evidence recommended" }
    ));

    out.push_str("── Rankings ──\n\n");

    for candidate in &ranking.rankings {
        out.push_str(&format!(
            "  #{} {} — {:.1}% confidence (relative: {:.2})\n",
            candidate.rank,
            candidate.author_name,
            candidate.comparison.confidence.value * 100.0,
            candidate.relative_score,
        ));
        out.push_str(&format!("     {}\n\n", candidate.explanation.narrative));
    }

    out
}

fn render_ranking_html(ranking: &identity::ranking::RankingResult) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><meta charset='utf-8'>\n");
    html.push_str("<title>Provenance — Authorship Ranking</title>\n");
    html.push_str("<style>\n");
    html.push_str("body{font-family:system-ui;max-width:900px;margin:2rem auto;padding:0 1rem;}\n");
    html.push_str("h1{color:#1a1a2e;} h2{color:#16213e;margin-top:1.5rem;}\n");
    html.push_str("table{border-collapse:collapse;width:100%;margin:1rem 0;}\n");
    html.push_str("th,td{border:1px solid #ddd;padding:8px;text-align:left;}\n");
    html.push_str("th{background:#f5f5f5;}\n");
    html.push_str(".rank-1{background:#e8f5e9;} .rank-2{background:#fff3e0;} .rank-3{background:#fce4ec;}\n");
    html.push_str(".summary{background:#f8f9fa;border-left:4px solid #1a1a2e;padding:1rem;margin:1rem 0;}\n");
    html.push_str("</style></head><body>\n");
    html.push_str("<h1>Authorship Ranking</h1>\n");
    html.push_str(&format!("<div class='summary'><p>{}</p>\n", ranking.summary));
    html.push_str(&format!(
        "<p>Top margin: <strong>{:.1} pp</strong> | Definitive: <strong>{}</strong></p></div>\n",
        ranking.top_margin * 100.0,
        if ranking.is_definitive { "Yes" } else { "No" }
    ));

    html.push_str("<h2>Candidate Rankings</h2>\n<table>\n");
    html.push_str("<tr><th>Rank</th><th>Author</th><th>Confidence</th><th>Relative Score</th><th>Assessment</th></tr>\n");
    for c in &ranking.rankings {
        let class = match c.rank {
            1 => "rank-1",
            2 => "rank-2",
            3 => "rank-3",
            _ => "",
        };
        html.push_str(&format!(
            "<tr class='{class}'><td>#{}</td><td>{}</td><td>{:.1}%</td><td>{:.2}</td><td>{}</td></tr>\n",
            c.rank, c.author_name,
            c.comparison.confidence.value * 100.0,
            c.relative_score,
            c.explanation.narrative,
        ));
    }
    html.push_str("</table>\n</body></html>");
    html
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

/// Generate an Ed25519 signing keypair and save to files.
pub fn generate_signing_key(output_dir: &str) -> Result<(String, String)> {
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| utils::errors::ProvenanceError::IoWithPath {
        path: output_dir.to_string(),
        source: e,
    })?;

    let key = crypto::keys::generate_keypair();
    let pub_info = crypto::keys::PublicKeyInfo::from_verifying_key(&key.verifying_key());

    let private_path = dir.join("provenance_signing.key");
    let public_path = dir.join("provenance_public.json");

    crypto::keys::save_signing_key(&key, &private_path)?;
    crypto::keys::save_public_key(&pub_info, &public_path)?;

    Ok((
        private_path.display().to_string(),
        public_path.display().to_string(),
    ))
}

/// Generate a Provenance certificate for an analyzed document.
pub fn generate_certificate(
    file_path: &str,
    signing_key_path: &str,
    profile_path: Option<&str>,
    output_path: Option<&str>,
) -> Result<String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(utils::errors::ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    // Load signing key
    let signing_key =
        crypto::keys::load_signing_key(std::path::Path::new(signing_key_path))?;

    // Run full analysis
    let forensic_report = forensics::examine(file_path)?;
    let text = extraction::extract_text(file_path)?;
    let analysis_result = analysis::analyze_text(&text)?;

    let comparison = if let Some(profile) = profile_path {
        let author_profile = identity::profile::load(profile)?;
        Some(identity::comparison::compare(&analysis_result, &author_profile))
    } else {
        None
    };

    let register = analysis::register::classify(&analysis_result);
    let baseline_report = analysis::baselines::compare(&analysis_result, &register.primary);

    let docx_profile = if file_path.to_lowercase().ends_with(".docx") {
        build_docx_profile(path).ok()
    } else {
        None
    };

    let acs = scoring::acs::compute(
        docx_profile.as_ref(),
        comparison.as_ref(),
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

    // Build certificate input
    let construction_pattern_str = docx_profile.as_ref().map(|p| {
        match p.construction_pattern {
            forensics::docx::profile::ConstructionPattern::Organic => "Organic",
            forensics::docx::profile::ConstructionPattern::BulkInsertion => "BulkInsertion",
            forensics::docx::profile::ConstructionPattern::Hybrid => "Hybrid",
            forensics::docx::profile::ConstructionPattern::Insufficient => "Insufficient",
        }
    });

    let input = crypto::certificate::CertificateInput {
        file_name: &forensic_report.metadata.file_name,
        file_hash: &forensic_report.integrity.sha256,
        file_size: forensic_report.integrity.file_size,
        format: &format!("{:?}", forensic_report.format.detected_type),
        word_count: analysis_result.lexical.total_words,
        acs_score: acs.score,
        acs_margin: acs.margin,
        pii_score: pii_score.score,
        pii_level: pii_score.level.label(),
        register: &register.label,
        anomaly_count: anomaly_report.flags.len(),
        construction_pattern: construction_pattern_str,
        has_docx_forensics: docx_profile.is_some(),
        has_author_profile: comparison.is_some(),
        rsid_session_count: docx_profile.as_ref().and_then(|p| {
            p.rsid_analysis.as_ref().map(|r| r.unique_rsid_count)
        }),
        formatting_consistency: docx_profile.as_ref().and_then(|p| {
            p.formatting_analysis.as_ref().map(|f| f.formatting_consistency_score)
        }),
    };

    let cert = crypto::certificate::generate(&input, &signing_key)?;

    // Determine output path
    let cert_path = if let Some(out) = output_path {
        std::path::PathBuf::from(out)
    } else {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        std::path::PathBuf::from(format!("{stem}.provenance.json"))
    };

    crypto::certificate::save(&cert, &cert_path)?;
    Ok(cert_path.display().to_string())
}

/// Verify a Provenance certificate.
pub fn verify_certificate(
    cert_path: &str,
    document_path: Option<&str>,
    format: OutputFormat,
) -> Result<String> {
    let cert = crypto::certificate::load(std::path::Path::new(cert_path))?;

    let result = if let Some(doc_path) = document_path {
        let doc_hash = crypto::hashing::sha256_file(doc_path)?;
        crypto::verify::verify_against_document(&cert, &doc_hash)?
    } else {
        crypto::verify::verify(&cert)?
    };

    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(&result)?),
        _ => Ok(crypto::verify::render_text(&result)),
    }
}
