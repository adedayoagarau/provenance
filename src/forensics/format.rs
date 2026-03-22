use serde::{Deserialize, Serialize};
use std::path::Path;

/// Detected file format information with magic byte analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    /// File extension (lowercase)
    pub extension: String,
    /// Detected file type from magic bytes
    pub detected_type: FileType,
    /// Detected encoding
    pub encoding: String,
    /// Raw magic bytes (first 16 bytes, hex)
    pub magic_bytes: String,
    /// Whether extension matches magic byte detection
    pub extension_matches: bool,
    /// Mismatch warning (if extension doesn't match content)
    pub mismatch_warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    PlainText,
    Markdown,
    Pdf,
    Docx,
    Html,
    Rtf,
    Odt,
    Epub,
    Eml,
    Mbox,
    Latex,
    ChatExport,
    Unknown(String),
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlainText => write!(f, "Plain Text"),
            Self::Markdown => write!(f, "Markdown"),
            Self::Pdf => write!(f, "PDF"),
            Self::Docx => write!(f, "DOCX"),
            Self::Html => write!(f, "HTML"),
            Self::Rtf => write!(f, "RTF"),
            Self::Odt => write!(f, "ODT"),
            Self::Epub => write!(f, "EPUB"),
            Self::Eml => write!(f, "EML"),
            Self::Mbox => write!(f, "MBOX"),
            Self::Latex => write!(f, "LaTeX"),
            Self::ChatExport => write!(f, "Chat Export"),
            Self::Unknown(ext) => write!(f, "Unknown ({ext})"),
        }
    }
}

/// Known magic byte signatures.
struct MagicSignature {
    bytes: &'static [u8],
    offset: usize,
    file_type: FileType,
    #[allow(dead_code)]
    description: &'static str,
}

const SIGNATURES: &[MagicSignature] = &[
    MagicSignature { bytes: b"%PDF", offset: 0, file_type: FileType::Pdf, description: "PDF document" },
    MagicSignature { bytes: &[0x50, 0x4B, 0x03, 0x04], offset: 0, file_type: FileType::Docx, description: "ZIP/DOCX/ODT/EPUB archive" },
    MagicSignature { bytes: b"{\\rtf", offset: 0, file_type: FileType::Rtf, description: "Rich Text Format" },
    MagicSignature { bytes: &[0xEF, 0xBB, 0xBF], offset: 0, file_type: FileType::PlainText, description: "UTF-8 BOM" },
    MagicSignature { bytes: &[0xFF, 0xFE], offset: 0, file_type: FileType::PlainText, description: "UTF-16 LE BOM" },
    MagicSignature { bytes: &[0xFE, 0xFF], offset: 0, file_type: FileType::PlainText, description: "UTF-16 BE BOM" },
];

/// Analyze the format of a file using magic bytes and extension.
pub fn analyze(path: &Path) -> FormatInfo {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let bytes = std::fs::read(path).unwrap_or_default();
    let magic_bytes = format_hex(&bytes[..bytes.len().min(16)]);
    let encoding = detect_encoding(&bytes);

    // Detect type from magic bytes
    let magic_type = detect_from_magic(&bytes, path);

    // Detect type from extension
    let ext_type = detect_from_extension(&extension);

    // Check for mismatch
    let (extension_matches, mismatch_warning) =
        check_mismatch(&magic_type, &ext_type, &extension);

    FormatInfo {
        extension,
        detected_type: magic_type,
        encoding,
        magic_bytes,
        extension_matches,
        mismatch_warning,
    }
}

/// Detect file type from magic bytes.
fn detect_from_magic(bytes: &[u8], path: &Path) -> FileType {
    if bytes.len() < 4 {
        return detect_text_subtype(bytes, path);
    }

    // Check binary signatures
    for sig in SIGNATURES {
        if bytes.len() >= sig.offset + sig.bytes.len()
            && bytes[sig.offset..sig.offset + sig.bytes.len()] == *sig.bytes
        {
            // ZIP-based format: disambiguate
            if sig.bytes == [0x50, 0x4B, 0x03, 0x04] {
                return detect_zip_subtype(path);
            }
            return sig.file_type.clone();
        }
    }

    // Text-based detection
    detect_text_subtype(bytes, path)
}

/// Detect specific ZIP-based format by inspecting archive contents.
fn detect_zip_subtype(path: &Path) -> FileType {
    if let Ok(file) = std::fs::File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            if archive.by_name("word/document.xml").is_ok() {
                return FileType::Docx;
            }
            if let Ok(mut mimetype) = archive.by_name("mimetype") {
                let mut mime = String::new();
                if std::io::Read::read_to_string(&mut mimetype, &mut mime).is_ok() {
                    if mime.contains("opendocument.text") {
                        return FileType::Odt;
                    }
                    if mime.contains("epub+zip") {
                        return FileType::Epub;
                    }
                }
            }
            if archive.by_name("META-INF/container.xml").is_ok() {
                return FileType::Epub;
            }
        }
    }

    // Fall back to extension
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "docx" => FileType::Docx,
        "odt" => FileType::Odt,
        "epub" => FileType::Epub,
        _ => FileType::Unknown(ext),
    }
}

/// Detect text-based format subtypes from content.
fn detect_text_subtype(bytes: &[u8], path: &Path) -> FileType {
    if let Ok(text) = std::str::from_utf8(&bytes[..bytes.len().min(4096)]) {
        // Email headers
        if (text.starts_with("From:")
            || text.starts_with("Received:")
            || text.starts_with("Return-Path:")
            || text.starts_with("MIME-Version:"))
            && text.contains("Subject:")
        {
            return FileType::Eml;
        }
        if text.starts_with("From ") && text.contains("\nFrom:") {
            return FileType::Mbox;
        }
        // LaTeX
        if text.contains("\\documentclass") || text.contains("\\begin{document}") {
            return FileType::Latex;
        }
        // HTML
        if text.trim_start().starts_with("<!DOCTYPE")
            || text.trim_start().starts_with("<html")
            || text.trim_start().starts_with("<HTML")
        {
            return FileType::Html;
        }
        // JSON (chat export)
        if text.trim_start().starts_with('{') || text.trim_start().starts_with('[') {
            return FileType::ChatExport;
        }
    }

    // Fall back to extension
    detect_from_extension(
        &path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
    )
}

/// Detect type from file extension only.
fn detect_from_extension(ext: &str) -> FileType {
    match ext {
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

/// Check for extension vs magic byte mismatch.
fn check_mismatch(
    magic_type: &FileType,
    ext_type: &FileType,
    extension: &str,
) -> (bool, Option<String>) {
    // Skip mismatch check for unknowns and text types (often ambiguous)
    if matches!(magic_type, FileType::Unknown(_))
        || matches!(ext_type, FileType::Unknown(_))
        || extension.is_empty()
    {
        return (true, None);
    }

    // Text subtypes are interchangeable (txt/md/tex all look like text)
    if is_text_type(magic_type) && is_text_type(ext_type) {
        return (true, None);
    }

    if magic_type != ext_type {
        let warning = format!(
            "Extension '.{extension}' suggests {ext_type}, but content detected as {magic_type}. \
             File may have been renamed or tampered with."
        );
        (false, Some(warning))
    } else {
        (true, None)
    }
}

fn is_text_type(ft: &FileType) -> bool {
    matches!(
        ft,
        FileType::PlainText | FileType::Markdown | FileType::Latex | FileType::ChatExport
    )
}

/// Detect text encoding from byte content.
fn detect_encoding(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return "UTF-8 (BOM)".to_string();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return "UTF-16 LE".to_string();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return "UTF-16 BE".to_string();
    }

    // Check if valid UTF-8
    if std::str::from_utf8(bytes).is_ok() {
        return "UTF-8".to_string();
    }

    // Try Windows-1252 / Latin-1 heuristic
    let high_bytes = bytes.iter().filter(|&&b| b >= 0x80).count();
    if high_bytes > 0 {
        "Windows-1252 (likely)".to_string()
    } else {
        "ASCII".to_string()
    }
}

/// Format bytes as hex string.
fn format_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_magic_pdf() {
        let dir = std::env::temp_dir().join("provenance_test_magic");
        let _ = fs::create_dir_all(&dir);

        // Create a fake PDF with correct magic bytes
        let path = dir.join("test.pdf");
        fs::write(&path, b"%PDF-1.4 fake content").unwrap();

        let info = analyze(&path);
        assert_eq!(info.detected_type, FileType::Pdf);
        assert!(info.extension_matches);
        assert!(info.mismatch_warning.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_magic_rtf() {
        let dir = std::env::temp_dir().join("provenance_test_magic_rtf");
        let _ = fs::create_dir_all(&dir);

        let path = dir.join("doc.rtf");
        fs::write(&path, b"{\\rtf1 test content}").unwrap();

        let info = analyze(&path);
        assert_eq!(info.detected_type, FileType::Rtf);
        assert!(info.extension_matches);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extension_mismatch() {
        let dir = std::env::temp_dir().join("provenance_test_mismatch");
        let _ = fs::create_dir_all(&dir);

        // PDF content with .txt extension
        let path = dir.join("fake.txt");
        fs::write(&path, b"%PDF-1.4 this is really a PDF").unwrap();

        let info = analyze(&path);
        assert_eq!(info.detected_type, FileType::Pdf);
        assert!(!info.extension_matches);
        assert!(info.mismatch_warning.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_encoding_detection() {
        assert_eq!(detect_encoding(b"hello world"), "UTF-8");
        assert_eq!(detect_encoding(&[0xEF, 0xBB, 0xBF, b'h']), "UTF-8 (BOM)");
        assert_eq!(detect_encoding(&[0xFF, 0xFE, 0, 0]), "UTF-16 LE");
        assert_eq!(detect_encoding(&[0xFE, 0xFF, 0, 0]), "UTF-16 BE");
    }

    #[test]
    fn test_format_hex() {
        assert_eq!(format_hex(&[0x50, 0x4B, 0x03, 0x04]), "50 4b 03 04");
        assert_eq!(format_hex(&[0xFF, 0xFE]), "ff fe");
    }
}
