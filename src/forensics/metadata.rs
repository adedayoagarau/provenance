use crate::utils::errors::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

/// Metadata extracted from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_name: String,
    pub file_size: u64,
    pub created_epoch: Option<u64>,
    pub modified_epoch: Option<u64>,
    pub accessed_epoch: Option<u64>,
    pub is_readonly: bool,
}

fn to_epoch(time: Option<SystemTime>) -> Option<u64> {
    time.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Extract metadata from a file.
pub fn extract(path: &Path) -> Result<FileMetadata> {
    let meta = std::fs::metadata(path)?;

    Ok(FileMetadata {
        file_name: path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        file_size: meta.len(),
        created_epoch: to_epoch(meta.created().ok()),
        modified_epoch: to_epoch(meta.modified().ok()),
        accessed_epoch: to_epoch(meta.accessed().ok()),
        is_readonly: meta.permissions().readonly(),
    })
}
