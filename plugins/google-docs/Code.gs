/**
 * Provenance Process Capture — Google Docs Add-on
 *
 * Captures writing process events (edits, cursor movement, paste detection)
 * from Google Docs and exports them as Provenance-compatible capture sessions.
 *
 * Install: Extensions → Apps Script → paste this code → deploy as Add-on
 */

// ─── Configuration ───────────────────────────────────────────────────

const SCHEMA_VERSION = 1;
const PLUGIN_VERSION = "1.0.0";
const CAPTURE_SOURCE = "GoogleDocs";
const POLL_INTERVAL_MS = 5000; // Check for changes every 5 seconds
const MAX_EVENTS_PER_SESSION = 50000;

// ─── Session State ───────────────────────────────────────────────────

let sessionState = {
  sessionId: null,
  events: [],
  seq: 0,
  startTime: null,
  lastSnapshot: null,
  lastCursorPosition: null,
};

// ─── Add-on Lifecycle ────────────────────────────────────────────────

/**
 * Creates the add-on menu when the document is opened.
 */
function onOpen(e) {
  DocumentApp.getUi()
    .createAddonMenu()
    .addItem("Start Capture", "startCapture")
    .addItem("Stop Capture", "stopCapture")
    .addItem("Export Session", "exportSession")
    .addItem("View Session Stats", "showStats")
    .addToUi();
}

/**
 * Runs when the add-on is installed.
 */
function onInstall(e) {
  onOpen(e);
}

// ─── Capture Control ─────────────────────────────────────────────────

/**
 * Start a new capture session.
 */
function startCapture() {
  const doc = DocumentApp.getActiveDocument();
  const now = new Date();

  sessionState.sessionId = Utilities.getUuid();
  sessionState.events = [];
  sessionState.seq = 0;
  sessionState.startTime = now;
  sessionState.lastSnapshot = getDocumentSnapshot(doc);
  sessionState.lastCursorPosition = getCursorInfo();

  pushEvent("SessionStart", null);

  // Store session state in document properties for persistence
  saveState();

  // Set up time-driven trigger for polling
  ScriptApp.newTrigger("pollForChanges")
    .timeBased()
    .everyMinutes(1)
    .create();

  DocumentApp.getUi().alert(
    "Provenance Capture Started",
    "Writing process events are now being recorded.\n\n" +
    "Session ID: " + sessionState.sessionId.substring(0, 8) + "...\n\n" +
    "Use 'Stop Capture' when finished, then 'Export Session' to save.",
    DocumentApp.getUi().ButtonSet.OK
  );
}

/**
 * Stop the current capture session.
 */
function stopCapture() {
  if (!sessionState.sessionId) {
    DocumentApp.getUi().alert("No active capture session.");
    return;
  }

  pushEvent("SessionEnd", null);

  // Remove polling trigger
  const triggers = ScriptApp.getProjectTriggers();
  for (const trigger of triggers) {
    if (trigger.getHandlerFunction() === "pollForChanges") {
      ScriptApp.deleteTrigger(trigger);
    }
  }

  saveState();

  DocumentApp.getUi().alert(
    "Capture Stopped",
    "Session recorded " + sessionState.events.length + " events.\n" +
    "Use 'Export Session' to download the capture data.",
    DocumentApp.getUi().ButtonSet.OK
  );
}

// ─── Change Detection ────────────────────────────────────────────────

/**
 * Poll for document changes (called by time trigger).
 */
function pollForChanges() {
  loadState();
  if (!sessionState.sessionId) return;

  const doc = DocumentApp.getActiveDocument();
  const newSnapshot = getDocumentSnapshot(doc);
  const newCursor = getCursorInfo();

  if (!sessionState.lastSnapshot) {
    sessionState.lastSnapshot = newSnapshot;
    saveState();
    return;
  }

  // Detect text changes
  const oldText = sessionState.lastSnapshot.text;
  const newText = newSnapshot.text;

  if (newText !== oldText) {
    const lengthDelta = newText.length - oldText.length;

    if (lengthDelta > 200) {
      // Large insertion — likely paste
      pushEvent("Paste", lengthDelta);
    } else if (lengthDelta > 0) {
      // Normal typing
      pushEvent("KeystrokeBatch", lengthDelta);
    } else if (lengthDelta < -100) {
      // Large deletion — cut or bulk delete
      pushEvent("Cut", Math.abs(lengthDelta));
    } else if (lengthDelta < 0) {
      // Normal deletion
      pushEvent("Delete", Math.abs(lengthDelta));
    }
  }

  // Detect cursor movement without text change
  if (newText === oldText && newCursor && sessionState.lastCursorPosition) {
    if (newCursor.offset !== sessionState.lastCursorPosition.offset) {
      pushEvent("CursorMove", null);
    }
  }

  // Detect paragraph count change (structural edit)
  if (newSnapshot.paragraphCount !== sessionState.lastSnapshot.paragraphCount) {
    pushEvent("FormatChange", null);
  }

  sessionState.lastSnapshot = newSnapshot;
  sessionState.lastCursorPosition = newCursor;
  saveState();
}

/**
 * Get a snapshot of the current document state.
 */
function getDocumentSnapshot(doc) {
  const body = doc.getBody();
  return {
    text: body.getText(),
    paragraphCount: body.getNumChildren(),
    charCount: body.getText().length,
  };
}

/**
 * Get current cursor position info.
 */
function getCursorInfo() {
  try {
    const cursor = DocumentApp.getActiveDocument().getCursor();
    if (!cursor) return null;

    const element = cursor.getElement();
    const offset = cursor.getOffset();
    const paragraph = element.getParent();
    const body = DocumentApp.getActiveDocument().getBody();

    let paragraphIndex = 0;
    for (let i = 0; i < body.getNumChildren(); i++) {
      if (body.getChild(i).asText && body.getChild(i) === paragraph) {
        paragraphIndex = i;
        break;
      }
    }

    return { paragraph: paragraphIndex, offset: offset };
  } catch (e) {
    return null;
  }
}

// ─── Event Management ────────────────────────────────────────────────

/**
 * Push a capture event to the session.
 */
function pushEvent(eventType, textLength) {
  if (!sessionState.sessionId) return;
  if (sessionState.events.length >= MAX_EVENTS_PER_SESSION) return;

  const now = new Date();
  const elapsedMs = now.getTime() - sessionState.startTime.getTime();
  const cursor = getCursorInfo();

  const event = {
    seq: sessionState.seq++,
    timestamp: now.toISOString(),
    elapsed_ms: elapsedMs,
    event_type: eventType,
    source: CAPTURE_SOURCE,
  };

  if (textLength !== null && textLength !== undefined) {
    event.text_length = Math.abs(textLength);
  }

  if (cursor) {
    event.position = {
      paragraph: cursor.paragraph,
      offset: cursor.offset,
      selection_length: 0,
    };
  }

  sessionState.events.push(event);
}

// ─── Export ──────────────────────────────────────────────────────────

/**
 * Export the capture session as a JSON file in Google Drive.
 */
function exportSession() {
  loadState();
  if (!sessionState.sessionId || sessionState.events.length === 0) {
    DocumentApp.getUi().alert("No capture data to export.");
    return;
  }

  const doc = DocumentApp.getActiveDocument();
  const session = {
    session_id: sessionState.sessionId,
    document_name: doc.getName(),
    source: CAPTURE_SOURCE,
    started_at: sessionState.startTime.toISOString(),
    ended_at: new Date().toISOString(),
    events: sessionState.events,
    plugin_version: PLUGIN_VERSION,
    schema_version: SCHEMA_VERSION,
  };

  const json = JSON.stringify(session, null, 2);
  const fileName = doc.getName() + "_provenance_capture_" +
    sessionState.sessionId.substring(0, 8) + ".json";

  const file = DriveApp.createFile(fileName, json, MimeType.PLAIN_TEXT);

  DocumentApp.getUi().alert(
    "Session Exported",
    "Capture data saved to Google Drive:\n\n" +
    fileName + "\n\n" +
    "Events: " + sessionState.events.length + "\n" +
    "Download this file and import into Provenance:\n" +
    "  provenance import-capture --file " + fileName,
    DocumentApp.getUi().ButtonSet.OK
  );
}

/**
 * Show session statistics.
 */
function showStats() {
  loadState();
  if (!sessionState.sessionId) {
    DocumentApp.getUi().alert("No active capture session.");
    return;
  }

  const counts = {};
  for (const event of sessionState.events) {
    counts[event.event_type] = (counts[event.event_type] || 0) + 1;
  }

  let totalTyped = 0;
  let totalPasted = 0;
  for (const event of sessionState.events) {
    if (event.event_type === "KeystrokeBatch" && event.text_length) {
      totalTyped += event.text_length;
    }
    if (event.event_type === "Paste" && event.text_length) {
      totalPasted += event.text_length;
    }
  }

  const elapsedMin = sessionState.events.length > 0
    ? (sessionState.events[sessionState.events.length - 1].elapsed_ms / 60000).toFixed(1)
    : 0;

  let msg = "Session: " + sessionState.sessionId.substring(0, 8) + "...\n";
  msg += "Duration: " + elapsedMin + " minutes\n";
  msg += "Total events: " + sessionState.events.length + "\n\n";
  msg += "Characters typed: " + totalTyped + "\n";
  msg += "Characters pasted: " + totalPasted + "\n";
  msg += "Paste ratio: " + ((totalPasted / (totalTyped + totalPasted || 1)) * 100).toFixed(1) + "%\n\n";
  msg += "Event breakdown:\n";
  for (const [type, count] of Object.entries(counts)) {
    msg += "  " + type + ": " + count + "\n";
  }

  DocumentApp.getUi().alert("Capture Statistics", msg, DocumentApp.getUi().ButtonSet.OK);
}

// ─── State Persistence ───────────────────────────────────────────────

function saveState() {
  const props = PropertiesService.getDocumentProperties();
  props.setProperty("provenance_session", JSON.stringify({
    sessionId: sessionState.sessionId,
    events: sessionState.events,
    seq: sessionState.seq,
    startTime: sessionState.startTime ? sessionState.startTime.toISOString() : null,
    lastSnapshot: sessionState.lastSnapshot,
    lastCursorPosition: sessionState.lastCursorPosition,
  }));
}

function loadState() {
  const props = PropertiesService.getDocumentProperties();
  const stored = props.getProperty("provenance_session");
  if (stored) {
    const data = JSON.parse(stored);
    sessionState.sessionId = data.sessionId;
    sessionState.events = data.events || [];
    sessionState.seq = data.seq || 0;
    sessionState.startTime = data.startTime ? new Date(data.startTime) : null;
    sessionState.lastSnapshot = data.lastSnapshot;
    sessionState.lastCursorPosition = data.lastCursorPosition;
  }
}
