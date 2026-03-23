"use client";

import { useState } from "react";
import DropZone from "@/components/DropZone";
import ProgressIndicator from "@/components/ProgressIndicator";
import ResultsView from "@/components/ResultsView";
import ReliabilityDisclosure from "@/components/ReliabilityDisclosure";
import { AnalysisResult } from "@/lib/types";

type ViewState =
  | { kind: "upload" }
  | { kind: "analyzing"; fileName: string }
  | { kind: "error"; message: string; details?: string }
  | { kind: "results"; fileName: string; fileSize: number; result: AnalysisResult };

export default function AnalyzePage() {
  const [view, setView] = useState<ViewState>({ kind: "upload" });

  const handleFiles = async (files: File[]) => {
    const file = files[0];
    if (!file) return;

    setView({ kind: "analyzing", fileName: file.name });

    try {
      const formData = new FormData();
      formData.append("file", file);

      const res = await fetch("/api/analyze", {
        method: "POST",
        body: formData,
      });

      const data = await res.json();

      if (!res.ok) {
        setView({
          kind: "error",
          message: data.error || "Analysis failed",
          details: data.details,
        });
        return;
      }

      setView({
        kind: "results",
        fileName: data.file_name,
        fileSize: data.file_size,
        result: data.result,
      });
    } catch (err) {
      setView({
        kind: "error",
        message: "Network error — could not reach analysis server",
        details: err instanceof Error ? err.message : String(err),
      });
    }
  };

  if (view.kind === "results") {
    return (
      <ResultsView
        fileName={view.fileName}
        fileSize={view.fileSize}
        result={view.result}
        onBack={() => setView({ kind: "upload" })}
      />
    );
  }

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="text-center mb-8">
        <h1 className="font-mono text-sm font-semibold text-text-primary uppercase tracking-wider mb-1">
          Document Analysis
        </h1>
        <p className="text-xs text-text-tertiary">
          Upload a document for forensic authorship analysis
        </p>
      </div>

      {view.kind === "upload" && (
        <DropZone onFiles={handleFiles} />
      )}

      {view.kind === "analyzing" && (
        <ProgressIndicator fileName={view.fileName} />
      )}

      {view.kind === "error" && (
        <div className="border border-status-red p-5">
          <div className="text-xs font-mono text-status-red uppercase tracking-wide mb-2">
            Analysis Error
          </div>
          <p className="text-sm text-text-primary mb-2">{view.message}</p>
          {view.details && (
            <pre className="text-xs font-mono text-text-tertiary whitespace-pre-wrap break-all max-h-40 overflow-y-auto">
              {view.details}
            </pre>
          )}
          <button
            onClick={() => setView({ kind: "upload" })}
            className="mt-4 text-xs font-mono text-accent hover:underline"
          >
            &larr; Try again
          </button>
        </div>
      )}

      {/* Reliability disclosure shown on upload page too */}
      {view.kind === "upload" && (
        <ReliabilityDisclosure disclosure={null} />
      )}
    </div>
  );
}
