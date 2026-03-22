use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Extract plain text body from an EML email file.
///
/// Handles MIME multipart messages, extracting text/plain parts preferentially,
/// falling back to text/html with HTML stripping.
pub fn extract_eml(path: &Path) -> Result<String> {
    let raw = std::fs::read(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let parsed = mailparse::parse_mail(&raw).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to parse email '{}': {e}", path.display()),
    })?;

    let body = extract_body_from_mail(&parsed)?;

    if body.trim().is_empty() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("No extractable text in email '{}'", path.display()),
        });
    }

    Ok(body)
}

/// Extract text from an MBOX file containing multiple emails.
///
/// Returns concatenated text from all messages, separated by markers.
pub fn extract_mbox(path: &Path) -> Result<String> {
    let content = std::fs::read(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let content_str = String::from_utf8_lossy(&content);
    let mut all_text = String::new();
    let mut current_message = Vec::new();
    let mut message_count = 0;

    for line in content_str.as_bytes().split(|&b| b == b'\n') {
        if line.starts_with(b"From ") && !current_message.is_empty() {
            // Process accumulated message
            if let Ok(text) = extract_body_from_raw(&current_message) {
                if !text.trim().is_empty() {
                    message_count += 1;
                    if !all_text.is_empty() {
                        all_text.push_str("\n\n---\n\n");
                    }
                    all_text.push_str(&text);
                }
            }
            current_message.clear();
        } else {
            current_message.extend_from_slice(line);
            current_message.push(b'\n');
        }
    }

    // Process last message
    if !current_message.is_empty() {
        if let Ok(text) = extract_body_from_raw(&current_message) {
            if !text.trim().is_empty() {
                message_count += 1;
                if !all_text.is_empty() {
                    all_text.push_str("\n\n---\n\n");
                }
                all_text.push_str(&text);
            }
        }
    }

    if all_text.trim().is_empty() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("No extractable text in MBOX '{}' ({message_count} messages)", path.display()),
        });
    }

    Ok(all_text)
}

/// Extract body text from parsed mail.
fn extract_body_from_mail(mail: &mailparse::ParsedMail) -> Result<String> {
    // If this is a multipart message, recurse into parts
    if !mail.subparts.is_empty() {
        // Prefer text/plain
        for part in &mail.subparts {
            let ct = part.ctype.mimetype.to_lowercase();
            if ct == "text/plain" {
                if let Ok(body) = part.get_body() {
                    if !body.trim().is_empty() {
                        return Ok(body);
                    }
                }
            }
        }

        // Fall back to text/html, strip tags
        for part in &mail.subparts {
            let ct = part.ctype.mimetype.to_lowercase();
            if ct == "text/html" {
                if let Ok(body) = part.get_body() {
                    let text = super::html::strip_html(&body);
                    if !text.trim().is_empty() {
                        return Ok(text);
                    }
                }
            }
        }

        // Recurse into multipart/* sub-parts
        for part in &mail.subparts {
            if !part.subparts.is_empty() {
                if let Ok(body) = extract_body_from_mail(part) {
                    if !body.trim().is_empty() {
                        return Ok(body);
                    }
                }
            }
        }
    }

    // Single-part message
    let ct = mail.ctype.mimetype.to_lowercase();
    if let Ok(body) = mail.get_body() {
        if ct.contains("html") {
            return Ok(super::html::strip_html(&body));
        }
        return Ok(body);
    }

    Ok(String::new())
}

/// Extract body from raw email bytes.
fn extract_body_from_raw(raw: &[u8]) -> Result<String> {
    let parsed = mailparse::parse_mail(raw).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to parse email message: {e}"),
    })?;
    extract_body_from_mail(&parsed)
}
