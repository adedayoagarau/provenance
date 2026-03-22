//! Capture session — stores and manages a sequence of writing process events.
//!
//! A session represents a single continuous writing period. Multiple sessions
//! can be aggregated into a full document history.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::events::{CaptureEvent, CaptureSource, EventType};
use crate::utils::errors::{ProvenanceError, Result};

/// A complete capture session containing all events from one writing period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Document being edited (filename or identifier).
    pub document_name: String,
    /// Which editor captured these events.
    pub source: CaptureSource,
    /// Session start time (ISO 8601).
    pub started_at: String,
    /// Session end time (ISO 8601), if ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// All captured events, ordered by sequence number.
    pub events: Vec<CaptureEvent>,
    /// Plugin version that captured these events.
    pub plugin_version: String,
    /// Capture schema version (for forward compatibility).
    pub schema_version: u32,
}

impl CaptureSession {
    /// Create a new empty session.
    pub fn new(
        session_id: &str,
        document_name: &str,
        source: CaptureSource,
        started_at: &str,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            document_name: document_name.to_string(),
            source,
            started_at: started_at.to_string(),
            ended_at: None,
            events: Vec::new(),
            plugin_version: "1.0.0".to_string(),
            schema_version: 1,
        }
    }

    /// Add an event to the session.
    pub fn push_event(&mut self, event: CaptureEvent) {
        self.events.push(event);
    }

    /// Total number of events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Duration of the session in milliseconds (from first to last event).
    pub fn duration_ms(&self) -> u64 {
        if self.events.len() < 2 {
            return 0;
        }
        let first = self.events.first().map(|e| e.elapsed_ms).unwrap_or(0);
        let last = self.events.last().map(|e| e.elapsed_ms).unwrap_or(0);
        last.saturating_sub(first)
    }

    /// Count events by type.
    pub fn event_counts(&self) -> HashMap<EventType, usize> {
        let mut counts = HashMap::new();
        for event in &self.events {
            *counts.entry(event.event_type).or_insert(0) += 1;
        }
        counts
    }

    /// Total characters typed (from keystroke/batch events).
    pub fn total_typed_chars(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::Keystroke | EventType::KeystrokeBatch))
            .filter_map(|e| e.text_length)
            .sum()
    }

    /// Total characters pasted.
    pub fn total_pasted_chars(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.event_type == EventType::Paste)
            .filter_map(|e| e.text_length)
            .sum()
    }

    /// Ratio of pasted content to total content additions.
    pub fn paste_ratio(&self) -> f64 {
        let typed = self.total_typed_chars() as f64;
        let pasted = self.total_pasted_chars() as f64;
        let total = typed + pasted;
        if total == 0.0 {
            return 0.0;
        }
        pasted / total
    }
}

/// A document's full capture history across multiple sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureHistory {
    /// Document name or identifier.
    pub document_name: String,
    /// All sessions for this document, ordered chronologically.
    pub sessions: Vec<CaptureSession>,
    /// Summary statistics across all sessions.
    pub summary: CaptureSummary,
}

/// Aggregated statistics across all capture sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSummary {
    /// Total number of sessions.
    pub session_count: usize,
    /// Total events across all sessions.
    pub total_events: usize,
    /// Total time spent writing (sum of session durations), in milliseconds.
    pub total_writing_time_ms: u64,
    /// Total characters typed.
    pub total_typed_chars: usize,
    /// Total characters pasted.
    pub total_pasted_chars: usize,
    /// Overall paste ratio.
    pub paste_ratio: f64,
    /// Total AI suggestions accepted.
    pub ai_suggestions_accepted: usize,
    /// Total saves (manual + auto).
    pub total_saves: usize,
    /// Distinct editor sources used.
    pub sources_used: Vec<CaptureSource>,
    /// Number of paste events.
    pub paste_event_count: usize,
    /// Number of undo events (high count may indicate experimentation/revision).
    pub undo_count: usize,
    /// Focus loss events (indicates task switching).
    pub focus_loss_count: usize,
}

/// Build a capture history from multiple sessions.
pub fn build_history(document_name: &str, sessions: Vec<CaptureSession>) -> CaptureHistory {
    let mut total_events = 0;
    let mut total_writing_time_ms = 0u64;
    let mut total_typed = 0usize;
    let mut total_pasted = 0usize;
    let mut ai_accepted = 0usize;
    let mut total_saves = 0usize;
    let mut paste_events = 0usize;
    let mut undo_count = 0usize;
    let mut focus_loss = 0usize;
    let mut sources = Vec::new();

    for session in &sessions {
        total_events += session.event_count();
        total_writing_time_ms += session.duration_ms();
        total_typed += session.total_typed_chars();
        total_pasted += session.total_pasted_chars();

        let counts = session.event_counts();
        ai_accepted += counts.get(&EventType::AiSuggestionAccepted).copied().unwrap_or(0);
        total_saves += counts.get(&EventType::Save).copied().unwrap_or(0)
            + counts.get(&EventType::AutoSave).copied().unwrap_or(0);
        paste_events += counts.get(&EventType::Paste).copied().unwrap_or(0);
        undo_count += counts.get(&EventType::Undo).copied().unwrap_or(0);
        focus_loss += counts.get(&EventType::FocusLoss).copied().unwrap_or(0);

        if !sources.contains(&session.source) {
            sources.push(session.source);
        }
    }

    let total_content = (total_typed + total_pasted) as f64;
    let paste_ratio = if total_content > 0.0 {
        total_pasted as f64 / total_content
    } else {
        0.0
    };

    CaptureHistory {
        document_name: document_name.to_string(),
        sessions,
        summary: CaptureSummary {
            session_count: 0, // Will be set below
            total_events,
            total_writing_time_ms,
            total_typed_chars: total_typed,
            total_pasted_chars: total_pasted,
            paste_ratio,
            ai_suggestions_accepted: ai_accepted,
            total_saves,
            sources_used: sources,
            paste_event_count: paste_events,
            undo_count,
            focus_loss_count: focus_loss,
        },
    }
}

/// Save a capture session to a JSON file.
pub fn save_session(session: &CaptureSession, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(path, json).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })
}

/// Load a capture session from a JSON file.
pub fn load_session(path: &Path) -> Result<CaptureSession> {
    let content = crate::utils::errors::read_file_string(path)?;
    let session: CaptureSession = serde_json::from_str(&content)?;
    Ok(session)
}

/// Load all capture sessions from a directory.
pub fn load_sessions_from_dir(dir: &Path) -> Result<Vec<CaptureSession>> {
    let mut sessions = Vec::new();

    let entries = std::fs::read_dir(dir).map_err(|e| ProvenanceError::IoWithPath {
        path: dir.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            match load_session(&path) {
                Ok(session) => sessions.push(session),
                Err(_) => continue, // Skip non-session JSON files
            }
        }
    }

    // Sort by start time
    sessions.sort_by(|a, b| a.started_at.cmp(&b.started_at));
    Ok(sessions)
}

/// Import a capture session from raw JSON (e.g., from a plugin HTTP POST).
pub fn import_from_json(json: &str) -> Result<CaptureSession> {
    let session: CaptureSession = serde_json::from_str(json).map_err(|e| {
        ProvenanceError::MetadataError {
            reason: format!("Invalid capture session JSON: {e}"),
        }
    })?;

    // Validate schema version
    if session.schema_version > 1 {
        return Err(ProvenanceError::MetadataError {
            reason: format!(
                "Unsupported capture schema version {} (max supported: 1)",
                session.schema_version
            ),
        });
    }

    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::events::*;

    fn mock_session() -> CaptureSession {
        let mut session = CaptureSession::new(
            "sess-001",
            "essay.docx",
            CaptureSource::GoogleDocs,
            "2026-03-22T10:00:00Z",
        );

        // Simulate typing 500 chars over 10 minutes
        for i in 0..50 {
            session.push_event(CaptureEvent {
                seq: i,
                timestamp: format!("2026-03-22T10:{:02}:00Z", i / 5),
                elapsed_ms: i * 12000,
                event_type: EventType::KeystrokeBatch,
                text_length: Some(10),
                position: None,
                source: CaptureSource::GoogleDocs,
            });
        }

        // Simulate a paste event of 200 chars
        session.push_event(CaptureEvent {
            seq: 50,
            timestamp: "2026-03-22T10:10:00Z".into(),
            elapsed_ms: 600_000,
            event_type: EventType::Paste,
            text_length: Some(200),
            position: Some(DocumentPosition {
                paragraph: 3,
                offset: 0,
                selection_length: 0,
            }),
            source: CaptureSource::GoogleDocs,
        });

        // Save event
        session.push_event(CaptureEvent {
            seq: 51,
            timestamp: "2026-03-22T10:10:30Z".into(),
            elapsed_ms: 630_000,
            event_type: EventType::Save,
            text_length: None,
            position: None,
            source: CaptureSource::GoogleDocs,
        });

        session
    }

    #[test]
    fn test_session_metrics() {
        let session = mock_session();
        assert_eq!(session.event_count(), 52);
        assert_eq!(session.total_typed_chars(), 500);
        assert_eq!(session.total_pasted_chars(), 200);

        let ratio = session.paste_ratio();
        assert!((ratio - 200.0 / 700.0).abs() < 0.01);
    }

    #[test]
    fn test_session_duration() {
        let session = mock_session();
        assert_eq!(session.duration_ms(), 630_000);
    }

    #[test]
    fn test_session_event_counts() {
        let session = mock_session();
        let counts = session.event_counts();
        assert_eq!(counts[&EventType::KeystrokeBatch], 50);
        assert_eq!(counts[&EventType::Paste], 1);
        assert_eq!(counts[&EventType::Save], 1);
    }

    #[test]
    fn test_session_save_load() {
        let session = mock_session();
        let dir = std::env::temp_dir().join("provenance_test_capture");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_session.json");

        save_session(&session, &path).unwrap();
        let loaded = load_session(&path).unwrap();

        assert_eq!(loaded.session_id, "sess-001");
        assert_eq!(loaded.event_count(), 52);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_import_from_json() {
        let session = mock_session();
        let json = serde_json::to_string(&session).unwrap();
        let imported = import_from_json(&json).unwrap();
        assert_eq!(imported.session_id, session.session_id);
    }

    #[test]
    fn test_import_rejects_future_schema() {
        let mut session = mock_session();
        session.schema_version = 99;
        let json = serde_json::to_string(&session).unwrap();
        assert!(import_from_json(&json).is_err());
    }

    #[test]
    fn test_build_history() {
        let s1 = mock_session();
        let mut s2 = mock_session();
        s2.session_id = "sess-002".into();
        s2.started_at = "2026-03-23T10:00:00Z".into();

        let history = build_history("essay.docx", vec![s1, s2]);
        assert_eq!(history.sessions.len(), 2);
        assert_eq!(history.summary.total_events, 104);
        assert_eq!(history.summary.total_typed_chars, 1000);
        assert_eq!(history.summary.total_pasted_chars, 400);
        assert_eq!(history.summary.paste_event_count, 2);
        assert_eq!(history.summary.total_saves, 2);
    }
}
