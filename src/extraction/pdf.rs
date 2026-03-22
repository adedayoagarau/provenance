use crate::utils::errors::{ProvenanceError, Result};
use lopdf::Document;
use std::path::Path;

/// Extract plain text from a PDF file.
pub fn extract(path: &Path) -> Result<String> {
    let doc = Document::load(path).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to load PDF '{}': {e}", path.display()),
    })?;

    // Check for encryption
    if doc.is_encrypted() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("PDF '{}' is encrypted/password-protected", path.display()),
        });
    }

    let mut all_text = String::new();
    let pages = doc.get_pages();

    for (page_num, _) in pages.iter() {
        match doc.extract_text(&[*page_num]) {
            Ok(text) => {
                if !all_text.is_empty() && !text.is_empty() {
                    all_text.push('\n');
                }
                all_text.push_str(&text);
            }
            Err(e) => {
                tracing::warn!(page = page_num, error = %e, "Failed to extract text from PDF page");
            }
        }
    }

    if all_text.trim().is_empty() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!(
                "No extractable text in PDF '{}' (may be image-only)",
                path.display()
            ),
        });
    }

    Ok(all_text)
}
