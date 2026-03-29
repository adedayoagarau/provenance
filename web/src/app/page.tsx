"use client";

import { useState } from "react";
import DropZone from "@/components/DropZone";
import ProgressIndicator from "@/components/ProgressIndicator";
import ResultsView from "@/components/ResultsView";
import ReliabilityDisclosure from "@/components/ReliabilityDisclosure";
import { AnalysisResult } from "@/lib/types";

type Tab = "detect" | "analyze";

type DetectionResult = {
  score: number;
  adjusted_score: number;
  tier: string;
  tier_label: string;
  action: string;
  confidence: number;
  features_used: number;
  feature_scores: Array<{
    name: string;
    raw_value: number;
    normalized_score: number;
    weight: number;
    weighted_contribution: number;
    explanation: string;
  }>;
};

type ViewState =
  | { kind: "idle" }
  | { kind: "analyzing"; source: string }
  | { kind: "error"; message: string; details?: string }
  | { kind: "detect-results"; detection: DetectionResult; wordCount: number }
  | { kind: "analyze-results"; fileName: string; fileSize: number; result: AnalysisResult };

const TIER_COLORS: Record<string, string> = {
  "HUMAN AUTHORED": "text-green-400 border-green-400/30 bg-green-400/5",
  "LIKELY HUMAN": "text-green-300 border-green-300/30 bg-green-300/5",
  "MIXED / UNCERTAIN": "text-amber-400 border-amber-400/30 bg-amber-400/5",
  "AI ASSISTED": "text-orange-400 border-orange-400/30 bg-orange-400/5",
  "AI GENERATED": "text-red-400 border-red-400/30 bg-red-400/5",
};

export default function AnalyzePage() {
  const [tab, setTab] = useState<Tab>("detect");
  const [view, setView] = useState<ViewState>({ kind: "idle" });
  const [text, setText] = useState("");

  const characterCount = text.length;
  const wordCount = text.trim() ? text.trim().split(/\s+/).length : 0;

  const handleDetect = async () => {
    if (wordCount < 50) return;
    setView({ kind: "analyzing", source: "text" });

    try {
      const res = await fetch("/api/detect", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text }),
      });

      const data = await res.json();

      if (!res.ok) {
        setView({ kind: "error", message: data.error, details: data.details });
        return;
      }

      // Extract detection score from the nested response
      const detection = data.result?.detection?.score || data.result?.score;
      if (!detection) {
        setView({ kind: "error", message: "No detection results returned" });
        return;
      }

      setView({ kind: "detect-results", detection, wordCount: data.word_count });
    } catch (err) {
      setView({
        kind: "error",
        message: "Network error",
        details: err instanceof Error ? err.message : String(err),
      });
    }
  };

  const handleFiles = async (files: File[]) => {
    const file = files[0];
    if (!file) return;
    setView({ kind: "analyzing", source: file.name });

    try {
      const formData = new FormData();
      formData.append("file", file);

      const res = await fetch("/api/analyze", { method: "POST", body: formData });
      const data = await res.json();

      if (!res.ok) {
        setView({ kind: "error", message: data.error, details: data.details });
        return;
      }

      setView({
        kind: "analyze-results",
        fileName: data.file_name,
        fileSize: data.file_size,
        result: data.result,
      });
    } catch (err) {
      setView({
        kind: "error",
        message: "Network error",
        details: err instanceof Error ? err.message : String(err),
      });
    }
  };

  // Full analysis results view
  if (view.kind === "analyze-results") {
    return (
      <ResultsView
        fileName={view.fileName}
        fileSize={view.fileSize}
        result={view.result}
        onBack={() => setView({ kind: "idle" })}
      />
    );
  }

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      {/* Tab Switcher */}
      <div className="flex justify-center">
        <div className="inline-flex border border-border-primary rounded-full p-0.5">
          <button
            onClick={() => { setTab("detect"); setView({ kind: "idle" }); }}
            className={`px-5 py-1.5 text-xs font-mono rounded-full transition-colors ${
              tab === "detect"
                ? "bg-accent text-white"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            AI Detector
          </button>
          <button
            onClick={() => { setTab("analyze"); setView({ kind: "idle" }); }}
            className={`px-5 py-1.5 text-xs font-mono rounded-full transition-colors ${
              tab === "analyze"
                ? "bg-accent text-white"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            Full Analysis
          </button>
        </div>
      </div>

      {/* AI Detection Tab */}
      {tab === "detect" && view.kind === "idle" && (
        <div className="space-y-4">
          <div className="text-center mb-4">
            <h1 className="font-mono text-sm font-semibold text-text-primary uppercase tracking-wider mb-1">
              AI Content Detector
            </h1>
            <p className="text-xs text-text-tertiary">
              Paste text to check if it was written by a human or AI
            </p>
          </div>

          <div className="border border-border-primary rounded-lg overflow-hidden">
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder="Paste your text here..."
              className="w-full h-64 p-4 bg-bg-secondary text-text-primary text-sm font-sans resize-none focus:outline-none placeholder:text-text-tertiary"
            />
            <div className="flex items-center justify-between px-4 py-2 bg-bg-tertiary border-t border-border-primary">
              <span className="text-xs font-mono text-text-tertiary">
                {wordCount} words / {characterCount.toLocaleString()} characters
              </span>
              <button
                onClick={handleDetect}
                disabled={wordCount < 50}
                className={`px-6 py-2 text-sm font-mono font-semibold rounded transition-colors ${
                  wordCount >= 50
                    ? "bg-accent text-white hover:bg-accent/80"
                    : "bg-bg-elevated text-text-tertiary cursor-not-allowed"
                }`}
              >
                Scan
              </button>
            </div>
          </div>

          {wordCount > 0 && wordCount < 50 && (
            <p className="text-xs text-status-amber text-center">
              Need at least 50 words for detection ({50 - wordCount} more needed)
            </p>
          )}
        </div>
      )}

      {/* Full Analysis Tab */}
      {tab === "analyze" && view.kind === "idle" && (
        <div className="space-y-4">
          <div className="text-center mb-4">
            <h1 className="font-mono text-sm font-semibold text-text-primary uppercase tracking-wider mb-1">
              Document Analysis
            </h1>
            <p className="text-xs text-text-tertiary">
              Upload a document for forensic authorship analysis
            </p>
          </div>
          <DropZone onFiles={handleFiles} />
        </div>
      )}

      {/* Analyzing State */}
      {view.kind === "analyzing" && (
        <ProgressIndicator fileName={view.source} />
      )}

      {/* Detection Results */}
      {view.kind === "detect-results" && (
        <DetectionResultsView
          detection={view.detection}
          wordCount={view.wordCount}
          onBack={() => setView({ kind: "idle" })}
        />
      )}

      {/* Error State */}
      {view.kind === "error" && (
        <div className="border border-status-red p-5 rounded-lg">
          <div className="text-xs font-mono text-status-red uppercase tracking-wide mb-2">
            Error
          </div>
          <p className="text-sm text-text-primary mb-2">{view.message}</p>
          {view.details && (
            <pre className="text-xs font-mono text-text-tertiary whitespace-pre-wrap break-all max-h-40 overflow-y-auto">
              {view.details}
            </pre>
          )}
          <button
            onClick={() => setView({ kind: "idle" })}
            className="mt-4 text-xs font-mono text-accent hover:underline"
          >
            &larr; Try again
          </button>
        </div>
      )}

      {/* Reliability notice on idle */}
      {view.kind === "idle" && tab === "analyze" && (
        <ReliabilityDisclosure disclosure={null} />
      )}
    </div>
  );
}

/* ─── Detection Results Component ────────────────────────────────────────── */

function DetectionResultsView({
  detection,
  wordCount,
  onBack,
}: {
  detection: DetectionResult;
  wordCount: number;
  onBack: () => void;
}) {
  const tierColor = TIER_COLORS[detection.tier_label] || "text-text-primary border-border-primary";
  const scorePercent = Math.round(detection.adjusted_score * 100);
  const confidencePercent = Math.round(detection.confidence * 100);

  return (
    <div className="space-y-6">
      <button
        onClick={onBack}
        className="text-xs font-mono text-accent hover:underline"
      >
        &larr; New scan
      </button>

      {/* Main Result Card */}
      <div className={`border-2 rounded-lg p-8 text-center ${tierColor}`}>
        <div className="text-3xl font-mono font-bold mb-2">
          {detection.tier_label}
        </div>
        <div className="text-sm opacity-80">
          {detection.action}
        </div>
      </div>

      {/* Score Details */}
      <div className="grid grid-cols-3 gap-4">
        <div className="border border-border-primary rounded-lg p-4 text-center">
          <div className="text-2xl font-mono font-bold text-text-primary">{scorePercent}%</div>
          <div className="text-xs font-mono text-text-tertiary mt-1">AI Likelihood</div>
        </div>
        <div className="border border-border-primary rounded-lg p-4 text-center">
          <div className="text-2xl font-mono font-bold text-text-primary">{confidencePercent}%</div>
          <div className="text-xs font-mono text-text-tertiary mt-1">Confidence</div>
        </div>
        <div className="border border-border-primary rounded-lg p-4 text-center">
          <div className="text-2xl font-mono font-bold text-text-primary">{wordCount}</div>
          <div className="text-xs font-mono text-text-tertiary mt-1">Words</div>
        </div>
      </div>

      {/* Score Bar */}
      <div className="border border-border-primary rounded-lg p-4">
        <div className="flex justify-between text-xs font-mono text-text-tertiary mb-2">
          <span>Human</span>
          <span>AI Generated</span>
        </div>
        <div className="w-full h-3 bg-bg-tertiary rounded-full overflow-hidden">
          <div
            className="h-full rounded-full transition-all duration-500"
            style={{
              width: `${scorePercent}%`,
              backgroundColor: scorePercent < 35 ? "#4ade80" : scorePercent < 65 ? "#fbbf24" : "#f87171",
            }}
          />
        </div>
        <div className="flex justify-between text-xs font-mono text-text-tertiary mt-2">
          <span>0%</span>
          <span>50%</span>
          <span>100%</span>
        </div>
      </div>

      {/* Feature Breakdown */}
      {detection.feature_scores.length > 0 && (
        <div className="border border-border-primary rounded-lg p-4">
          <div className="text-xs font-mono text-text-secondary uppercase tracking-wide mb-3">
            Signal Breakdown ({detection.features_used} features analyzed)
          </div>
          <div className="space-y-2">
            {detection.feature_scores
              .sort((a, b) => Math.abs(b.weighted_contribution) - Math.abs(a.weighted_contribution))
              .slice(0, 7)
              .map((f, i) => (
                <div key={i} className="flex items-center gap-3">
                  <div className="w-40 text-xs font-mono text-text-secondary truncate" title={f.name}>
                    {f.name}
                  </div>
                  <div className="flex-1 h-2 bg-bg-tertiary rounded-full overflow-hidden">
                    <div
                      className="h-full rounded-full"
                      style={{
                        width: `${Math.min(f.normalized_score * 100, 100)}%`,
                        backgroundColor: f.normalized_score < 0.4 ? "#4ade80" : f.normalized_score < 0.6 ? "#fbbf24" : "#f87171",
                      }}
                    />
                  </div>
                  <div className="w-12 text-xs font-mono text-text-tertiary text-right">
                    {(f.normalized_score * 100).toFixed(0)}%
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}

      {/* Disclaimer */}
      <div className="text-xs text-text-tertiary text-center border-t border-border-primary pt-4">
        These results are statistical indicators, not definitive proof. Human review is always recommended.
      </div>
    </div>
  );
}
