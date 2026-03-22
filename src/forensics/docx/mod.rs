pub mod rsid;
pub mod formatting;
pub mod structure;
pub mod profile;

use crate::utils::errors::{ProvenanceError, Result};
use std::io::Read;
use std::path::Path;

/// Open a .docx file and extract raw XML content from its ZIP entries.
pub struct DocxArchive {
    pub document_xml: String,
    pub settings_xml: Option<String>,
    pub styles_xml: Option<String>,
    pub core_xml: Option<String>,
    pub app_xml: Option<String>,
}

impl DocxArchive {
    /// Open and extract all relevant XML parts from a .docx file.
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(|e| ProvenanceError::IoWithPath {
            path: path.display().to_string(),
            source: e,
        })?;

        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| ProvenanceError::ExtractionError {
                reason: format!("Failed to open DOCX '{}' as ZIP: {e}", path.display()),
            })?;

        let document_xml = read_zip_entry(&mut archive, "word/document.xml").ok_or_else(|| {
            ProvenanceError::ExtractionError {
                reason: format!(
                    "'{}' is not a valid DOCX file (missing word/document.xml)",
                    path.display()
                ),
            }
        })?;

        Ok(DocxArchive {
            document_xml,
            settings_xml: read_zip_entry(&mut archive, "word/settings.xml"),
            styles_xml: read_zip_entry(&mut archive, "word/styles.xml"),
            core_xml: read_zip_entry(&mut archive, "docProps/core.xml"),
            app_xml: read_zip_entry(&mut archive, "docProps/app.xml"),
        })
    }
}

fn read_zip_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut content = String::new();
    entry.read_to_string(&mut content).ok()?;
    Some(content)
}
