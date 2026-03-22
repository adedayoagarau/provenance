use crate::utils::errors::{ProvenanceError, Result};
use std::io::Read;
use std::path::Path;

/// Extract plain text from a DOCX file.
///
/// DOCX files are ZIP archives containing XML. The main content is in
/// `word/document.xml`. We parse the XML to extract text from `<w:t>` elements.
pub fn extract(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to open DOCX '{}' as ZIP: {e}", path.display()),
    })?;

    // Validate that this is actually a DOCX (must contain [Content_Types].xml)
    if archive.by_name("[Content_Types].xml").is_err() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("'{}' is not a valid DOCX file (missing [Content_Types].xml)", path.display()),
        });
    }

    // Extract text from word/document.xml
    let mut document_xml = String::new();
    {
        let mut doc_file = archive.by_name("word/document.xml").map_err(|e| {
            ProvenanceError::ExtractionError {
                reason: format!("Failed to read word/document.xml from '{}': {e}", path.display()),
            }
        })?;
        doc_file.read_to_string(&mut document_xml).map_err(|e| {
            ProvenanceError::IoWithPath {
                path: path.display().to_string(),
                source: e,
            }
        })?;
    }

    Ok(extract_text_from_docx_xml(&document_xml))
}

/// Parse DOCX XML and extract text content.
///
/// DOCX text lives in `<w:t>` elements within `<w:p>` (paragraph) elements.
/// We extract all text and insert paragraph breaks between `<w:p>` blocks.
fn extract_text_from_docx_xml(xml: &str) -> String {
    let mut text = String::new();
    let mut in_paragraph = false;
    let mut paragraph_has_text = false;

    // Simple state-machine XML parser for w:t elements
    let mut chars = xml.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Read tag name
            let mut tag = String::new();
            while let Some(&c) = chars.peek() {
                if c == '>' || c == ' ' || c == '/' {
                    break;
                }
                tag.push(c);
                chars.next();
            }

            // Consume rest of tag
            let mut full_tag = tag.clone();
            while let Some(&c) = chars.peek() {
                full_tag.push(c);
                chars.next();
                if c == '>' {
                    break;
                }
            }

            match tag.as_str() {
                "w:p" => {
                    if in_paragraph && paragraph_has_text {
                        text.push_str("\n\n");
                    }
                    in_paragraph = true;
                    paragraph_has_text = false;
                }
                "/w:p" => {
                    in_paragraph = false;
                }
                "w:t" => {
                    // Extract text content until </w:t>
                    let mut content = String::new();
                    while let Some(c) = chars.next() {
                        if c == '<' {
                            // Should be </w:t>
                            while let Some(&c2) = chars.peek() {
                                chars.next();
                                if c2 == '>' {
                                    break;
                                }
                            }
                            break;
                        }
                        content.push(c);
                    }
                    if !content.is_empty() {
                        text.push_str(&content);
                        paragraph_has_text = true;
                    }
                }
                "w:tab" => {
                    text.push(' ');
                }
                "w:br" => {
                    text.push('\n');
                }
                _ => {}
            }
        }
    }

    text
}
