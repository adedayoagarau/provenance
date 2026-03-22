use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Extract text from chat/messaging export files.
///
/// Supports common export formats from WhatsApp, Telegram, Discord, Slack,
/// and Twitter/X. Auto-detects format based on content structure.
pub fn extract(path: &Path) -> Result<String> {
    let ext = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "json" => extract_json_chat(path),
        "csv" => extract_csv_chat(path),
        _ => Err(ProvenanceError::UnsupportedFormat {
            format: format!("chat export .{ext}"),
        }),
    }
}

/// Extract text from JSON-based chat exports (Discord, Slack, Telegram, Twitter).
fn extract_json_chat(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        ProvenanceError::ExtractionError {
            reason: format!("Failed to parse JSON '{}': {e}", path.display()),
        }
    })?;

    let mut messages = Vec::new();

    // Try common JSON structures
    if let Some(arr) = value.as_array() {
        // Array of messages (Discord, Slack style)
        for msg in arr {
            if let Some(text) = extract_message_text(msg) {
                messages.push(text);
            }
        }
    } else if let Some(obj) = value.as_object() {
        // Object with messages array (Telegram, Twitter style)
        for key in &["messages", "tweets", "data", "chats", "items"] {
            if let Some(arr) = obj.get(*key).and_then(|v| v.as_array()) {
                for msg in arr {
                    if let Some(text) = extract_message_text(msg) {
                        messages.push(text);
                    }
                }
                if !messages.is_empty() {
                    break;
                }
            }
        }
    }

    if messages.is_empty() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("No messages found in JSON chat export '{}'", path.display()),
        });
    }

    Ok(messages.join("\n\n"))
}

/// Extract text content from a single message JSON object.
fn extract_message_text(msg: &serde_json::Value) -> Option<String> {
    // Try common field names for message content
    let text_fields = ["content", "text", "body", "message", "full_text", "tweet"];

    for field in &text_fields {
        if let Some(text) = msg.get(field).and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // Telegram: text can be an array of objects
    if let Some(arr) = msg.get("text").and_then(|v| v.as_array()) {
        let parts: Vec<String> = arr
            .iter()
            .filter_map(|part| {
                if let Some(s) = part.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = part.as_object() {
                    obj.get("text").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        if !parts.is_empty() {
            return Some(parts.join(""));
        }
    }

    None
}

/// Extract text from CSV chat exports (WhatsApp, generic).
fn extract_csv_chat(path: &Path) -> Result<String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .from_path(path)
        .map_err(|e| ProvenanceError::ExtractionError {
            reason: format!("Failed to read CSV '{}': {e}", path.display()),
        })?;

    let headers = reader.headers().map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to read CSV headers: {e}"),
    })?.clone();

    // Find the column most likely to contain message text
    let text_col = find_text_column(&headers);

    let mut messages = Vec::new();
    for result in reader.records() {
        if let Ok(record) = result {
            if let Some(idx) = text_col {
                if let Some(text) = record.get(idx) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        messages.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    if messages.is_empty() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("No message text found in CSV '{}'", path.display()),
        });
    }

    Ok(messages.join("\n\n"))
}

/// Find the column index most likely to contain message text.
fn find_text_column(headers: &csv::StringRecord) -> Option<usize> {
    let text_names = ["content", "text", "body", "message", "msg", "comment"];

    for (i, header) in headers.iter().enumerate() {
        let h = header.to_lowercase();
        for name in &text_names {
            if h.contains(name) {
                return Some(i);
            }
        }
    }

    // Fall back to last column (common pattern)
    if headers.len() > 1 {
        Some(headers.len() - 1)
    } else {
        None
    }
}
