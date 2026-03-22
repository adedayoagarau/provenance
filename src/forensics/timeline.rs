use serde::{Deserialize, Serialize};
use std::path::Path;

use super::metadata::FileMetadata;

/// Chronological timeline of a document's life.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTimeline {
    pub events: Vec<TimelineEvent>,
    /// Anomaly score for the timeline (0.0 = normal, 1.0 = highly anomalous)
    pub anomaly_score: f64,
    /// Detected anomalies
    pub anomalies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub epoch_secs: u64,
    pub event_type: EventType,
    pub source: EventSource,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Created,
    Modified,
    Accessed,
    Revision,
    Printed,
}

/// Where the event data comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    /// From filesystem metadata
    Filesystem,
    /// From embedded document metadata (DOCX, PDF)
    DocumentMetadata,
    /// Inferred from revision history
    RevisionHistory,
}

/// Construct a timeline from file metadata, correlating multiple sources.
pub fn construct(_path: &Path, metadata: &FileMetadata) -> DocumentTimeline {
    let mut events = Vec::new();

    // Filesystem events
    if let Some(created) = metadata.created_epoch {
        events.push(TimelineEvent {
            epoch_secs: created,
            event_type: EventType::Created,
            source: EventSource::Filesystem,
            description: "File created (filesystem)".to_string(),
        });
    }

    if let Some(modified) = metadata.modified_epoch {
        events.push(TimelineEvent {
            epoch_secs: modified,
            event_type: EventType::Modified,
            source: EventSource::Filesystem,
            description: "File modified (filesystem)".to_string(),
        });
    }

    if let Some(accessed) = metadata.accessed_epoch {
        events.push(TimelineEvent {
            epoch_secs: accessed,
            event_type: EventType::Accessed,
            source: EventSource::Filesystem,
            description: "File accessed (filesystem)".to_string(),
        });
    }

    // Document metadata events
    if let Some(ref doc_meta) = metadata.document_metadata {
        if let Some(ref creation_date) = doc_meta.creation_date {
            if let Some(epoch) = parse_date_to_epoch(creation_date) {
                events.push(TimelineEvent {
                    epoch_secs: epoch,
                    event_type: EventType::Created,
                    source: EventSource::DocumentMetadata,
                    description: format!("Document created (metadata: {creation_date})"),
                });
            }
        }

        if let Some(ref mod_date) = doc_meta.modification_date {
            if let Some(epoch) = parse_date_to_epoch(mod_date) {
                events.push(TimelineEvent {
                    epoch_secs: epoch,
                    event_type: EventType::Modified,
                    source: EventSource::DocumentMetadata,
                    description: format!("Document modified (metadata: {mod_date})"),
                });
            }
        }

        // Infer revision events from revision count
        if let (Some(revisions), Some(ref creation_date), Some(ref mod_date)) = (
            doc_meta.revision_count,
            &doc_meta.creation_date,
            &doc_meta.modification_date,
        ) {
            if revisions > 1 {
                if let (Some(start), Some(end)) = (
                    parse_date_to_epoch(creation_date),
                    parse_date_to_epoch(mod_date),
                ) {
                    if end > start && revisions > 2 {
                        // Interpolate revision events
                        let interval = (end - start) / (revisions as u64 - 1).max(1);
                        for i in 1..revisions.min(10) {
                            // Cap at 10 interpolated events
                            events.push(TimelineEvent {
                                epoch_secs: start + interval * i as u64,
                                event_type: EventType::Revision,
                                source: EventSource::RevisionHistory,
                                description: format!("Revision {i} (interpolated from {revisions} total)"),
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort chronologically
    events.sort_by_key(|e| e.epoch_secs);

    // Detect timeline anomalies
    let (anomaly_score, anomalies) = analyze_timeline_anomalies(&events, metadata);

    DocumentTimeline {
        events,
        anomaly_score,
        anomalies,
    }
}

/// Analyze timeline for anomalies.
fn analyze_timeline_anomalies(
    events: &[TimelineEvent],
    metadata: &FileMetadata,
) -> (f64, Vec<String>) {
    let mut anomalies = Vec::new();
    let mut score: f64 = 0.0;

    // Check 1: Events out of logical order
    // Created should be before modified
    let fs_created = events.iter().find(|e| {
        matches!(e.event_type, EventType::Created)
            && matches!(e.source, EventSource::Filesystem)
    });
    let fs_modified = events.iter().find(|e| {
        matches!(e.event_type, EventType::Modified)
            && matches!(e.source, EventSource::Filesystem)
    });

    if let (Some(created), Some(modified)) = (fs_created, fs_modified) {
        if created.epoch_secs > modified.epoch_secs + 60 {
            anomalies.push("Filesystem creation date is after modification date".to_string());
            score += 0.3;
        }
    }

    // Check 2: Document metadata vs filesystem dates disagree
    let doc_created = events.iter().find(|e| {
        matches!(e.event_type, EventType::Created)
            && matches!(e.source, EventSource::DocumentMetadata)
    });
    if let (Some(fs_c), Some(doc_c)) = (fs_created, doc_created) {
        let diff = if fs_c.epoch_secs > doc_c.epoch_secs {
            fs_c.epoch_secs - doc_c.epoch_secs
        } else {
            doc_c.epoch_secs - fs_c.epoch_secs
        };
        // More than 7 days apart
        if diff > 7 * 86400 {
            anomalies.push(format!(
                "Filesystem and document creation dates differ by {} days",
                diff / 86400
            ));
            score += 0.2;
        }
    }

    // Check 3: Future dates
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for event in events {
        if event.epoch_secs > now + 86400 {
            anomalies.push(format!(
                "Event '{}' has a future date",
                event.description
            ));
            score += 0.4;
        }
    }

    // Check 4: Very old creation with recent modification (possible backdating)
    if let (Some(created), Some(modified)) = (metadata.created_epoch, metadata.modified_epoch) {
        let age = modified.saturating_sub(created);
        if age > 365 * 86400 * 10 {
            // 10+ years between creation and modification
            anomalies.push(format!(
                "File spans {} years between creation and last modification",
                age / (365 * 86400)
            ));
            score += 0.1;
        }
    }

    (score.min(1.0), anomalies)
}

/// Parse various date formats to epoch seconds.
fn parse_date_to_epoch(date_str: &str) -> Option<u64> {
    let cleaned = date_str
        .trim_start_matches("D:")
        .replace('T', " ")
        .replace('Z', "");

    // Try YYYY-MM-DD or YYYY/MM/DD
    let parts: Vec<&str> = cleaned.split(|c: char| !c.is_ascii_digit()).collect();
    if parts.len() >= 3 {
        let year: u64 = parts[0].parse().ok()?;
        let month: u64 = parts[1].parse().ok()?;
        let day: u64 = parts[2].parse().ok()?;

        if year >= 1970 && year <= 2100 && month >= 1 && month <= 12 && day >= 1 && day <= 31 {
            let epoch = (year - 1970) * 365 * 86400
                + (month - 1) * 30 * 86400
                + (day - 1) * 86400;
            return Some(epoch);
        }
    }

    // YYYYMMDD format
    if cleaned.len() >= 8 {
        if let (Ok(year), Ok(month), Ok(day)) = (
            cleaned[0..4].parse::<u64>(),
            cleaned[4..6].parse::<u64>(),
            cleaned[6..8].parse::<u64>(),
        ) {
            if year >= 1970 && year <= 2100 && month >= 1 && month <= 12 && day >= 1 && day <= 31 {
                let epoch = (year - 1970) * 365 * 86400
                    + (month - 1) * 30 * 86400
                    + (day - 1) * 86400;
                return Some(epoch);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_timeline() {
        let metadata = FileMetadata {
            file_name: "test.txt".into(),
            file_size: 100,
            created_epoch: Some(1700000000),
            modified_epoch: Some(1700001000),
            accessed_epoch: Some(1700002000),
            is_readonly: false,
            document_metadata: None,
        };

        let timeline = construct(Path::new("test.txt"), &metadata);
        assert_eq!(timeline.events.len(), 3);
        assert!(timeline.anomaly_score < 0.1); // Clean timeline

        // Events should be sorted chronologically
        for i in 1..timeline.events.len() {
            assert!(timeline.events[i].epoch_secs >= timeline.events[i - 1].epoch_secs);
        }
    }

    #[test]
    fn test_anomalous_timeline() {
        let metadata = FileMetadata {
            file_name: "suspicious.txt".into(),
            file_size: 100,
            created_epoch: Some(1700005000), // Created after modified!
            modified_epoch: Some(1700000000),
            accessed_epoch: None,
            is_readonly: false,
            document_metadata: None,
        };

        let timeline = construct(Path::new("test.txt"), &metadata);
        assert!(timeline.anomaly_score > 0.0);
        assert!(!timeline.anomalies.is_empty());
    }

    #[test]
    fn test_parse_dates() {
        assert!(parse_date_to_epoch("2024-01-15").is_some());
        assert!(parse_date_to_epoch("D:20240115").is_some());
        assert!(parse_date_to_epoch("garbage").is_none());
    }
}
