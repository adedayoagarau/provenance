use crate::utils::errors::{ProvenanceError, Result};
use scraper::{Html, Selector};
use std::path::Path;

/// Extract plain text from an HTML file, stripping all tags.
pub fn extract(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(strip_html(&content))
}

/// Strip HTML tags and return plain text.
pub fn strip_html(html_content: &str) -> String {
    let document = Html::parse_document(html_content);

    // Try to extract from <body> if it exists, otherwise use full document
    let body_selector = Selector::parse("body").unwrap();
    let root = if let Some(body) = document.select(&body_selector).next() {
        body.text().collect::<Vec<_>>().join(" ")
    } else {
        document.root_element().text().collect::<Vec<_>>().join(" ")
    };

    let mut clean = root;

    // Simple approach: extract text excluding script/style elements
    let document = Html::parse_document(html_content);
    let mut parts = Vec::new();

    fn collect_text(node: scraper::ElementRef, skip_selector: &Selector, parts: &mut Vec<String>) {
        // Skip script/style elements
        if skip_selector.matches(&node) {
            return;
        }

        for child in node.children() {
            if let Some(text) = child.value().as_text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
            if let Some(element) = scraper::ElementRef::wrap(child) {
                collect_text(element, skip_selector, parts);
            }
        }
    }

    let skip = Selector::parse("script, style, noscript, head").unwrap();
    if let Some(body) = document.select(&body_selector).next() {
        parts.clear();
        collect_text(body, &skip, &mut parts);
        clean = parts.join(" ");
    }

    // Decode HTML entities (scraper handles this automatically)
    // Normalize whitespace
    let result: String = clean
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    result
}
