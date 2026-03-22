use crate::utils::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

/// Metadata extracted from a file (filesystem + document-embedded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    // Filesystem metadata
    pub file_name: String,
    pub file_size: u64,
    pub created_epoch: Option<u64>,
    pub modified_epoch: Option<u64>,
    pub accessed_epoch: Option<u64>,
    pub is_readonly: bool,

    // Document-embedded metadata (from DOCX, PDF, etc.)
    pub document_metadata: Option<DocumentMetadata>,
}

/// Metadata embedded within document formats (DOCX, PDF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document subject/description
    pub subject: Option<String>,
    /// Creating application name
    pub creator_tool: Option<String>,
    /// Producer (for PDFs: the PDF library used)
    pub producer: Option<String>,
    /// Creation date (ISO 8601 string if available)
    pub creation_date: Option<String>,
    /// Last modification date
    pub modification_date: Option<String>,
    /// Revision count (DOCX)
    pub revision_count: Option<u32>,
    /// Page/section count
    pub page_count: Option<u32>,
    /// Word count (as reported by the document)
    pub reported_word_count: Option<u32>,
    /// Additional key-value metadata
    pub custom: HashMap<String, String>,
}

fn to_epoch(time: Option<SystemTime>) -> Option<u64> {
    time.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Extract metadata from a file (filesystem + embedded document metadata).
pub fn extract(path: &Path) -> Result<FileMetadata> {
    let meta = std::fs::metadata(path)?;

    let document_metadata = extract_document_metadata(path);

    Ok(FileMetadata {
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        file_size: meta.len(),
        created_epoch: to_epoch(meta.created().ok()),
        modified_epoch: to_epoch(meta.modified().ok()),
        accessed_epoch: to_epoch(meta.accessed().ok()),
        is_readonly: meta.permissions().readonly(),
        document_metadata,
    })
}

/// Extract embedded metadata from document formats.
fn extract_document_metadata(path: &Path) -> Option<DocumentMetadata> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "pdf" => extract_pdf_metadata(path),
        "docx" => extract_docx_metadata(path),
        _ => {
            // Try magic bytes for format detection
            if let Ok(bytes) = std::fs::read(path) {
                if bytes.starts_with(b"%PDF") {
                    return extract_pdf_metadata(path);
                }
                if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                    return extract_docx_metadata(path);
                }
            }
            None
        }
    }
}

/// Extract metadata from PDF files using lopdf.
fn extract_pdf_metadata(path: &Path) -> Option<DocumentMetadata> {
    let doc = lopdf::Document::load(path).ok()?;

    let mut meta = DocumentMetadata {
        title: None,
        author: None,
        subject: None,
        creator_tool: None,
        producer: None,
        creation_date: None,
        modification_date: None,
        revision_count: None,
        page_count: Some(doc.get_pages().len() as u32),
        reported_word_count: None,
        custom: HashMap::new(),
    };

    // Extract from /Info dictionary
    if let Ok(info_id) = doc.trailer.get(b"Info") {
        if let Ok(info_ref) = info_id.as_reference() {
            if let Ok(info) = doc.get_dictionary(info_ref) {
                meta.title = get_pdf_string(info, b"Title");
                meta.author = get_pdf_string(info, b"Author");
                meta.subject = get_pdf_string(info, b"Subject");
                meta.creator_tool = get_pdf_string(info, b"Creator");
                meta.producer = get_pdf_string(info, b"Producer");
                meta.creation_date = get_pdf_string(info, b"CreationDate");
                meta.modification_date = get_pdf_string(info, b"ModDate");

                // Collect any other keys as custom metadata
                for (key, value) in info.iter() {
                    let key_str = String::from_utf8_lossy(key).to_string();
                    if !["Title", "Author", "Subject", "Creator", "Producer", "CreationDate", "ModDate"]
                        .contains(&key_str.as_str())
                    {
                        if let Ok(s) = value.as_string() {
                            meta.custom.insert(key_str, s.into_owned());
                        }
                    }
                }
            }
        }
    }

    Some(meta)
}

/// Helper to extract a string from a PDF dictionary.
fn get_pdf_string(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key)
        .ok()
        .and_then(|v| {
            v.as_string()
                .ok()
                .map(|s| s.to_string())
                .or_else(|| v.as_name_str().ok().map(|s| s.to_string()))
        })
}

/// Extract metadata from DOCX files (ZIP + XML).
fn extract_docx_metadata(path: &Path) -> Option<DocumentMetadata> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    let mut meta = DocumentMetadata {
        title: None,
        author: None,
        subject: None,
        creator_tool: None,
        producer: None,
        creation_date: None,
        modification_date: None,
        revision_count: None,
        page_count: None,
        reported_word_count: None,
        custom: HashMap::new(),
    };

    // Parse docProps/core.xml (Dublin Core metadata)
    if let Ok(mut core_xml) = archive.by_name("docProps/core.xml") {
        let mut content = String::new();
        if std::io::Read::read_to_string(&mut core_xml, &mut content).is_ok() {
            meta.title = extract_xml_value(&content, "dc:title");
            meta.author = extract_xml_value(&content, "dc:creator");
            meta.subject = extract_xml_value(&content, "dc:subject");
            meta.creation_date = extract_xml_value(&content, "dcterms:created");
            meta.modification_date = extract_xml_value(&content, "dcterms:modified");

            if let Some(rev) = extract_xml_value(&content, "cp:revision") {
                meta.revision_count = rev.parse().ok();
            }
        }
    }

    // Parse docProps/app.xml (application metadata)
    if let Ok(mut app_xml) = archive.by_name("docProps/app.xml") {
        let mut content = String::new();
        if std::io::Read::read_to_string(&mut app_xml, &mut content).is_ok() {
            meta.creator_tool = extract_xml_value(&content, "Application");

            if let Some(pages) = extract_xml_value(&content, "Pages") {
                meta.page_count = pages.parse().ok();
            }
            if let Some(words) = extract_xml_value(&content, "Words") {
                meta.reported_word_count = words.parse().ok();
            }
        }
    }

    Some(meta)
}

/// Simple XML value extractor (no full XML parser needed).
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");

    let start_pos = xml.find(&open)?;
    let after_open = &xml[start_pos + open.len()..];

    // Skip attributes and find >
    let gt_pos = after_open.find('>')?;
    let content_start = &after_open[gt_pos + 1..];

    let end_pos = content_start.find(&close)?;
    let value = content_start[..end_pos].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_filesystem_metadata() {
        let dir = std::env::temp_dir().join("provenance_test_meta");
        let _ = fs::create_dir_all(&dir);

        let path = dir.join("test.txt");
        fs::write(&path, "hello world").unwrap();

        let meta = extract(&path).unwrap();
        assert_eq!(meta.file_name, "test.txt");
        assert_eq!(meta.file_size, 11);
        assert!(meta.modified_epoch.is_some());
        assert!(!meta.is_readonly);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_xml_value_extraction() {
        let xml = r#"<?xml version="1.0"?>
            <cp:coreProperties>
                <dc:title>My Document</dc:title>
                <dc:creator>Alice</dc:creator>
                <cp:revision>5</cp:revision>
            </cp:coreProperties>"#;

        assert_eq!(extract_xml_value(xml, "dc:title"), Some("My Document".into()));
        assert_eq!(extract_xml_value(xml, "dc:creator"), Some("Alice".into()));
        assert_eq!(extract_xml_value(xml, "cp:revision"), Some("5".into()));
        assert_eq!(extract_xml_value(xml, "nonexistent"), None);
    }

    #[test]
    fn test_xml_value_with_attributes() {
        let xml = r#"<dcterms:created xsi:type="dcterms:W3CDTF">2024-01-15</dcterms:created>"#;
        assert_eq!(
            extract_xml_value(xml, "dcterms:created"),
            Some("2024-01-15".into())
        );
    }
}
