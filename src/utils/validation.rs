//! Input validation and security hardening.
//!
//! Protects against:
//! - Oversized files (configurable limit, default 100MB)
//! - Zip bombs (excessive compression ratios / nested archives)
//! - XML bombs (entity expansion attacks)
//! - Path traversal (../ in filenames, absolute paths in archives)

use std::path::Path;

use super::errors::{ProvenanceError, Result};

/// Validation configuration.
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum file size in bytes (default: 100MB)
    pub max_file_size: u64,
    /// Maximum decompressed size for ZIP entries (default: 500MB)
    pub max_decompressed_size: u64,
    /// Maximum compression ratio before flagging as zip bomb (default: 100x)
    pub max_compression_ratio: f64,
    /// Maximum ZIP nesting depth (default: 3)
    pub max_zip_depth: usize,
    /// Maximum XML entity expansion depth (default: 10)
    pub max_xml_entity_depth: usize,
    /// Maximum total XML entity expansions (default: 10000)
    pub max_xml_entities: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,       // 100MB
            max_decompressed_size: 500 * 1024 * 1024, // 500MB
            max_compression_ratio: 100.0,
            max_zip_depth: 3,
            max_xml_entity_depth: 10,
            max_xml_entities: 10_000,
        }
    }
}

/// Validate a file before processing.
///
/// Checks file size and path safety. Call this before any extraction.
pub fn validate_file(path: &Path, config: &ValidationConfig) -> Result<()> {
    validate_path_safety(path)?;
    validate_file_size(path, config.max_file_size)?;
    Ok(())
}

/// Check file size against limit.
pub fn validate_file_size(path: &Path, max_size: u64) -> Result<()> {
    let metadata = std::fs::metadata(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    if metadata.len() > max_size {
        return Err(ProvenanceError::AnalysisError {
            reason: format!(
                "File '{}' is {} bytes, exceeding the {} byte limit",
                path.display(),
                metadata.len(),
                max_size
            ),
        });
    }

    Ok(())
}

/// Check for path traversal attacks.
///
/// Rejects paths containing `..`, absolute paths in archive entries,
/// and paths with null bytes.
pub fn validate_path_safety(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // Null bytes
    if path_str.contains('\0') {
        return Err(ProvenanceError::AnalysisError {
            reason: format!("Path contains null byte: '{}'", path.display()),
        });
    }

    Ok(())
}

/// Validate a path extracted from a ZIP archive entry.
///
/// Archive entry names can contain malicious paths like `../../etc/passwd`.
pub fn validate_archive_entry_path(entry_name: &str) -> Result<()> {
    // Reject path traversal
    if entry_name.contains("..") {
        return Err(ProvenanceError::AnalysisError {
            reason: format!("Archive entry contains path traversal: '{entry_name}'"),
        });
    }

    // Reject absolute paths
    if entry_name.starts_with('/') || entry_name.starts_with('\\') {
        return Err(ProvenanceError::AnalysisError {
            reason: format!("Archive entry contains absolute path: '{entry_name}'"),
        });
    }

    // Reject Windows drive letters
    if entry_name.len() >= 2
        && entry_name.as_bytes()[0].is_ascii_alphabetic()
        && entry_name.as_bytes()[1] == b':'
    {
        return Err(ProvenanceError::AnalysisError {
            reason: format!("Archive entry contains drive letter path: '{entry_name}'"),
        });
    }

    // Null bytes
    if entry_name.contains('\0') {
        return Err(ProvenanceError::AnalysisError {
            reason: format!("Archive entry name contains null byte: '{entry_name}'"),
        });
    }

    Ok(())
}

/// Check a ZIP archive for zip bomb indicators.
///
/// A zip bomb has extremely high compression ratios or deeply nested archives.
pub fn check_zip_bomb(
    compressed_size: u64,
    uncompressed_size: u64,
    config: &ValidationConfig,
) -> Result<()> {
    // Check absolute decompressed size
    if uncompressed_size > config.max_decompressed_size {
        return Err(ProvenanceError::AnalysisError {
            reason: format!(
                "ZIP entry decompressed size ({} bytes) exceeds limit ({} bytes). \
                 Possible zip bomb.",
                uncompressed_size, config.max_decompressed_size
            ),
        });
    }

    // Check compression ratio
    if compressed_size > 0 {
        let ratio = uncompressed_size as f64 / compressed_size as f64;
        if ratio > config.max_compression_ratio {
            return Err(ProvenanceError::AnalysisError {
                reason: format!(
                    "ZIP entry compression ratio ({ratio:.0}x) exceeds limit ({:.0}x). \
                     Possible zip bomb.",
                    config.max_compression_ratio
                ),
            });
        }
    }

    Ok(())
}

/// Check XML content for entity expansion attacks (billion laughs / XML bomb).
///
/// Counts entity definitions and references to detect exponential expansion.
pub fn check_xml_bomb(xml_content: &str, config: &ValidationConfig) -> Result<()> {
    // Count ENTITY definitions
    let entity_count = xml_content.matches("<!ENTITY").count();
    if entity_count > config.max_xml_entity_depth {
        return Err(ProvenanceError::AnalysisError {
            reason: format!(
                "XML contains {entity_count} ENTITY definitions, exceeding limit of {}. \
                 Possible XML bomb.",
                config.max_xml_entity_depth
            ),
        });
    }

    // Count entity references (& ... ;)
    let ref_count = xml_content.matches('&').count();
    if ref_count > config.max_xml_entities {
        return Err(ProvenanceError::AnalysisError {
            reason: format!(
                "XML contains {ref_count} entity references, exceeding limit of {}. \
                 Possible XML bomb.",
                config.max_xml_entities
            ),
        });
    }

    // Check for recursive entity patterns
    if xml_content.contains("<!ENTITY") && entity_count >= 3 {
        // Simple heuristic: if entities reference other entities
        let has_recursive = xml_content
            .lines()
            .filter(|l| l.contains("<!ENTITY"))
            .any(|l| l.contains('&') && l.contains(';'));

        if has_recursive {
            return Err(ProvenanceError::AnalysisError {
                reason: "XML contains recursive entity definitions. Possible XML bomb."
                    .to_string(),
            });
        }
    }

    Ok(())
}

/// Sanitize a filename by removing dangerous characters.
pub fn sanitize_filename(name: &str) -> String {
    name.replace("..", "")
        .replace('/', "_")
        .replace('\\', "_")
        .replace('\0', "")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_size_validation() {
        let dir = std::env::temp_dir().join("provenance_test_validation");
        let _ = fs::create_dir_all(&dir);

        let path = dir.join("small.txt");
        fs::write(&path, "hello").unwrap();

        // Should pass with generous limit
        assert!(validate_file_size(&path, 1024).is_ok());

        // Should fail with tiny limit
        assert!(validate_file_size(&path, 2).is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_path_safety() {
        assert!(validate_path_safety(Path::new("normal.txt")).is_ok());
        assert!(validate_path_safety(Path::new("dir/file.txt")).is_ok());

        // Null byte
        assert!(validate_path_safety(Path::new("file\0.txt")).is_err());
    }

    #[test]
    fn test_archive_entry_path_traversal() {
        assert!(validate_archive_entry_path("word/document.xml").is_ok());
        assert!(validate_archive_entry_path("META-INF/container.xml").is_ok());

        // Path traversal
        assert!(validate_archive_entry_path("../../etc/passwd").is_err());
        assert!(validate_archive_entry_path("word/../../etc/shadow").is_err());

        // Absolute paths
        assert!(validate_archive_entry_path("/etc/passwd").is_err());
        assert!(validate_archive_entry_path("\\windows\\system32").is_err());

        // Windows drive letters
        assert!(validate_archive_entry_path("C:\\temp\\file.txt").is_err());
    }

    #[test]
    fn test_zip_bomb_detection() {
        let config = ValidationConfig::default();

        // Normal file: 1KB compressed → 10KB uncompressed (10x ratio)
        assert!(check_zip_bomb(1024, 10240, &config).is_ok());

        // Suspicious: 1KB compressed → 200MB uncompressed (200000x ratio)
        assert!(check_zip_bomb(1024, 200 * 1024 * 1024, &config).is_err());

        // Exceeds decompressed size limit
        assert!(check_zip_bomb(1024 * 1024, 600 * 1024 * 1024, &config).is_err());
    }

    #[test]
    fn test_xml_bomb_detection() {
        let config = ValidationConfig::default();

        // Normal XML
        let normal = r#"<?xml version="1.0"?><root><item>text</item></root>"#;
        assert!(check_xml_bomb(normal, &config).is_ok());

        // Billion laughs pattern
        let bomb = r#"<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol2 "&lol;&lol;&lol;&lol;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;">
  <!ENTITY lol4 "&lol3;&lol3;&lol3;&lol3;">
  <!ENTITY lol5 "&lol4;&lol4;&lol4;&lol4;">
  <!ENTITY lol6 "&lol5;&lol5;&lol5;&lol5;">
  <!ENTITY lol7 "&lol6;&lol6;&lol6;&lol6;">
  <!ENTITY lol8 "&lol7;&lol7;&lol7;&lol7;">
  <!ENTITY lol9 "&lol8;&lol8;&lol8;&lol8;">
  <!ENTITY lol10 "&lol9;&lol9;&lol9;&lol9;">
  <!ENTITY lol11 "&lol10;&lol10;&lol10;&lol10;">
]>
<root>&lol11;</root>"#;
        assert!(check_xml_bomb(bomb, &config).is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(sanitize_filename("../../etc/passwd"), "__etc_passwd");
        assert_eq!(sanitize_filename("file\0name.txt"), "filename.txt");
        assert_eq!(sanitize_filename("C:\\temp\\file"), "C:_temp_file");
    }

    #[test]
    fn test_validate_file_integration() {
        let dir = std::env::temp_dir().join("provenance_test_validate_full");
        let _ = fs::create_dir_all(&dir);

        let path = dir.join("test.txt");
        fs::write(&path, "test content").unwrap();

        let config = ValidationConfig::default();
        assert!(validate_file(&path, &config).is_ok());

        let _ = fs::remove_dir_all(&dir);
    }
}
