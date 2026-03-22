use serde::{Deserialize, Serialize};
use std::path::Path;

/// Detected file format information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub extension: String,
    pub detected_type: FileType,
    pub encoding: String,
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

/// Analyze the format of a file.
pub fn analyze(path: &Path) -> FormatInfo {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let detected_type = match extension.as_str() {
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
        other => FileType::Unknown(other.to_string()),
    };

    FormatInfo {
        extension,
        detected_type,
        encoding: "utf-8".to_string(),
    }
}
