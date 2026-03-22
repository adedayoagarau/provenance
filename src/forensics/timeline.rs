use anyhow::Result;
use std::path::Path;
use std::time::SystemTime;

use super::metadata::FileMetadata;

/// Chronological timeline of a document's life.
#[derive(Debug, Clone)]
pub struct DocumentTimeline {
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub timestamp: SystemTime,
    pub event_type: EventType,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum EventType {
    Created,
    Modified,
    Accessed,
}

/// Construct a timeline from file metadata.
pub fn construct(_path: &Path, metadata: &FileMetadata) -> Result<DocumentTimeline> {
    let mut events = Vec::new();

    if let Some(created) = metadata.created {
        events.push(TimelineEvent {
            timestamp: created,
            event_type: EventType::Created,
            description: "File created".to_string(),
        });
    }

    if let Some(modified) = metadata.modified {
        events.push(TimelineEvent {
            timestamp: modified,
            event_type: EventType::Modified,
            description: "File modified".to_string(),
        });
    }

    if let Some(accessed) = metadata.accessed {
        events.push(TimelineEvent {
            timestamp: accessed,
            event_type: EventType::Accessed,
            description: "File accessed".to_string(),
        });
    }

    events.sort_by_key(|e| e.timestamp);

    Ok(DocumentTimeline { events })
}
