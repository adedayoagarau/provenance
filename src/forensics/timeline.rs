use serde::{Deserialize, Serialize};
use std::path::Path;

use super::metadata::FileMetadata;

/// Chronological timeline of a document's life.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTimeline {
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub epoch_secs: u64,
    pub event_type: EventType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Created,
    Modified,
    Accessed,
}

/// Construct a timeline from file metadata.
pub fn construct(_path: &Path, metadata: &FileMetadata) -> DocumentTimeline {
    let mut events = Vec::new();

    if let Some(created) = metadata.created_epoch {
        events.push(TimelineEvent {
            epoch_secs: created,
            event_type: EventType::Created,
            description: "File created".to_string(),
        });
    }

    if let Some(modified) = metadata.modified_epoch {
        events.push(TimelineEvent {
            epoch_secs: modified,
            event_type: EventType::Modified,
            description: "File modified".to_string(),
        });
    }

    if let Some(accessed) = metadata.accessed_epoch {
        events.push(TimelineEvent {
            epoch_secs: accessed,
            event_type: EventType::Accessed,
            description: "File accessed".to_string(),
        });
    }

    events.sort_by_key(|e| e.epoch_secs);

    DocumentTimeline { events }
}
