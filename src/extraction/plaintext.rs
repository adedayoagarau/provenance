use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Size threshold above which we use memory-mapped I/O (1 MB).
const MMAP_THRESHOLD: u64 = 1024 * 1024;

/// Extract text from a plain text file, handling encoding detection.
///
/// Uses memory-mapped I/O for files larger than 1 MB to avoid copying
/// file contents into a heap-allocated buffer.
pub fn extract(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let bytes: Box<dyn AsRef<[u8]>> = if metadata.len() >= MMAP_THRESHOLD {
        // Use memory-mapped I/O for large files
        let file = std::fs::File::open(path).map_err(|e| ProvenanceError::IoWithPath {
            path: path.display().to_string(),
            source: e,
        })?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file) }
            .map_err(|e| ProvenanceError::IoWithPath {
                path: path.display().to_string(),
                source: e,
            })?;
        Box::new(mmap)
    } else {
        let data = std::fs::read(path).map_err(|e| ProvenanceError::IoWithPath {
            path: path.display().to_string(),
            source: e,
        })?;
        Box::new(data)
    };

    decode_bytes((*bytes).as_ref(), path)
}

/// Decode bytes to string, detecting encoding.
fn decode_bytes(bytes: &[u8], path: &Path) -> Result<String> {
    // Check for BOM (Byte Order Mark)
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        // UTF-8 BOM — skip it
        return String::from_utf8(bytes[3..].to_vec()).map_err(|_| {
            ProvenanceError::ExtractionError {
                reason: format!("Invalid UTF-8 in '{}'", path.display()),
            }
        });
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        // UTF-16 LE BOM
        let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(bytes);
        if had_errors {
            return Err(ProvenanceError::ExtractionError {
                reason: format!("Encoding errors in UTF-16LE file '{}'", path.display()),
            });
        }
        return Ok(decoded.into_owned());
    }

    if bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 BE BOM
        let (decoded, _, had_errors) = encoding_rs::UTF_16BE.decode(bytes);
        if had_errors {
            return Err(ProvenanceError::ExtractionError {
                reason: format!("Encoding errors in UTF-16BE file '{}'", path.display()),
            });
        }
        return Ok(decoded.into_owned());
    }

    // Try UTF-8 first
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return Ok(text);
    }

    // Fall back to Windows-1252 (common for legacy text files)
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    Ok(decoded.into_owned())
}
