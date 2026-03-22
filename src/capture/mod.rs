//! Process Capture — records writing process events from editor plugins.
//!
//! Plugins for Google Docs, Word, VS Code, and Scrivener capture real-time
//! writing events (keystrokes, paste, undo, cursor movement, save). This module
//! defines the shared event model, session storage, and import/export.

pub mod events;
pub mod session;
pub mod metrics;
pub mod scrivener;
