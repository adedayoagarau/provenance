use anyhow::Result;
use std::path::Path;

/// Detected file format information.
#[derive(Debug, Clone)]
pub struct FormatInfo {
    pub extension: String,
    pub detected_type: FileType,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    PlainText,
    Markdown,
    Pdf,
    Docx,
    Html,
    Unknown(String),
}

/// Analyze the format of a file.
pub fn analyze(path: &Path) -> Result<FormatInfo> {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    let detected_type = match extension.as_str() {
        "txt" => FileType::PlainText,
        "md" | "markdown" => FileType::Markdown,
        "pdf" => FileType::Pdf,
        "docx" => FileType::Docx,
        "html" | "htm" => FileType::Html,
        other => FileType::Unknown(other.to_string()),
    };

    Ok(FormatInfo {
        extension,
        detected_type,
        encoding: "utf-8".to_string(),
    })
}
