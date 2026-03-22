/**
 * Provenance Process Capture — Microsoft Word Add-in (Office.js)
 *
 * Captures writing process events from Word documents and exports them
 * as Provenance-compatible capture sessions.
 */

/* global Office, Word */

const SCHEMA_VERSION = 1;
const PLUGIN_VERSION = "1.0.0";
const CAPTURE_SOURCE = "MicrosoftWord";
const POLL_INTERVAL_MS = 3000;
const MAX_EVENTS = 50000;

let session = {
  id: null,
  events: [],
  seq: 0,
  startTime: null,
  lastSnapshot: null,
  pollTimer: null,
};

// ─── Office.js Initialization ────────────────────────────────────────

Office.onReady((info) => {
  if (info.host === Office.HostType.Word) {
    document.getElementById("start-btn").onclick = startCapture;
    document.getElementById("stop-btn").onclick = stopCapture;
    document.getElementById("export-btn").onclick = exportSession;
    document.getElementById("stats-btn").onclick = showStats;
    updateUI("idle");
  }
});

// ─── Capture Control ─────────────────────────────────────────────────

async function startCapture() {
  session.id = crypto.randomUUID();
  session.events = [];
  session.seq = 0;
  session.startTime = new Date();

  session.lastSnapshot = await getDocumentSnapshot();
  pushEvent("SessionStart", null);

  // Start polling for changes
  session.pollTimer = setInterval(pollForChanges, POLL_INTERVAL_MS);

  // Register event handlers
  await Word.run(async (context) => {
    context.document.onContentChanged.add(onContentChanged);
    await context.sync();
  });

  updateUI("capturing");
  updateStatus(`Capturing... Session ${session.id.substring(0, 8)}`);
}

async function stopCapture() {
  if (!session.id) return;

  pushEvent("SessionEnd", null);

  if (session.pollTimer) {
    clearInterval(session.pollTimer);
    session.pollTimer = null;
  }

  updateUI("stopped");
  updateStatus(`Stopped. ${session.events.length} events captured.`);
}

// ─── Change Detection ────────────────────────────────────────────────

async function pollForChanges() {
  if (!session.id) return;

  try {
    const newSnapshot = await getDocumentSnapshot();
    if (!session.lastSnapshot) {
      session.lastSnapshot = newSnapshot;
      return;
    }

    const oldLen = session.lastSnapshot.charCount;
    const newLen = newSnapshot.charCount;
    const delta = newLen - oldLen;

    if (delta > 200) {
      pushEvent("Paste", delta);
    } else if (delta > 0) {
      pushEvent("KeystrokeBatch", delta);
    } else if (delta < -100) {
      pushEvent("Cut", Math.abs(delta));
    } else if (delta < 0) {
      pushEvent("Delete", Math.abs(delta));
    }

    // Detect paragraph structure changes
    if (newSnapshot.paragraphCount !== session.lastSnapshot.paragraphCount) {
      pushEvent("FormatChange", null);
    }

    session.lastSnapshot = newSnapshot;
    updateEventCount();
  } catch (e) {
    console.error("Poll error:", e);
  }
}

function onContentChanged(args) {
  // Office.js content changed callback — supplements polling
  // with more immediate detection of changes.
  // The actual diff logic is in pollForChanges.
}

async function getDocumentSnapshot() {
  return await Word.run(async (context) => {
    const body = context.document.body;
    body.load("text");
    const paragraphs = body.paragraphs;
    paragraphs.load("items");
    await context.sync();

    return {
      charCount: body.text.length,
      paragraphCount: paragraphs.items.length,
      wordCount: body.text.split(/\s+/).filter(w => w.length > 0).length,
    };
  });
}

// ─── Event Management ────────────────────────────────────────────────

function pushEvent(eventType, textLength) {
  if (!session.id || session.events.length >= MAX_EVENTS) return;

  const now = new Date();
  const elapsedMs = now.getTime() - session.startTime.getTime();

  const event = {
    seq: session.seq++,
    timestamp: now.toISOString(),
    elapsed_ms: elapsedMs,
    event_type: eventType,
    source: CAPTURE_SOURCE,
  };

  if (textLength !== null && textLength !== undefined) {
    event.text_length = Math.abs(textLength);
  }

  session.events.push(event);
}

// ─── Export ──────────────────────────────────────────────────────────

async function exportSession() {
  if (!session.id || session.events.length === 0) {
    updateStatus("No capture data to export.");
    return;
  }

  const docName = await Word.run(async (context) => {
    const properties = context.document.properties;
    properties.load("title");
    await context.sync();
    return properties.title || "untitled";
  });

  const captureSession = {
    session_id: session.id,
    document_name: docName,
    source: CAPTURE_SOURCE,
    started_at: session.startTime.toISOString(),
    ended_at: new Date().toISOString(),
    events: session.events,
    plugin_version: PLUGIN_VERSION,
    schema_version: SCHEMA_VERSION,
  };

  const json = JSON.stringify(captureSession, null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);

  const a = document.createElement("a");
  a.href = url;
  a.download = `${docName}_provenance_capture_${session.id.substring(0, 8)}.json`;
  a.click();
  URL.revokeObjectURL(url);

  updateStatus(`Exported ${session.events.length} events.`);
}

function showStats() {
  if (!session.id) {
    updateStatus("No active session.");
    return;
  }

  const counts = {};
  let totalTyped = 0;
  let totalPasted = 0;

  for (const event of session.events) {
    counts[event.event_type] = (counts[event.event_type] || 0) + 1;
    if (event.event_type === "KeystrokeBatch" && event.text_length) totalTyped += event.text_length;
    if (event.event_type === "Paste" && event.text_length) totalPasted += event.text_length;
  }

  const total = totalTyped + totalPasted;
  const pasteRatio = total > 0 ? ((totalPasted / total) * 100).toFixed(1) : "0.0";
  const elapsed = session.events.length > 0
    ? (session.events[session.events.length - 1].elapsed_ms / 60000).toFixed(1)
    : "0";

  let html = `<strong>Session:</strong> ${session.id.substring(0, 8)}...<br>`;
  html += `<strong>Duration:</strong> ${elapsed} min<br>`;
  html += `<strong>Events:</strong> ${session.events.length}<br>`;
  html += `<strong>Typed:</strong> ${totalTyped} chars<br>`;
  html += `<strong>Pasted:</strong> ${totalPasted} chars<br>`;
  html += `<strong>Paste ratio:</strong> ${pasteRatio}%<br><br>`;

  for (const [type, count] of Object.entries(counts)) {
    html += `${type}: ${count}<br>`;
  }

  document.getElementById("stats-panel").innerHTML = html;
}

// ─── UI Helpers ──────────────────────────────────────────────────────

function updateUI(state) {
  const startBtn = document.getElementById("start-btn");
  const stopBtn = document.getElementById("stop-btn");
  const exportBtn = document.getElementById("export-btn");

  startBtn.disabled = state === "capturing";
  stopBtn.disabled = state !== "capturing";
  exportBtn.disabled = state === "idle";
}

function updateStatus(msg) {
  document.getElementById("status").textContent = msg;
}

function updateEventCount() {
  document.getElementById("event-count").textContent = session.events.length;
}
