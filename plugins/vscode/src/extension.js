/**
 * Provenance Process Capture — VS Code Extension
 *
 * Captures writing process events (keystrokes, paste, undo, save, cursor movement)
 * from VS Code text editors and exports Provenance-compatible capture sessions.
 *
 * Advantages over Google Docs/Word plugins:
 * - Direct access to VS Code event APIs (onDidChangeTextDocument, onDidSave, etc.)
 * - Character-level change detection (not polling-based)
 * - Keystroke timing from real document change events
 */

const vscode = require("vscode");
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

const SCHEMA_VERSION = 1;
const PLUGIN_VERSION = "1.0.0";
const CAPTURE_SOURCE = "VSCode";
const MAX_EVENTS = 100000;

let session = null;
let statusBarItem = null;
let disposables = [];

// ─── Extension Lifecycle ─────────────────────────────────────────────

function activate(context) {
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100
  );
  statusBarItem.command = "provenance.showStats";
  context.subscriptions.push(statusBarItem);

  context.subscriptions.push(
    vscode.commands.registerCommand("provenance.startCapture", startCapture),
    vscode.commands.registerCommand("provenance.stopCapture", stopCapture),
    vscode.commands.registerCommand("provenance.exportSession", exportSession),
    vscode.commands.registerCommand("provenance.showStats", showStats)
  );

  // Auto-start if configured
  const config = vscode.workspace.getConfiguration("provenance");
  if (config.get("autoStart")) {
    const editor = vscode.window.activeTextEditor;
    if (editor && shouldCapture(editor.document)) {
      startCapture();
    }
  }
}

function deactivate() {
  if (session) {
    pushEvent("SessionEnd", null, null);
  }
  disposables.forEach((d) => d.dispose());
  disposables = [];
}

module.exports = { activate, deactivate };

// ─── Capture Control ─────────────────────────────────────────────────

function startCapture() {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage("No active editor to capture.");
    return;
  }

  session = {
    id: crypto.randomUUID(),
    documentUri: editor.document.uri.toString(),
    documentName: path.basename(editor.document.fileName),
    events: [],
    seq: 0,
    startTime: Date.now(),
    lastContent: editor.document.getText(),
    lastCursorOffset: editor.selection.active.character,
  };

  pushEvent("SessionStart", null, null);

  // Register event listeners
  disposables.push(
    vscode.workspace.onDidChangeTextDocument(onDocumentChange),
    vscode.workspace.onDidSaveTextDocument(onDocumentSave),
    vscode.window.onDidChangeTextEditorSelection(onSelectionChange),
    vscode.window.onDidChangeActiveTextEditor(onEditorChange),
    vscode.window.onDidChangeWindowState(onWindowStateChange)
  );

  updateStatusBar();
  vscode.window.showInformationMessage(
    `Provenance capture started for ${session.documentName}`
  );
}

function stopCapture() {
  if (!session) {
    vscode.window.showWarningMessage("No active capture session.");
    return;
  }

  pushEvent("SessionEnd", null, null);

  disposables.forEach((d) => d.dispose());
  disposables = [];

  statusBarItem.text = `$(record) Provenance: Stopped (${session.events.length} events)`;
  vscode.window.showInformationMessage(
    `Capture stopped. ${session.events.length} events recorded. Use 'Export' to save.`
  );
}

// ─── Event Handlers ──────────────────────────────────────────────────

function onDocumentChange(event) {
  if (!session) return;
  if (event.document.uri.toString() !== session.documentUri) return;

  for (const change of event.contentChanges) {
    const insertedLen = change.text.length;
    const deletedLen = change.rangeLength;

    if (deletedLen > 0 && insertedLen === 0) {
      // Pure deletion
      pushEvent("Delete", deletedLen, changeToPosition(change));
    } else if (insertedLen > 0 && deletedLen === 0) {
      // Pure insertion
      if (insertedLen > 200) {
        pushEvent("Paste", insertedLen, changeToPosition(change));
      } else if (insertedLen > 1) {
        pushEvent("KeystrokeBatch", insertedLen, changeToPosition(change));
      } else {
        pushEvent("Keystroke", 1, changeToPosition(change));
      }
    } else if (insertedLen > 0 && deletedLen > 0) {
      // Replace (could be find-replace, autocomplete, or paste-over-selection)
      if (insertedLen > 200) {
        pushEvent("Paste", insertedLen, changeToPosition(change));
      } else {
        pushEvent("FindReplace", insertedLen, changeToPosition(change));
      }
    }
  }

  updateStatusBar();
}

function onDocumentSave(document) {
  if (!session) return;
  if (document.uri.toString() !== session.documentUri) return;
  pushEvent("Save", null, null);
}

function onSelectionChange(event) {
  if (!session) return;
  if (event.textEditor.document.uri.toString() !== session.documentUri) return;

  const sel = event.selections[0];
  if (sel && !sel.isEmpty) {
    pushEvent("Selection", sel.end.character - sel.start.character, {
      paragraph: sel.start.line,
      offset: sel.start.character,
      selection_length:
        event.textEditor.document.offsetAt(sel.end) -
        event.textEditor.document.offsetAt(sel.start),
    });
  }
}

function onEditorChange(editor) {
  if (!session) return;

  if (!editor || editor.document.uri.toString() !== session.documentUri) {
    pushEvent("FocusLoss", null, null);
  } else {
    pushEvent("FocusGain", null, null);
  }
}

function onWindowStateChange(state) {
  if (!session) return;
  if (state.focused) {
    pushEvent("FocusGain", null, null);
  } else {
    pushEvent("FocusLoss", null, null);
  }
}

// ─── Event Management ────────────────────────────────────────────────

function pushEvent(eventType, textLength, position) {
  if (!session || session.events.length >= MAX_EVENTS) return;

  const event = {
    seq: session.seq++,
    timestamp: new Date().toISOString(),
    elapsed_ms: Date.now() - session.startTime,
    event_type: eventType,
    source: CAPTURE_SOURCE,
  };

  if (textLength !== null && textLength !== undefined) {
    event.text_length = Math.abs(textLength);
  }

  if (position) {
    event.position = position;
  }

  session.events.push(event);
}

function changeToPosition(change) {
  return {
    paragraph: change.range.start.line,
    offset: change.range.start.character,
    selection_length: 0,
  };
}

// ─── Export ──────────────────────────────────────────────────────────

async function exportSession() {
  if (!session || session.events.length === 0) {
    vscode.window.showWarningMessage("No capture data to export.");
    return;
  }

  const captureSession = {
    session_id: session.id,
    document_name: session.documentName,
    source: CAPTURE_SOURCE,
    started_at: new Date(session.startTime).toISOString(),
    ended_at: new Date().toISOString(),
    events: session.events,
    plugin_version: PLUGIN_VERSION,
    schema_version: SCHEMA_VERSION,
  };

  const json = JSON.stringify(captureSession, null, 2);

  // Determine output path
  const config = vscode.workspace.getConfiguration("provenance");
  const outputDir =
    config.get("outputDirectory") ||
    (vscode.workspace.workspaceFolders
      ? vscode.workspace.workspaceFolders[0].uri.fsPath
      : require("os").tmpdir());

  const fileName = `${session.documentName}_provenance_capture_${session.id.substring(0, 8)}.json`;
  const outputPath = path.join(outputDir, fileName);

  fs.writeFileSync(outputPath, json, "utf8");

  const action = await vscode.window.showInformationMessage(
    `Exported ${session.events.length} events to ${fileName}`,
    "Open File",
    "Copy Path"
  );

  if (action === "Open File") {
    const doc = await vscode.workspace.openTextDocument(outputPath);
    vscode.window.showTextDocument(doc);
  } else if (action === "Copy Path") {
    vscode.env.clipboard.writeText(outputPath);
  }
}

// ─── Statistics ──────────────────────────────────────────────────────

function showStats() {
  if (!session) {
    vscode.window.showWarningMessage("No active capture session.");
    return;
  }

  const counts = {};
  let totalTyped = 0;
  let totalPasted = 0;

  for (const event of session.events) {
    counts[event.event_type] = (counts[event.event_type] || 0) + 1;
    if (
      (event.event_type === "Keystroke" ||
        event.event_type === "KeystrokeBatch") &&
      event.text_length
    ) {
      totalTyped += event.text_length;
    }
    if (event.event_type === "Paste" && event.text_length) {
      totalPasted += event.text_length;
    }
  }

  const total = totalTyped + totalPasted;
  const pasteRatio = total > 0 ? ((totalPasted / total) * 100).toFixed(1) : "0.0";
  const elapsed =
    session.events.length > 0
      ? (
          session.events[session.events.length - 1].elapsed_ms / 60000
        ).toFixed(1)
      : "0";

  let msg = `Session: ${session.id.substring(0, 8)}...\n`;
  msg += `Duration: ${elapsed} min | Events: ${session.events.length}\n`;
  msg += `Typed: ${totalTyped} chars | Pasted: ${totalPasted} chars\n`;
  msg += `Paste ratio: ${pasteRatio}%\n\n`;

  for (const [type, count] of Object.entries(counts).sort(
    (a, b) => b[1] - a[1]
  )) {
    msg += `${type}: ${count}\n`;
  }

  vscode.window.showInformationMessage(msg, { modal: true });
}

// ─── Helpers ─────────────────────────────────────────────────────────

function shouldCapture(document) {
  const config = vscode.workspace.getConfiguration("provenance");
  const languages = config.get("captureLanguages") || [
    "markdown",
    "plaintext",
    "latex",
  ];
  return languages.includes(document.languageId);
}

function updateStatusBar() {
  if (!session) {
    statusBarItem.hide();
    return;
  }

  statusBarItem.text = `$(record) Provenance: ${session.events.length} events`;
  statusBarItem.tooltip = `Capturing ${session.documentName}`;
  statusBarItem.show();
}
