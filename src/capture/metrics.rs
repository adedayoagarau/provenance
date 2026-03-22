//! Capture metrics — derived forensic signals from process capture data.
//!
//! Converts raw capture events into forensic signals that feed into the
//! scoring model. These signals are robust to text-level evasion because
//! they measure the writing PROCESS, not the text itself.

use serde::{Deserialize, Serialize};

use super::events::EventType;
use super::session::{CaptureHistory, CaptureSession};

/// Process metrics derived from capture data, suitable for scoring integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetrics {
    /// Total writing time across all sessions (minutes).
    pub total_writing_minutes: f64,
    /// Number of distinct writing sessions.
    pub session_count: usize,
    /// Average session duration (minutes).
    pub avg_session_minutes: f64,
    /// Characters typed per minute (excludes paste).
    pub typing_speed_cpm: f64,
    /// Ratio of pasted content to total content (0.0 = all typed, 1.0 = all pasted).
    pub paste_ratio: f64,
    /// Number of distinct paste events.
    pub paste_event_count: usize,
    /// Largest single paste size (characters).
    pub largest_paste_chars: usize,
    /// Undo frequency (undos per 1000 typed characters).
    pub undo_rate: f64,
    /// Focus loss frequency (per hour of writing).
    pub focus_loss_per_hour: f64,
    /// AI suggestion acceptance count.
    pub ai_suggestions_accepted: usize,
    /// AI content ratio (AI-accepted chars / total content chars).
    pub ai_content_ratio: f64,
    /// Revision intensity (deletes + undos per 1000 typed chars).
    pub revision_intensity: f64,
    /// Inter-keystroke interval statistics (milliseconds).
    pub keystroke_timing: Option<KeystrokeTiming>,
    /// Process confidence — how much capture data is available (0.0-1.0).
    pub capture_confidence: f64,
    /// Flags for anomalous patterns.
    pub flags: Vec<ProcessFlag>,
}

/// Keystroke timing statistics for behavioral biometrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeystrokeTiming {
    /// Mean inter-keystroke interval (ms).
    pub mean_iki_ms: f64,
    /// Median inter-keystroke interval (ms).
    pub median_iki_ms: f64,
    /// Standard deviation of IKI.
    pub stddev_iki_ms: f64,
    /// Coefficient of variation of IKI.
    pub cv_iki: f64,
    /// Percentage of very fast intervals (<50ms, potential paste-as-type).
    pub fast_interval_ratio: f64,
    /// Percentage of long pauses (>5000ms, thinking/distraction).
    pub pause_ratio: f64,
}

/// Anomalous patterns detected in process data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessFlag {
    /// Type of flag.
    pub flag_type: ProcessFlagType,
    /// Severity.
    pub severity: ProcessFlagSeverity,
    /// Human-readable description (question framing).
    pub description: String,
    /// Time range where the pattern was observed (elapsed_ms).
    pub time_range: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessFlagType {
    /// Large paste with no preceding typing in the area.
    BulkPaste,
    /// Unusually high paste-to-type ratio.
    HighPasteRatio,
    /// Typing speed exceeds human norms (>800 CPM sustained).
    UnrealisticTypingSpeed,
    /// Very uniform keystroke timing (possible automated input).
    UniformTiming,
    /// Significant AI suggestion usage.
    HighAiUsage,
    /// Very short writing time relative to document length.
    InsufficientWritingTime,
    /// No revision activity (no undos, no deletes) in a long session.
    NoRevisionActivity,
    /// Content appeared without corresponding input events.
    UnexplainedContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessFlagSeverity {
    Low,
    Medium,
    High,
}

/// Compute process metrics from a single session.
pub fn compute_session_metrics(session: &CaptureSession) -> ProcessMetrics {
    let duration_ms = session.duration_ms();
    let duration_min = duration_ms as f64 / 60_000.0;

    let typed = session.total_typed_chars();
    let pasted = session.total_pasted_chars();
    let total_content = typed + pasted;

    let counts = session.event_counts();
    let undo_count = counts.get(&EventType::Undo).copied().unwrap_or(0);
    let delete_count = counts.get(&EventType::Delete).copied().unwrap_or(0);
    let focus_loss_count = counts.get(&EventType::FocusLoss).copied().unwrap_or(0);
    let ai_accepted = counts.get(&EventType::AiSuggestionAccepted).copied().unwrap_or(0);
    let paste_count = counts.get(&EventType::Paste).copied().unwrap_or(0);

    let ai_chars: usize = session
        .events
        .iter()
        .filter(|e| e.event_type == EventType::AiSuggestionAccepted)
        .filter_map(|e| e.text_length)
        .sum();

    let typing_speed = if duration_min > 0.0 {
        typed as f64 / duration_min
    } else {
        0.0
    };

    let paste_ratio = if total_content > 0 {
        pasted as f64 / total_content as f64
    } else {
        0.0
    };

    let ai_ratio = if total_content > 0 {
        ai_chars as f64 / total_content as f64
    } else {
        0.0
    };

    let undo_rate = if typed > 0 {
        (undo_count as f64 / typed as f64) * 1000.0
    } else {
        0.0
    };

    let revision_intensity = if typed > 0 {
        ((undo_count + delete_count) as f64 / typed as f64) * 1000.0
    } else {
        0.0
    };

    let hours = duration_min / 60.0;
    let focus_loss_per_hour = if hours > 0.0 {
        focus_loss_count as f64 / hours
    } else {
        0.0
    };

    let largest_paste = session
        .events
        .iter()
        .filter(|e| e.event_type == EventType::Paste)
        .filter_map(|e| e.text_length)
        .max()
        .unwrap_or(0);

    let keystroke_timing = compute_keystroke_timing(session);

    // Determine capture confidence based on data richness
    let capture_confidence = compute_capture_confidence(session);

    // Detect anomalous patterns
    let flags = detect_flags(
        typing_speed,
        paste_ratio,
        largest_paste,
        ai_ratio,
        revision_intensity,
        duration_min,
        total_content,
        &keystroke_timing,
    );

    ProcessMetrics {
        total_writing_minutes: duration_min,
        session_count: 1,
        avg_session_minutes: duration_min,
        typing_speed_cpm: typing_speed,
        paste_ratio,
        paste_event_count: paste_count,
        largest_paste_chars: largest_paste,
        undo_rate,
        focus_loss_per_hour,
        ai_suggestions_accepted: ai_accepted,
        ai_content_ratio: ai_ratio,
        revision_intensity,
        keystroke_timing,
        capture_confidence,
        flags,
    }
}

/// Compute process metrics from a full capture history.
pub fn compute_history_metrics(history: &CaptureHistory) -> ProcessMetrics {
    if history.sessions.is_empty() {
        return empty_metrics();
    }

    if history.sessions.len() == 1 {
        return compute_session_metrics(&history.sessions[0]);
    }

    // Aggregate across sessions
    let mut total_min = 0.0f64;
    let mut total_typed = 0usize;
    let mut total_pasted = 0usize;
    let mut total_ai_chars = 0usize;
    let mut total_undos = 0usize;
    let mut total_deletes = 0usize;
    let mut total_focus_loss = 0usize;
    let mut total_ai_accepted = 0usize;
    let mut total_paste_events = 0usize;
    let mut largest_paste = 0usize;

    for session in &history.sessions {
        let dm = session.duration_ms() as f64 / 60_000.0;
        total_min += dm;
        total_typed += session.total_typed_chars();
        total_pasted += session.total_pasted_chars();

        let counts = session.event_counts();
        total_undos += counts.get(&EventType::Undo).copied().unwrap_or(0);
        total_deletes += counts.get(&EventType::Delete).copied().unwrap_or(0);
        total_focus_loss += counts.get(&EventType::FocusLoss).copied().unwrap_or(0);
        total_ai_accepted += counts.get(&EventType::AiSuggestionAccepted).copied().unwrap_or(0);
        total_paste_events += counts.get(&EventType::Paste).copied().unwrap_or(0);

        total_ai_chars += session
            .events
            .iter()
            .filter(|e| e.event_type == EventType::AiSuggestionAccepted)
            .filter_map(|e| e.text_length)
            .sum::<usize>();

        let session_largest = session
            .events
            .iter()
            .filter(|e| e.event_type == EventType::Paste)
            .filter_map(|e| e.text_length)
            .max()
            .unwrap_or(0);
        largest_paste = largest_paste.max(session_largest);
    }

    let total_content = total_typed + total_pasted;
    let session_count = history.sessions.len();
    let avg_session = total_min / session_count as f64;

    let typing_speed = if total_min > 0.0 {
        total_typed as f64 / total_min
    } else {
        0.0
    };

    let paste_ratio = if total_content > 0 {
        total_pasted as f64 / total_content as f64
    } else {
        0.0
    };

    let ai_ratio = if total_content > 0 {
        total_ai_chars as f64 / total_content as f64
    } else {
        0.0
    };

    let undo_rate = if total_typed > 0 {
        (total_undos as f64 / total_typed as f64) * 1000.0
    } else {
        0.0
    };

    let revision_intensity = if total_typed > 0 {
        ((total_undos + total_deletes) as f64 / total_typed as f64) * 1000.0
    } else {
        0.0
    };

    let hours = total_min / 60.0;
    let focus_loss_per_hour = if hours > 0.0 {
        total_focus_loss as f64 / hours
    } else {
        0.0
    };

    let capture_confidence = (session_count as f64 * 0.1 + total_min * 0.01).min(1.0);

    let flags = detect_flags(
        typing_speed,
        paste_ratio,
        largest_paste,
        ai_ratio,
        revision_intensity,
        total_min,
        total_content,
        &None,
    );

    ProcessMetrics {
        total_writing_minutes: total_min,
        session_count,
        avg_session_minutes: avg_session,
        typing_speed_cpm: typing_speed,
        paste_ratio,
        paste_event_count: total_paste_events,
        largest_paste_chars: largest_paste,
        undo_rate,
        focus_loss_per_hour,
        ai_suggestions_accepted: total_ai_accepted,
        ai_content_ratio: ai_ratio,
        revision_intensity,
        keystroke_timing: None, // Cross-session timing not meaningful
        capture_confidence,
        flags,
    }
}

fn compute_keystroke_timing(session: &CaptureSession) -> Option<KeystrokeTiming> {
    let keystroke_times: Vec<u64> = session
        .events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Keystroke | EventType::KeystrokeBatch))
        .map(|e| e.elapsed_ms)
        .collect();

    if keystroke_times.len() < 10 {
        return None;
    }

    let intervals: Vec<f64> = keystroke_times
        .windows(2)
        .map(|w| (w[1] as f64) - (w[0] as f64))
        .filter(|&d| d > 0.0 && d < 30_000.0) // Filter unreasonable gaps
        .collect();

    if intervals.len() < 5 {
        return None;
    }

    let n = intervals.len() as f64;
    let mean = intervals.iter().sum::<f64>() / n;

    let mut sorted = intervals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    let variance = intervals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();
    let cv = if mean > 0.0 { stddev / mean } else { 0.0 };

    let fast_count = intervals.iter().filter(|&&x| x < 50.0).count();
    let pause_count = intervals.iter().filter(|&&x| x > 5000.0).count();

    Some(KeystrokeTiming {
        mean_iki_ms: mean,
        median_iki_ms: median,
        stddev_iki_ms: stddev,
        cv_iki: cv,
        fast_interval_ratio: fast_count as f64 / n,
        pause_ratio: pause_count as f64 / n,
    })
}

fn compute_capture_confidence(session: &CaptureSession) -> f64 {
    let mut score = 0.0;

    // More events = more confidence
    let events = session.event_count() as f64;
    score += (events / 100.0).min(0.3);

    // Longer sessions = more confidence
    let minutes = session.duration_ms() as f64 / 60_000.0;
    score += (minutes / 30.0).min(0.3);

    // Having keystroke-level data is most valuable
    let has_keystrokes = session
        .events
        .iter()
        .any(|e| matches!(e.event_type, EventType::Keystroke | EventType::KeystrokeBatch));
    if has_keystrokes {
        score += 0.2;
    }

    // Having position data adds value
    let has_positions = session.events.iter().any(|e| e.position.is_some());
    if has_positions {
        score += 0.1;
    }

    // Having save events shows complete session capture
    let has_saves = session
        .events
        .iter()
        .any(|e| matches!(e.event_type, EventType::Save | EventType::AutoSave));
    if has_saves {
        score += 0.1;
    }

    score.min(1.0)
}

fn detect_flags(
    typing_speed: f64,
    paste_ratio: f64,
    largest_paste: usize,
    ai_ratio: f64,
    revision_intensity: f64,
    total_minutes: f64,
    total_content: usize,
    keystroke_timing: &Option<KeystrokeTiming>,
) -> Vec<ProcessFlag> {
    let mut flags = Vec::new();

    // High paste ratio
    if paste_ratio > 0.50 {
        flags.push(ProcessFlag {
            flag_type: ProcessFlagType::HighPasteRatio,
            severity: if paste_ratio > 0.80 {
                ProcessFlagSeverity::High
            } else {
                ProcessFlagSeverity::Medium
            },
            description: format!(
                "Was {:.0}% of this document's content pasted from external sources?",
                paste_ratio * 100.0
            ),
            time_range: None,
        });
    }

    // Large single paste
    if largest_paste > 500 {
        flags.push(ProcessFlag {
            flag_type: ProcessFlagType::BulkPaste,
            severity: if largest_paste > 2000 {
                ProcessFlagSeverity::High
            } else {
                ProcessFlagSeverity::Medium
            },
            description: format!(
                "A single paste event inserted {largest_paste} characters. Was this content composed elsewhere?"
            ),
            time_range: None,
        });
    }

    // Unrealistic typing speed
    if typing_speed > 800.0 && total_minutes > 1.0 {
        flags.push(ProcessFlag {
            flag_type: ProcessFlagType::UnrealisticTypingSpeed,
            severity: ProcessFlagSeverity::High,
            description: format!(
                "Sustained typing speed of {typing_speed:.0} characters per minute exceeds typical human rates (200-400 CPM). Could automated input be involved?"
            ),
            time_range: None,
        });
    }

    // High AI usage
    if ai_ratio > 0.20 {
        flags.push(ProcessFlag {
            flag_type: ProcessFlagType::HighAiUsage,
            severity: if ai_ratio > 0.50 {
                ProcessFlagSeverity::High
            } else {
                ProcessFlagSeverity::Medium
            },
            description: format!(
                "{:.0}% of content originated from AI suggestions. How does this context's policy address AI-assisted writing?",
                ai_ratio * 100.0
            ),
            time_range: None,
        });
    }

    // Insufficient writing time
    if total_content > 1000 && total_minutes < 5.0 {
        flags.push(ProcessFlag {
            flag_type: ProcessFlagType::InsufficientWritingTime,
            severity: ProcessFlagSeverity::Medium,
            description: format!(
                "{total_content} characters of content were produced in only {total_minutes:.1} minutes of recorded writing time. Was the document composed primarily outside the captured editor?"
            ),
            time_range: None,
        });
    }

    // No revision activity
    if total_content > 500 && revision_intensity < 1.0 && total_minutes > 10.0 {
        flags.push(ProcessFlag {
            flag_type: ProcessFlagType::NoRevisionActivity,
            severity: ProcessFlagSeverity::Low,
            description:
                "Very little revision activity (undo/delete) was recorded. Was the text pre-composed and entered without revision?"
                    .into(),
            time_range: None,
        });
    }

    // Uniform keystroke timing
    if let Some(timing) = keystroke_timing {
        if timing.cv_iki < 0.15 && timing.mean_iki_ms < 200.0 {
            flags.push(ProcessFlag {
                flag_type: ProcessFlagType::UniformTiming,
                severity: ProcessFlagSeverity::High,
                description: format!(
                    "Keystroke timing is unusually uniform (CV={:.2}). Human typing typically shows more variation. Could automated text entry be involved?",
                    timing.cv_iki
                ),
                time_range: None,
            });
        }
    }

    flags
}

fn empty_metrics() -> ProcessMetrics {
    ProcessMetrics {
        total_writing_minutes: 0.0,
        session_count: 0,
        avg_session_minutes: 0.0,
        typing_speed_cpm: 0.0,
        paste_ratio: 0.0,
        paste_event_count: 0,
        largest_paste_chars: 0,
        undo_rate: 0.0,
        focus_loss_per_hour: 0.0,
        ai_suggestions_accepted: 0,
        ai_content_ratio: 0.0,
        revision_intensity: 0.0,
        keystroke_timing: None,
        capture_confidence: 0.0,
        flags: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::events::*;
    use crate::capture::session::CaptureSession;

    fn organic_session() -> CaptureSession {
        let mut session = CaptureSession::new(
            "organic-001",
            "essay.txt",
            CaptureSource::VSCode,
            "2026-03-22T10:00:00Z",
        );

        // 30 minutes of organic typing at ~300 CPM
        for i in 0..900 {
            session.push_event(CaptureEvent {
                seq: i,
                timestamp: "2026-03-22T10:00:00Z".into(),
                elapsed_ms: i * 2000, // ~2s per 10 chars
                event_type: EventType::KeystrokeBatch,
                text_length: Some(10),
                position: None,
                source: CaptureSource::VSCode,
            });
        }

        // Some undos
        for i in 900..920 {
            session.push_event(CaptureEvent {
                seq: i,
                timestamp: "2026-03-22T10:30:00Z".into(),
                elapsed_ms: 1_800_000 + (i - 900) * 1000,
                event_type: EventType::Undo,
                text_length: None,
                position: None,
                source: CaptureSource::VSCode,
            });
        }

        session
    }

    fn paste_heavy_session() -> CaptureSession {
        let mut session = CaptureSession::new(
            "paste-001",
            "submission.docx",
            CaptureSource::MicrosoftWord,
            "2026-03-22T14:00:00Z",
        );

        // Brief typing (100 chars)
        for i in 0..10 {
            session.push_event(CaptureEvent {
                seq: i,
                timestamp: "2026-03-22T14:00:00Z".into(),
                elapsed_ms: i * 5000,
                event_type: EventType::KeystrokeBatch,
                text_length: Some(10),
                position: None,
                source: CaptureSource::MicrosoftWord,
            });
        }

        // Large paste (3000 chars)
        session.push_event(CaptureEvent {
            seq: 10,
            timestamp: "2026-03-22T14:01:00Z".into(),
            elapsed_ms: 60_000,
            event_type: EventType::Paste,
            text_length: Some(3000),
            position: None,
            source: CaptureSource::MicrosoftWord,
        });

        // Save
        session.push_event(CaptureEvent {
            seq: 11,
            timestamp: "2026-03-22T14:01:30Z".into(),
            elapsed_ms: 90_000,
            event_type: EventType::Save,
            text_length: None,
            position: None,
            source: CaptureSource::MicrosoftWord,
        });

        session
    }

    #[test]
    fn test_organic_session_metrics() {
        let session = organic_session();
        let metrics = compute_session_metrics(&session);

        assert!(metrics.typing_speed_cpm > 100.0);
        assert!(metrics.paste_ratio < 0.01);
        assert!(metrics.undo_rate > 0.0);
        assert!(metrics.flags.is_empty() || metrics.flags.iter().all(|f| f.severity == ProcessFlagSeverity::Low));
    }

    #[test]
    fn test_paste_heavy_session_flags() {
        let session = paste_heavy_session();
        let metrics = compute_session_metrics(&session);

        assert!(metrics.paste_ratio > 0.9);
        assert!(metrics.largest_paste_chars == 3000);

        let has_paste_flag = metrics.flags.iter().any(|f| f.flag_type == ProcessFlagType::HighPasteRatio);
        assert!(has_paste_flag, "Should flag high paste ratio");

        let has_bulk_flag = metrics.flags.iter().any(|f| f.flag_type == ProcessFlagType::BulkPaste);
        assert!(has_bulk_flag, "Should flag bulk paste");
    }

    #[test]
    fn test_empty_metrics() {
        let metrics = empty_metrics();
        assert_eq!(metrics.session_count, 0);
        assert_eq!(metrics.capture_confidence, 0.0);
        assert!(metrics.flags.is_empty());
    }
}
