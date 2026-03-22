use crate::utils::errors::{ProvenanceError, Result};
use std::io::Read;
use std::path::Path;

/// Extract plain text from an EPUB file.
///
/// EPUB files are ZIP archives containing XHTML chapter files. We find the
/// content documents via the OPF manifest and extract text from each.
pub fn extract(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to open EPUB '{}' as ZIP: {e}", path.display()),
    })?;

    // Validate it's an EPUB (mimetype file should say "application/epub+zip")
    if let Ok(mut mimetype) = archive.by_name("mimetype") {
        let mut mime = String::new();
        let _ = mimetype.read_to_string(&mut mime);
        if !mime.trim().starts_with("application/epub+zip") {
            tracing::warn!("EPUB mimetype mismatch: {}", mime.trim());
        }
    }

    // Step 1: Read container.xml to find OPF path
    let container_xml = {
        let mut xml = String::new();
        if let Ok(mut f) = archive.by_name("META-INF/container.xml") {
            let _ = f.read_to_string(&mut xml);
        }
        xml
    };

    // Step 2: Parse OPF path from container
    let mut opf_path = String::new();
    let mut opf_dir = String::new();
    if let Some(start) = container_xml.find("full-path=\"") {
        let rest = &container_xml[start + 11..];
        if let Some(end) = rest.find('"') {
            opf_path = rest[..end].to_string();
            opf_dir = opf_path.rsplitn(2, '/').nth(1).unwrap_or("").to_string();
        }
    }

    // Step 3: Read OPF and find spine items
    let mut content_files: Vec<String> = Vec::new();
    if !opf_path.is_empty() {
        let opf_xml = {
            let mut xml = String::new();
            if let Ok(mut f) = archive.by_name(&opf_path) {
                let _ = f.read_to_string(&mut xml);
            }
            xml
        };
        if !opf_xml.is_empty() {
            content_files = find_spine_items(&opf_xml, &opf_dir);
        }
    }

    // Fallback: just find all XHTML/HTML files
    if content_files.is_empty() {
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm") {
                    if !name.contains("nav") && !name.contains("toc") {
                        content_files.push(name);
                    }
                }
            }
        }
        content_files.sort();
    }

    // Step 4: Extract text from each content file
    let mut all_text = String::new();
    for content_path in &content_files {
        if let Ok(mut file) = archive.by_name(content_path) {
            let mut html = String::new();
            if file.read_to_string(&mut html).is_ok() {
                let text = super::html::strip_html(&html);
                if !text.is_empty() {
                    if !all_text.is_empty() {
                        all_text.push_str("\n\n");
                    }
                    all_text.push_str(&text);
                }
            }
        }
    }

    if all_text.trim().is_empty() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!("No extractable text in EPUB '{}'", path.display()),
        });
    }

    Ok(all_text)
}

/// Parse OPF manifest to find spine content documents in reading order.
fn find_spine_items(opf_xml: &str, opf_dir: &str) -> Vec<String> {
    let mut manifest: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut spine_order: Vec<String> = Vec::new();

    for line in opf_xml.lines() {
        let line = line.trim();
        if line.contains("<item ") || line.contains("<item\t") {
            if let (Some(id), Some(href)) = (extract_attr(line, "id"), extract_attr(line, "href")) {
                let media = extract_attr(line, "media-type").unwrap_or_default();
                if media.contains("xhtml") || media.contains("html") {
                    manifest.insert(id, href);
                }
            }
        }
        if line.contains("<itemref ") {
            if let Some(idref) = extract_attr(line, "idref") {
                spine_order.push(idref);
            }
        }
    }

    spine_order
        .iter()
        .filter_map(|id| manifest.get(id))
        .map(|href| {
            if opf_dir.is_empty() {
                href.to_string()
            } else {
                format!("{opf_dir}/{href}")
            }
        })
        .collect()
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    if let Some(start) = tag.find(&pattern) {
        let rest = &tag[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}
