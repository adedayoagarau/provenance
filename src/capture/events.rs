//! Capture event types — the shared data model for all editor plugins.
//!
//! Every plugin (Google Docs, Word, VS Code, Scrivener) emits events conforming
//! to this schema. Events are timestamped, typed, and carry optional location info.

use serde::{Deserialize, Serialize};

/// A single captured writing event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureEvent {
    /// Monotonically increasing event sequence number within this session.
    pub seq: u64,
    /// ISO 8601 timestamp when the event occurred.
    pub timestamp: String,
    /// Milliseconds since session start (for timing analysis).
    pub elapsed_ms: u64,
    /// Type of event.
    pub event_type: EventType,
    /// Text content involved (if applicable; redacted for privacy by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_length: Option<usize>,
    /// Cursor/selection position at time of event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<DocumentPosition>,
    /// Source plugin that generated this event.
    pub source: CaptureSource,
}

/// Types of writing process events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Text was typed character-by-character.
    Keystroke,
    /// A batch of keystrokes (aggregated for efficiency; includes char count).
    KeystrokeBatch,
    /// Text was pasted from clipboard.
    Paste,
    /// Text was deleted (backspace/delete key).
    Delete,
    /// A block of text was cut.
    Cut,
    /// Undo action.
    Undo,
    /// Redo action.
    Redo,
    /// Cursor moved without text change.
    CursorMove,
    /// Selection was created or changed.
    Selection,
    /// Document was saved.
    Save,
    /// Autosave triggered.
    AutoSave,
    /// Writing session started.
    SessionStart,
    /// Writing session ended (tab closed, plugin detached).
    SessionEnd,
    /// Focus gained — editor window came to foreground.
    FocusGain,
    /// Focus lost — user switched away from editor.
    FocusLoss,
    /// Find/replace operation.
    FindReplace,
    /// Formatting change (bold, italic, heading, etc.).
    FormatChange,
    /// Comment added to document.
    Comment,
    /// Suggestion/tracked change accepted.
    SuggestionAccepted,
    /// Suggestion/tracked change rejected.
    SuggestionRejected,
    /// Spell check correction applied.
    SpellCorrect,
    /// AI autocomplete/suggestion accepted (Copilot, Docs suggestions, etc.).
    AiSuggestionAccepted,
    /// AI autocomplete/suggestion dismissed.
    AiSuggestionDismissed,
    /// Plugin-specific event not covered above.
    Custom,
}

impl EventType {
    /// Whether this event represents text content entering the document.
    pub fn is_content_addition(&self) -> bool {
        matches!(
            self,
            EventType::Keystroke
                | EventType::KeystrokeBatch
                | EventType::Paste
                | EventType::AiSuggestionAccepted
                | EventType::SpellCorrect
        )
    }

    /// Whether this event represents text leaving the document.
    pub fn is_content_removal(&self) -> bool {
        matches!(
            self,
            EventType::Delete | EventType::Cut | EventType::Undo
        )
    }

    /// Whether this event suggests non-organic text insertion.
    pub fn is_external_insertion(&self) -> bool {
        matches!(
            self,
            EventType::Paste | EventType::AiSuggestionAccepted
        )
    }
}

/// Position within the document at time of event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPosition {
    /// Paragraph index (0-based).
    pub paragraph: usize,
    /// Character offset within paragraph.
    pub offset: usize,
    /// Selection length (0 = cursor, >0 = selected region).
    #[serde(default)]
    pub selection_length: usize,
}

/// Which editor plugin captured this event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureSource {
    GoogleDocs,
    MicrosoftWord,
    VSCode,
    Scrivener,
    /// Generic source for testing or third-party integrations.
    Generic,
}

impl CaptureSource {
    pub fn label(&self) -> &'static str {
        match self {
            CaptureSource::GoogleDocs => "Google Docs",
            CaptureSource::MicrosoftWord => "Microsoft Word",
            CaptureSource::VSCode => "VS Code",
            CaptureSource::Scrivener => "Scrivener",
            CaptureSource::Generic => "Generic",
        }
    }
}

/// Metadata about a paste event for forensic analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasteMetadata {
    /// Number of characters pasted.
    pub char_count: usize,
    /// Number of words pasted (estimated).
    pub word_count: usize,
    /// Whether the paste replaced existing selected text.
    pub replaced_selection: bool,
    /// Paragraph range where paste landed.
    pub target_paragraphs: (usize, usize),
}

/// Metadata about an AI suggestion event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSuggestionMetadata {
    /// AI tool that generated the suggestion (e.g., "Copilot", "Docs Smart Compose").
    pub tool_name: String,
    /// Number of characters in the suggestion.
    pub char_count: usize,
    /// Whether the full suggestion was accepted or only part of it.
    pub partial_accept: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_classification() {
        assert!(EventType::Keystroke.is_content_addition());
        assert!(EventType::Paste.is_content_addition());
        assert!(EventType::Paste.is_external_insertion());
        assert!(!EventType::Keystroke.is_external_insertion());
        assert!(EventType::Delete.is_content_removal());
        assert!(!EventType::Save.is_content_addition());
        assert!(!EventType::Save.is_content_removal());
        assert!(EventType::AiSuggestionAccepted.is_external_insertion());
    }

    #[test]
    fn test_capture_event_serialization() {
        let event = CaptureEvent {
            seq: 1,
            timestamp: "2026-03-22T10:00:00Z".into(),
            elapsed_ms: 0,
            event_type: EventType::Keystroke,
            text_length: Some(1),
            position: Some(DocumentPosition {
                paragraph: 0,
                offset: 15,
                selection_length: 0,
            }),
            source: CaptureSource::GoogleDocs,
        };

        let json = serde_json::to_string(&event).unwrap();
        let recovered: CaptureEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.seq, 1);
        assert_eq!(recovered.event_type, EventType::Keystroke);
        assert_eq!(recovered.source, CaptureSource::GoogleDocs);
    }

    #[test]
    fn test_source_labels() {
        assert_eq!(CaptureSource::GoogleDocs.label(), "Google Docs");
        assert_eq!(CaptureSource::Scrivener.label(), "Scrivener");
    }
}
