pub mod metadata;
pub mod integrity;
pub mod format;
pub mod timeline;

use crate::utils::errors::{self, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Forensic examination report for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicReport {
    pub metadata: metadata::FileMetadata,
    pub integrity: integrity::IntegrityResult,
    pub format: format::FormatInfo,
    pub timeline: timeline::DocumentTimeline,
}

/// Perform a full forensic examination of a file.
pub fn examine(file_path: &str) -> Result<ForensicReport> {
    let path = Path::new(file_path);

    let metadata = metadata::extract(path)?;
    let integrity = integrity::check(path)?;
    let format_info = format::analyze(path);
    let timeline = timeline::construct(path, &metadata);

    Ok(ForensicReport {
        metadata,
        integrity,
        format: format_info,
        timeline,
    })
}

/// Extract plain text from a document file.
pub fn extract_text(file_path: &str) -> Result<String> {
    let path = Path::new(file_path);
    errors::read_file_string(path)
}

/// Collect file paths from a samples directory.
pub fn collect_samples(dir: &str) -> Result<Vec<String>> {
    let mut samples = Vec::new();
    let path = Path::new(dir);

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(path_str) = entry.path().to_str() {
                samples.push(path_str.to_string());
            }
        }
    }

    samples.sort();
    Ok(samples)
}
