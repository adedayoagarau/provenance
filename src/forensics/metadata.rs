use crate::utils::errors::Result;
use std::path::Path;
use std::time::SystemTime;

/// Metadata extracted from a file.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_name: String,
    pub file_size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub is_readonly: bool,
}

/// Extract metadata from a file.
pub fn extract(path: &Path) -> Result<FileMetadata> {
    let meta = std::fs::metadata(path)?;

    Ok(FileMetadata {
        file_name: path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        file_size: meta.len(),
        created: meta.created().ok(),
        modified: meta.modified().ok(),
        accessed: meta.accessed().ok(),
        is_readonly: meta.permissions().readonly(),
    })
}
