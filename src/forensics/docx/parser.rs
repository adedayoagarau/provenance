use crate::utils::errors::{ProvenanceError, Result};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// Raw XML content extracted from a DOCX ZIP archive.
#[derive(Debug, Clone, Default)]
pub struct DocxArchiveContents {
    pub document_xml: Option<String>,
    pub core_xml: Option<String>,
    pub app_xml: Option<String>,
    pub settings_xml: Option<String>,
    pub styles_xml: Option<String>,
}

/// Open a .docx file as a ZIP archive and extract all forensically relevant XML parts.
pub fn extract_archive_contents(path: &Path) -> Result<DocxArchiveContents> {
    let file = std::fs::File::open(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| ProvenanceError::ExtractionError {
        reason: format!("Failed to open '{}' as ZIP archive: {e}", path.display()),
    })?;

    if archive.by_name("[Content_Types].xml").is_err() {
        return Err(ProvenanceError::ExtractionError {
            reason: format!(
                "'{}' is not a valid DOCX file (missing [Content_Types].xml)",
                path.display()
            ),
        });
    }

    let mut contents = DocxArchiveContents::default();
    contents.document_xml = read_zip_entry(&mut archive, "word/document.xml");
    contents.core_xml = read_zip_entry(&mut archive, "docProps/core.xml");
    contents.app_xml = read_zip_entry(&mut archive, "docProps/app.xml");
    contents.settings_xml = read_zip_entry(&mut archive, "word/settings.xml");
    contents.styles_xml = read_zip_entry(&mut archive, "word/styles.xml");

    Ok(contents)
}

fn read_zip_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut content = String::new();
    entry.read_to_string(&mut content).ok()?;
    Some(content)
}

/// Helper: get local name as owned String from a quick-xml LocalName.
fn local_name_to_string(name: quick_xml::name::LocalName<'_>) -> String {
    String::from_utf8_lossy(name.into_inner()).to_string()
}

/// Extract a simple XML element value using quick-xml.
pub fn extract_xml_element_text(xml: &str, target_local_name: &str) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut found = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = local_name_to_string(e.local_name());
                if name == target_local_name {
                    found = true;
                }
            }
            Ok(Event::Text(ref e)) if found => {
                if let Ok(text) = e.unescape() {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
                found = false;
            }
            Ok(Event::End(_)) if found => {
                found = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Extract all values of a specific attribute from elements matching a local name.
pub fn extract_element_attributes(
    xml: &str,
    element_local_name: &str,
    attr_names: &[&str],
) -> Vec<HashMap<String, String>> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut results = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = local_name_to_string(e.local_name());
                if name == element_local_name {
                    let mut attrs = HashMap::new();
                    for attr in e.attributes().flatten() {
                        let key = local_name_to_string(attr.key.local_name());
                        if attr_names.contains(&key.as_str()) {
                            if let Ok(val) = attr.unescape_value() {
                                attrs.insert(key, val.to_string());
                            }
                        }
                    }
                    if !attrs.is_empty() {
                        results.push(attrs);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    results
}

/// Parse paragraphs from document.xml, returning structured data about each paragraph.
pub fn parse_document_paragraphs(document_xml: &str) -> Vec<ParsedParagraph> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(document_xml);
    let mut buf = Vec::new();
    let mut paragraphs = Vec::new();

    let mut current_paragraph: Option<ParsedParagraph> = None;
    let mut current_run: Option<ParsedRun> = None;
    let mut in_run_props = false;
    let mut in_para_props = false;
    let mut capturing_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = local_name_to_string(e.local_name());

                match local.as_str() {
                    "p" => {
                        if let Some(para) = current_paragraph.take() {
                            paragraphs.push(para);
                        }
                        let mut para = ParsedParagraph::default();
                        for attr in e.attributes().flatten() {
                            let key = local_name_to_string(attr.key.local_name());
                            if let Ok(val) = attr.unescape_value() {
                                match key.as_str() {
                                    "rsidR" => para.rsid_r = Some(val.to_string()),
                                    "rsidRDefault" => {
                                        para.rsid_r_default = Some(val.to_string())
                                    }
                                    "rsidP" => para.rsid_p = Some(val.to_string()),
                                    _ => {}
                                }
                            }
                        }
                        current_paragraph = Some(para);
                    }
                    "pPr" => {
                        in_para_props = true;
                    }
                    "r" => {
                        let mut run = ParsedRun::default();
                        for attr in e.attributes().flatten() {
                            let key = local_name_to_string(attr.key.local_name());
                            if let Ok(val) = attr.unescape_value() {
                                match key.as_str() {
                                    "rsidR" => run.rsid_r = Some(val.to_string()),
                                    "rsidRPr" => run.rsid_r_pr = Some(val.to_string()),
                                    "rsidDel" => run.rsid_del = Some(val.to_string()),
                                    _ => {}
                                }
                            }
                        }
                        current_run = Some(run);
                    }
                    "rPr" => {
                        if current_run.is_some() {
                            in_run_props = true;
                        }
                    }
                    "t" => {
                        capturing_text = true;
                    }
                    "rFonts" if in_run_props => {
                        if let Some(ref mut run) = current_run {
                            for attr in e.attributes().flatten() {
                                let key = local_name_to_string(attr.key.local_name());
                                if key == "ascii" || key == "hAnsi" || key == "cs" {
                                    if let Ok(val) = attr.unescape_value() {
                                        run.font_name = Some(val.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    "sz" if in_run_props => {
                        if let Some(ref mut run) = current_run {
                            for attr in e.attributes().flatten() {
                                let key = local_name_to_string(attr.key.local_name());
                                if key == "val" {
                                    if let Ok(val) = attr.unescape_value() {
                                        run.font_size = val.parse().ok();
                                    }
                                }
                            }
                        }
                    }
                    "lang" if in_run_props || in_para_props => {
                        if let Some(ref mut run) = current_run {
                            for attr in e.attributes().flatten() {
                                let key = local_name_to_string(attr.key.local_name());
                                if key == "val" {
                                    if let Ok(val) = attr.unescape_value() {
                                        run.language = Some(val.to_string());
                                    }
                                }
                            }
                        }
                    }
                    "b" if in_run_props => {
                        if let Some(ref mut run) = current_run {
                            run.bold = true;
                        }
                    }
                    "i" if in_run_props => {
                        if let Some(ref mut run) = current_run {
                            run.italic = true;
                        }
                    }
                    "color" if in_run_props => {
                        if let Some(ref mut run) = current_run {
                            for attr in e.attributes().flatten() {
                                let key = local_name_to_string(attr.key.local_name());
                                if key == "val" {
                                    if let Ok(val) = attr.unescape_value() {
                                        run.color = Some(val.to_string());
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if capturing_text {
                    if let Ok(text) = e.unescape() {
                        if let Some(ref mut run) = current_run {
                            run.text.push_str(&text);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name_to_string(e.local_name());
                match local.as_str() {
                    "p" => {
                        if let Some(run) = current_run.take() {
                            if let Some(ref mut para) = current_paragraph {
                                para.runs.push(run);
                            }
                        }
                        if let Some(para) = current_paragraph.take() {
                            paragraphs.push(para);
                        }
                    }
                    "r" => {
                        if let Some(run) = current_run.take() {
                            if let Some(ref mut para) = current_paragraph {
                                para.runs.push(run);
                            }
                        }
                    }
                    "rPr" => {
                        in_run_props = false;
                    }
                    "pPr" => {
                        in_para_props = false;
                    }
                    "t" => {
                        capturing_text = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if let Some(para) = current_paragraph.take() {
        paragraphs.push(para);
    }

    paragraphs
}

/// A parsed paragraph from document.xml.
#[derive(Debug, Clone, Default)]
pub struct ParsedParagraph {
    pub rsid_r: Option<String>,
    pub rsid_r_default: Option<String>,
    pub rsid_p: Option<String>,
    pub runs: Vec<ParsedRun>,
}

impl ParsedParagraph {
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    pub fn word_count(&self) -> usize {
        self.text().split_whitespace().count()
    }
}

/// A parsed run (w:r) from document.xml.
#[derive(Debug, Clone, Default)]
pub struct ParsedRun {
    pub rsid_r: Option<String>,
    pub rsid_r_pr: Option<String>,
    pub rsid_del: Option<String>,
    pub text: String,
    pub font_name: Option<String>,
    pub font_size: Option<u32>,
    pub language: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub color: Option<String>,
}

/// Extract rsid values from word/settings.xml <w:rsids> element.
pub fn extract_settings_rsids(settings_xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(settings_xml);
    let mut buf = Vec::new();
    let mut rsids = Vec::new();
    let mut in_rsids = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = local_name_to_string(e.local_name());
                match local.as_str() {
                    "rsids" => {
                        in_rsids = true;
                    }
                    "rsidRoot" | "rsid" if in_rsids => {
                        for attr in e.attributes().flatten() {
                            let key = local_name_to_string(attr.key.local_name());
                            if key == "val" {
                                if let Ok(val) = attr.unescape_value() {
                                    rsids.push(val.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name_to_string(e.local_name());
                if local == "rsids" {
                    in_rsids = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    rsids
}

/// Parse style defaults from word/styles.xml.
pub fn parse_style_defaults(styles_xml: &str) -> StyleDefaults {
    let mut defaults = StyleDefaults::default();

    if let Some(pos) = styles_xml.find("docDefaults") {
        let section = &styles_xml[pos..];
        defaults.default_font = extract_attr_from_section(section, "rFonts", "ascii");
        defaults.default_font_size = extract_attr_from_section(section, "sz", "val")
            .and_then(|v| v.parse().ok());
        defaults.default_language = extract_attr_from_section(section, "lang", "val");
        defaults.default_line_spacing = extract_attr_from_section(section, "spacing", "line")
            .and_then(|v| v.parse().ok());
        defaults.default_spacing_before =
            extract_attr_from_section(section, "spacing", "before")
                .and_then(|v| v.parse().ok());
        defaults.default_spacing_after =
            extract_attr_from_section(section, "spacing", "after")
                .and_then(|v| v.parse().ok());
    }

    defaults
}

fn extract_attr_from_section(xml: &str, element: &str, attr_name: &str) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = local_name_to_string(e.local_name());
                if local == element {
                    for attr in e.attributes().flatten() {
                        let key = local_name_to_string(attr.key.local_name());
                        if key == attr_name {
                            if let Ok(val) = attr.unescape_value() {
                                return Some(val.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Default style values extracted from styles.xml.
#[derive(Debug, Clone, Default)]
pub struct StyleDefaults {
    pub default_font: Option<String>,
    pub default_font_size: Option<u32>,
    pub default_language: Option<String>,
    pub default_line_spacing: Option<u32>,
    pub default_spacing_before: Option<u32>,
    pub default_spacing_after: Option<u32>,
}
