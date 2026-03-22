pub mod plaintext;
pub mod markdown;
pub mod html;
pub mod pdf;
pub mod docx;

use crate::forensics::format::FileType;
use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Extract plain text from a file, auto-detecting format.
pub fn extract_text(file_path: &str) -> Result<String> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    let file_type = detect_type(path);

    let raw_text = match file_type {
        FileType::PlainText => plaintext::extract(path)?,
        FileType::Markdown => markdown::extract(path)?,
        FileType::Html => html::extract(path)?,
        FileType::Pdf => pdf::extract(path)?,
        FileType::Docx => docx::extract(path)?,
        FileType::Unknown(ref ext) => {
            // Try as plain text for unknown extensions
            plaintext::extract(path).map_err(|_| ProvenanceError::UnsupportedFormat {
                format: ext.clone(),
            })?
        }
    };

    Ok(normalize_text(&raw_text))
}

/// Collect file paths from a samples directory.
pub fn collect_samples(dir: &str) -> Result<Vec<String>> {
    let path = Path::new(dir);
    let mut samples = Vec::new();

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

/// Detect file type using extension and magic bytes.
fn detect_type(path: &Path) -> FileType {
    // Try magic bytes first
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.len() >= 4 {
            // PDF: starts with %PDF
            if bytes.starts_with(b"%PDF") {
                return FileType::Pdf;
            }
            // ZIP-based (DOCX): starts with PK\x03\x04
            if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                // Check if it's a DOCX by looking for [Content_Types].xml
                if let Ok(content) = std::str::from_utf8(&bytes) {
                    if content.contains("[Content_Types].xml") || content.contains("word/") {
                        return FileType::Docx;
                    }
                }
                // Also check by extension for ZIP-based formats
                if let Some(ext) = path.extension() {
                    if ext == "docx" {
                        return FileType::Docx;
                    }
                }
            }
        }
    }

    // Fall back to extension
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "txt" | "text" => FileType::PlainText,
        "md" | "markdown" => FileType::Markdown,
        "pdf" => FileType::Pdf,
        "docx" => FileType::Docx,
        "html" | "htm" => FileType::Html,
        other => FileType::Unknown(other.to_string()),
    }
}

/// Normalize extracted text: NFC unicode normalization, whitespace cleanup.
fn normalize_text(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    let normalized: String = text.nfc().collect();

    // Normalize line endings to \n
    let normalized = normalized.replace("\r\n", "\n").replace('\r', "\n");

    // Collapse runs of 3+ newlines to 2 (paragraph boundary)
    let mut result = String::with_capacity(normalized.len());
    let mut newline_count = 0;

    for ch in normalized.chars() {
        if ch == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(ch);
            }
        } else {
            newline_count = 0;
            result.push(ch);
        }
    }

    result.trim().to_string()
}
