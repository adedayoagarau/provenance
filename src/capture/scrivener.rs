//! Scrivener file watcher agent — monitors .scriv project directories for changes.
//!
//! Scrivener stores projects as directories containing RTF/text files, snapshots,
//! and a .scrivx XML manifest. This agent watches for file modifications and
//! translates them into CaptureEvents.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::events::{CaptureEvent, CaptureSource, EventType};
use crate::utils::errors::{ProvenanceError, Result};

/// Configuration for the Scrivener watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrivenerWatchConfig {
    /// Path to the .scriv project directory.
    pub project_path: String,
    /// Poll interval in seconds (Scrivener doesn't support filesystem events well).
    pub poll_interval_secs: u64,
    /// Whether to track snapshot creation as save events.
    pub track_snapshots: bool,
}

/// State of a Scrivener project at a point in time.
#[derive(Debug, Clone)]
pub struct ScrivenerState {
    /// File path → (size, modified_epoch) for all content files.
    pub files: HashMap<PathBuf, FileState>,
    /// Snapshot count per document.
    pub snapshot_counts: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct FileState {
    pub size: u64,
    pub modified_epoch: u64,
}

/// Scan a Scrivener project directory and return its current state.
pub fn scan_project(project_path: &Path) -> Result<ScrivenerState> {
    if !project_path.exists() {
        return Err(ProvenanceError::FileNotFound {
            path: project_path.display().to_string(),
        });
    }

    let mut files = HashMap::new();
    let mut snapshot_counts = HashMap::new();

    scan_dir_recursive(project_path, &mut files, &mut snapshot_counts)?;

    Ok(ScrivenerState {
        files,
        snapshot_counts,
    })
}

fn scan_dir_recursive(
    dir: &Path,
    files: &mut HashMap<PathBuf, FileState>,
    snapshot_counts: &mut HashMap<String, usize>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| ProvenanceError::IoWithPath {
        path: dir.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Track snapshots directory
            if path.file_name().and_then(|n| n.to_str()) == Some("Snapshots") {
                let parent_name = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let count = std::fs::read_dir(&path)
                    .map(|rd| rd.count())
                    .unwrap_or(0);
                snapshot_counts.insert(parent_name, count);
            }
            scan_dir_recursive(&path, files, snapshot_counts)?;
        } else if is_content_file(&path) {
            let metadata = std::fs::metadata(&path)?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            files.insert(
                path,
                FileState {
                    size: metadata.len(),
                    modified_epoch: modified,
                },
            );
        }
    }

    Ok(())
}

/// Check if a file is a Scrivener content file worth tracking.
fn is_content_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rtf") | Some("txt") | Some("md") | Some("rtfd") => true,
        Some("scrivx") => true, // Project manifest
        _ => false,
    }
}

/// Diff two project states and generate capture events for changes.
pub fn diff_states(
    old: &ScrivenerState,
    new: &ScrivenerState,
    session_elapsed_ms: u64,
    seq_start: u64,
) -> Vec<CaptureEvent> {
    let mut events = Vec::new();
    let mut seq = seq_start;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let timestamp = crate::scoring::engine::format_epoch_public(now);

    // Check for new or modified files
    for (path, new_state) in &new.files {
        match old.files.get(path) {
            None => {
                // New file — could be new document or paste
                let size_delta = new_state.size;
                events.push(CaptureEvent {
                    seq,
                    timestamp: timestamp.clone(),
                    elapsed_ms: session_elapsed_ms,
                    event_type: if size_delta > 5000 {
                        EventType::Paste // Large new file suggests external content
                    } else {
                        EventType::KeystrokeBatch
                    },
                    text_length: Some(size_delta as usize),
                    position: None,
                    source: CaptureSource::Scrivener,
                });
                seq += 1;
            }
            Some(old_state) => {
                if new_state.modified_epoch > old_state.modified_epoch {
                    // File was modified
                    let size_delta = (new_state.size as i64 - old_state.size as i64).unsigned_abs();
                    let grew = new_state.size > old_state.size;

                    if grew && size_delta > 2000 {
                        // Large growth — likely paste
                        events.push(CaptureEvent {
                            seq,
                            timestamp: timestamp.clone(),
                            elapsed_ms: session_elapsed_ms,
                            event_type: EventType::Paste,
                            text_length: Some(size_delta as usize),
                            position: None,
                            source: CaptureSource::Scrivener,
                        });
                    } else if grew {
                        // Normal growth — typing
                        events.push(CaptureEvent {
                            seq,
                            timestamp: timestamp.clone(),
                            elapsed_ms: session_elapsed_ms,
                            event_type: EventType::KeystrokeBatch,
                            text_length: Some(size_delta as usize),
                            position: None,
                            source: CaptureSource::Scrivener,
                        });
                    } else {
                        // Shrunk — deletion
                        events.push(CaptureEvent {
                            seq,
                            timestamp: timestamp.clone(),
                            elapsed_ms: session_elapsed_ms,
                            event_type: EventType::Delete,
                            text_length: Some(size_delta as usize),
                            position: None,
                            source: CaptureSource::Scrivener,
                        });
                    }
                    seq += 1;
                }
            }
        }
    }

    // Check for deleted files
    for path in old.files.keys() {
        if !new.files.contains_key(path) {
            events.push(CaptureEvent {
                seq,
                timestamp: timestamp.clone(),
                elapsed_ms: session_elapsed_ms,
                event_type: EventType::Delete,
                text_length: old.files.get(path).map(|s| s.size as usize),
                position: None,
                source: CaptureSource::Scrivener,
            });
            seq += 1;
        }
    }

    // Check for new snapshots (treated as save events)
    for (doc, new_count) in &new.snapshot_counts {
        let old_count = old.snapshot_counts.get(doc).copied().unwrap_or(0);
        if *new_count > old_count {
            events.push(CaptureEvent {
                seq,
                timestamp: timestamp.clone(),
                elapsed_ms: session_elapsed_ms,
                event_type: EventType::Save,
                text_length: None,
                position: None,
                source: CaptureSource::Scrivener,
            });
            seq += 1;
        }
    }

    events
}

/// Parse a .scrivx manifest to extract document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrivenerManifest {
    /// Project title.
    pub title: String,
    /// Number of binder items (documents/folders).
    pub binder_item_count: usize,
    /// Document IDs in binder order.
    pub document_ids: Vec<String>,
}

pub fn parse_manifest(scrivx_path: &Path) -> Result<ScrivenerManifest> {
    let content = crate::utils::errors::read_file_string(scrivx_path)?;

    // Simple XML parsing for key fields
    let title = extract_xml_text(&content, "Title").unwrap_or_else(|| "Untitled".to_string());

    let mut document_ids = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<BinderItem") {
            if let Some(id) = extract_xml_attr(trimmed, "ID") {
                document_ids.push(id);
            }
        }
    }

    Ok(ScrivenerManifest {
        title,
        binder_item_count: document_ids.len(),
        document_ids,
    })
}

fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

fn extract_xml_attr(tag_line: &str, attr: &str) -> Option<String> {
    let search = format!("{attr}=\"");
    let start = tag_line.find(&search)? + search.len();
    let end = tag_line[start..].find('"')? + start;
    Some(tag_line[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_content_file() {
        assert!(is_content_file(Path::new("doc.rtf")));
        assert!(is_content_file(Path::new("content.txt")));
        assert!(is_content_file(Path::new("project.scrivx")));
        assert!(!is_content_file(Path::new("image.png")));
        assert!(!is_content_file(Path::new("data.bin")));
    }

    #[test]
    fn test_diff_states_new_file() {
        let old = ScrivenerState {
            files: HashMap::new(),
            snapshot_counts: HashMap::new(),
        };

        let mut new_files = HashMap::new();
        new_files.insert(
            PathBuf::from("Files/Data/doc1.rtf"),
            FileState {
                size: 1500,
                modified_epoch: 1000,
            },
        );
        let new = ScrivenerState {
            files: new_files,
            snapshot_counts: HashMap::new(),
        };

        let events = diff_states(&old, &new, 0, 0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::KeystrokeBatch);
        assert_eq!(events[0].text_length, Some(1500));
    }

    #[test]
    fn test_diff_states_large_new_file() {
        let old = ScrivenerState {
            files: HashMap::new(),
            snapshot_counts: HashMap::new(),
        };

        let mut new_files = HashMap::new();
        new_files.insert(
            PathBuf::from("Files/Data/pasted.rtf"),
            FileState {
                size: 10000,
                modified_epoch: 1000,
            },
        );
        let new = ScrivenerState {
            files: new_files,
            snapshot_counts: HashMap::new(),
        };

        let events = diff_states(&old, &new, 0, 0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::Paste); // Large file = paste
    }

    #[test]
    fn test_diff_states_modified_file() {
        let mut old_files = HashMap::new();
        old_files.insert(
            PathBuf::from("doc.rtf"),
            FileState {
                size: 1000,
                modified_epoch: 100,
            },
        );
        let old = ScrivenerState {
            files: old_files,
            snapshot_counts: HashMap::new(),
        };

        let mut new_files = HashMap::new();
        new_files.insert(
            PathBuf::from("doc.rtf"),
            FileState {
                size: 1200,
                modified_epoch: 200,
            },
        );
        let new = ScrivenerState {
            files: new_files,
            snapshot_counts: HashMap::new(),
        };

        let events = diff_states(&old, &new, 5000, 0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::KeystrokeBatch);
        assert_eq!(events[0].text_length, Some(200));
    }

    #[test]
    fn test_diff_states_deleted_file() {
        let mut old_files = HashMap::new();
        old_files.insert(
            PathBuf::from("doc.rtf"),
            FileState {
                size: 500,
                modified_epoch: 100,
            },
        );
        let old = ScrivenerState {
            files: old_files,
            snapshot_counts: HashMap::new(),
        };

        let new = ScrivenerState {
            files: HashMap::new(),
            snapshot_counts: HashMap::new(),
        };

        let events = diff_states(&old, &new, 0, 0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::Delete);
    }

    #[test]
    fn test_diff_states_new_snapshot() {
        let mut old_snaps = HashMap::new();
        old_snaps.insert("doc1".to_string(), 2);
        let old = ScrivenerState {
            files: HashMap::new(),
            snapshot_counts: old_snaps,
        };

        let mut new_snaps = HashMap::new();
        new_snaps.insert("doc1".to_string(), 3);
        let new = ScrivenerState {
            files: HashMap::new(),
            snapshot_counts: new_snaps,
        };

        let events = diff_states(&old, &new, 0, 0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::Save);
    }

    #[test]
    fn test_extract_xml_text() {
        let xml = "<Root><Title>My Novel</Title><Other>stuff</Other></Root>";
        assert_eq!(extract_xml_text(xml, "Title"), Some("My Novel".to_string()));
        assert_eq!(extract_xml_text(xml, "Missing"), None);
    }

    #[test]
    fn test_extract_xml_attr() {
        let tag = r#"<BinderItem ID="42" Type="Text">"#;
        assert_eq!(extract_xml_attr(tag, "ID"), Some("42".to_string()));
        assert_eq!(extract_xml_attr(tag, "Type"), Some("Text".to_string()));
        assert_eq!(extract_xml_attr(tag, "Missing"), None);
    }
}
