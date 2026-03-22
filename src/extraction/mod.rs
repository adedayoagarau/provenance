pub mod plaintext;
pub mod markdown;
pub mod html;
pub mod pdf;
pub mod docx;
pub mod rtf;
pub mod odt;
pub mod epub;
pub mod email;
pub mod latex;
pub mod chat_export;

use crate::forensics::format::FileType;
use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Extract plain text from a file, auto-detecting format.
///
/// Supports: TXT, Markdown, HTML, PDF, DOCX, RTF, ODT, EPUB, EML, MBOX,
/// LaTeX, and JSON/CSV chat exports.
pub fn extract_text(file_path: &str) -> Result<String> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    // Security: validate file before processing
    let validation_config = crate::utils::validation::ValidationConfig::default();
    crate::utils::validation::validate_file(path, &validation_config)?;

    let file_type = detect_type(path);

    let raw_text = match file_type {
        FileType::PlainText => plaintext::extract(path)?,
        FileType::Markdown => markdown::extract(path)?,
        FileType::Html => html::extract(path)?,
        FileType::Pdf => pdf::extract(path)?,
        FileType::Docx => docx::extract(path)?,
        FileType::Rtf => rtf::extract(path)?,
        FileType::Odt => odt::extract(path)?,
        FileType::Epub => epub::extract(path)?,
        FileType::Eml => email::extract_eml(path)?,
        FileType::Mbox => email::extract_mbox(path)?,
        FileType::Latex => latex::extract(path)?,
        FileType::ChatExport => chat_export::extract(path)?,
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

/// Detect file type using magic bytes and extension.
fn detect_type(path: &Path) -> FileType {
    // Try magic bytes first
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.len() >= 4 {
            // PDF: starts with %PDF
            if bytes.starts_with(b"%PDF") {
                return FileType::Pdf;
            }

            // ZIP-based formats: starts with PK\x03\x04
            if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                return detect_zip_type(path, &bytes);
            }

            // RTF: starts with {\rtf
            if bytes.starts_with(b"{\\rtf") {
                return FileType::Rtf;
            }
        }

        // EML: look for email headers
        if let Ok(text) = std::str::from_utf8(&bytes[..bytes.len().min(2048)]) {
            if (text.starts_with("From:") || text.starts_with("Received:")
                || text.starts_with("Return-Path:") || text.starts_with("MIME-Version:"))
                && text.contains("Subject:")
            {
                return FileType::Eml;
            }

            // MBOX: starts with "From " (space after From)
            if text.starts_with("From ") && text.contains("\nFrom:") {
                return FileType::Mbox;
            }

            // LaTeX: look for \documentclass or \begin{document}
            if text.contains("\\documentclass") || text.contains("\\begin{document}") {
                return FileType::Latex;
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
        "html" | "htm" | "xhtml" => FileType::Html,
        "rtf" => FileType::Rtf,
        "odt" => FileType::Odt,
        "epub" => FileType::Epub,
        "eml" => FileType::Eml,
        "mbox" | "mbx" => FileType::Mbox,
        "tex" | "latex" => FileType::Latex,
        "json" | "csv" => FileType::ChatExport,
        other => FileType::Unknown(other.to_string()),
    }
}

/// Detect specific ZIP-based format (DOCX, ODT, EPUB).
fn detect_zip_type(path: &Path, _bytes: &[u8]) -> FileType {
    // Try to open as ZIP and check contents
    if let Ok(file) = std::fs::File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            // DOCX: contains word/document.xml
            if archive.by_name("word/document.xml").is_ok() {
                return FileType::Docx;
            }
            // ODT: mimetype contains "opendocument.text"
            if let Ok(mut mimetype) = archive.by_name("mimetype") {
                let mut mime = String::new();
                if std::io::Read::read_to_string(&mut mimetype, &mut mime).is_ok() {
                    if mime.contains("opendocument.text") {
                        return FileType::Odt;
                    }
                }
            }
            // EPUB: mimetype contains "epub+zip"
            if let Ok(mut mimetype) = archive.by_name("mimetype") {
                let mut mime = String::new();
                if std::io::Read::read_to_string(&mut mimetype, &mut mime).is_ok() {
                    if mime.contains("epub+zip") {
                        return FileType::Epub;
                    }
                }
            }
            // EPUB fallback: has META-INF/container.xml
            if archive.by_name("META-INF/container.xml").is_ok() {
                return FileType::Epub;
            }
        }
    }

    // Fall back to extension for ZIP formats
    let ext = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "docx" => FileType::Docx,
        "odt" => FileType::Odt,
        "epub" => FileType::Epub,
        _ => FileType::Unknown(ext),
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
