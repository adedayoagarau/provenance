use crate::utils::errors::{ProvenanceError, Result};
use std::io::Read;
use std::path::Path;

/// Extract plain text from an ODT (OpenDocument Text) file.
///
/// ODT files are ZIP archives containing XML. The main content is in
/// `content.xml`. Text lives in `<text:p>` elements.
pub fn extract(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to open ODT '{}' as ZIP: {e}", path.display()),
    })?;

    // Validate this is an ODT (should have mimetype file)
    if let Ok(mut mimetype) = archive.by_name("mimetype") {
        let mut mime = String::new();
        let _ = mimetype.read_to_string(&mut mime);
        if !mime.contains("opendocument.text") {
            tracing::warn!("ODT mimetype mismatch: {}", mime.trim());
        }
    }

    // Extract text from content.xml
    let mut content_xml = String::new();
    {
        let mut content_file = archive.by_name("content.xml").map_err(|e| {
            ProvenanceError::ExtractionError {
                reason: format!("Failed to read content.xml from '{}': {e}", path.display()),
            }
        })?;
        content_file.read_to_string(&mut content_xml).map_err(|e| {
            ProvenanceError::IoWithPath {
                path: path.display().to_string(),
                source: e,
            }
        })?;
    }

    Ok(extract_text_from_odt_xml(&content_xml))
}

/// Parse ODT content.xml and extract text.
///
/// Text lives in `<text:p>` and `<text:h>` elements. `<text:span>` contains
/// inline text runs. `<text:s/>` represents spaces, `<text:tab/>` represents tabs.
fn extract_text_from_odt_xml(xml: &str) -> String {
    let mut text = String::new();
    let mut in_text_element = false;
    let mut chars = xml.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            let mut is_closing = false;

            if let Some(&'/') = chars.peek() {
                is_closing = true;
                chars.next();
            }

            // Read tag name
            while let Some(&c) = chars.peek() {
                if c == '>' || c == ' ' || c == '/' {
                    break;
                }
                tag.push(c);
                chars.next();
            }

            // Check for self-closing
            let mut is_self_closing = false;
            while let Some(&c) = chars.peek() {
                if c == '>' {
                    chars.next();
                    break;
                }
                if c == '/' {
                    is_self_closing = true;
                }
                chars.next();
            }

            match tag.as_str() {
                "text:p" | "text:h" if !is_closing => {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push_str("\n\n");
                    }
                    in_text_element = true;
                }
                "text:p" | "text:h" if is_closing => {
                    in_text_element = false;
                }
                "text:s" if is_self_closing || !is_closing => {
                    text.push(' ');
                }
                "text:tab" if is_self_closing || !is_closing => {
                    text.push('\t');
                }
                "text:line-break" if is_self_closing || !is_closing => {
                    text.push('\n');
                }
                _ => {}
            }
        } else if in_text_element {
            text.push(ch);
        }
    }

    text.trim().to_string()
}
