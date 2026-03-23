"use client";

import { useState } from "react";
import DropZone from "@/components/DropZone";
import ProgressIndicator from "@/components/ProgressIndicator";
import ResultsView from "@/components/ResultsView";
import ConstructionBadge from "@/components/ConstructionBadge";
import ReliabilityDisclosure from "@/components/ReliabilityDisclosure";
import { AnalysisResult, BatchResult, ConstructionPattern } from "@/lib/types";

type BatchState =
  | { kind: "upload" }
  | { kind: "analyzing"; fileNames: string[]; progress: number }
  | { kind: "error"; message: string }
  | { kind: "results"; results: BatchResult[] }
  | { kind: "detail"; results: BatchResult[]; selected: BatchResult };

function extractBatchResult(
  fileName: string,
  fileSize: number,
  result: AnalysisResult
): BatchResult {
  const docxProfile = result.forensic_report?.docx_profile;
  return {
    file_name: fileName,
    construction_pattern: docxProfile?.construction_pattern as ConstructionPattern | null || null,
    acs_score: result.acs?.score ?? null,
    acs_margin: result.acs?.margin ?? null,
    pii_score: result.pii?.score ?? null,
    anomaly_count: result.anomaly_report?.flags.length ?? 0,
    register: result.register?.label ?? null,
    full_result: result,
  };
}

function exportCSV(results: BatchResult[]) {
  const headers = [
    "File Name",
    "Construction Pattern",
    "ACS Score",
    "ACS Margin",
    "PII Score",
    "Anomaly Count",
    "Register",
  ];

  const rows = results.map((r) => [
    r.file_name,
    r.construction_pattern || "—",
    r.acs_score?.toFixed(1) ?? "—",
    r.acs_margin?.toFixed(1) ?? "—",
    r.pii_score?.toFixed(1) ?? "—",
    r.anomaly_count.toString(),
    r.register || "—",
  ]);

  const csv = [headers, ...rows]
    .map((row) => row.map((cell) => `"${cell.replace(/"/g, '""')}"`).join(","))
    .join("\n");

  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `provenance-batch-${new Date().toISOString().slice(0, 10)}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

function anomalyCountColor(count: number): string {
  if (count === 0) return "text-text-tertiary";
  if (count <= 2) return "text-status-amber";
  return "text-status-red";
}

export default function BatchPage() {
  const [state, setState] = useState<BatchState>({ kind: "upload" });

  const handleFiles = async (files: File[]) => {
    setState({
      kind: "analyzing",
      fileNames: files.map((f) => f.name),
      progress: 0,
    });

    try {
      const formData = new FormData();
      files.forEach((f) => formData.append("files", f));

      const res = await fetch("/api/batch", {
        method: "POST",
        body: formData,
      });

      const data = await res.json();

      if (!res.ok) {
        setState({ kind: "error", message: data.error || "Batch analysis failed" });
        return;
      }

      const batchResults: BatchResult[] = data.results
        .filter((r: { result?: AnalysisResult; error?: string }) => r.result)
        .map((r: { file_name: string; file_size: number; result: AnalysisResult }) =>
          extractBatchResult(r.file_name, r.file_size, r.result)
        );

      setState({ kind: "results", results: batchResults });
    } catch (err) {
      setState({
        kind: "error",
        message: err instanceof Error ? err.message : "Network error",
      });
    }
  };

  if (state.kind === "detail") {
    return (
      <ResultsView
        fileName={state.selected.file_name}
        fileSize={0}
        result={state.selected.full_result}
        onBack={() => setState({ kind: "results", results: state.results })}
      />
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-mono text-sm font-semibold text-text-primary uppercase tracking-wider mb-1">
            Batch Analysis
          </h1>
          <p className="text-xs text-text-tertiary">
            Upload multiple documents for comparative analysis
          </p>
        </div>
        {state.kind === "results" && (
          <div className="flex gap-3">
            <button
              onClick={() => exportCSV(state.results)}
              className="text-xs font-mono px-3 py-1.5 border border-border text-text-secondary hover:text-text-primary hover:border-border-focus transition-colors"
            >
              Export CSV
            </button>
            <button
              onClick={() => setState({ kind: "upload" })}
              className="text-xs font-mono px-3 py-1.5 border border-border text-text-secondary hover:text-text-primary hover:border-border-focus transition-colors"
            >
              New batch
            </button>
          </div>
        )}
      </div>

      {state.kind === "upload" && (
        <DropZone onFiles={handleFiles} multiple />
      )}

      {state.kind === "analyzing" && (
        <div className="space-y-3">
          <div className="text-xs font-mono text-text-secondary">
            Analyzing {state.fileNames.length} file{state.fileNames.length > 1 ? "s" : ""}...
          </div>
          <ProgressIndicator fileName={state.fileNames.join(", ")} />
        </div>
      )}

      {state.kind === "error" && (
        <div className="border border-status-red p-5">
          <div className="text-xs font-mono text-status-red uppercase tracking-wide mb-2">
            Batch Error
          </div>
          <p className="text-sm text-text-primary">{state.message}</p>
          <button
            onClick={() => setState({ kind: "upload" })}
            className="mt-4 text-xs font-mono text-accent hover:underline"
          >
            &larr; Try again
          </button>
        </div>
      )}

      {state.kind === "results" && (
        <>
          <div className="border border-border overflow-x-auto">
            <table className="w-full text-xs font-mono">
              <thead>
                <tr className="border-b border-border text-text-tertiary uppercase tracking-wide">
                  <th className="text-left py-2 px-3 font-medium">File</th>
                  <th className="text-left py-2 px-3 font-medium">Pattern</th>
                  <th className="text-right py-2 px-3 font-medium">ACS</th>
                  <th className="text-right py-2 px-3 font-medium">PII</th>
                  <th className="text-right py-2 px-3 font-medium">Anomalies</th>
                  <th className="text-left py-2 px-3 font-medium">Register</th>
                </tr>
              </thead>
              <tbody>
                {state.results.map((r, i) => (
                  <tr
                    key={i}
                    className="border-b border-border hover:bg-bg-tertiary cursor-pointer transition-colors"
                    onClick={() =>
                      setState({ kind: "detail", results: state.results, selected: r })
                    }
                  >
                    <td className="py-2 px-3 text-text-primary">{r.file_name}</td>
                    <td className="py-2 px-3">
                      {r.construction_pattern ? (
                        <ConstructionBadge pattern={r.construction_pattern} />
                      ) : (
                        <span className="text-text-tertiary">—</span>
                      )}
                    </td>
                    <td className="py-2 px-3 text-right text-text-primary">
                      {r.acs_score != null ? (
                        <span>
                          {r.acs_score.toFixed(1)}
                          {r.acs_margin != null && (
                            <span className="text-text-tertiary">
                              {" "}±{r.acs_margin.toFixed(0)}
                            </span>
                          )}
                        </span>
                      ) : (
                        "—"
                      )}
                    </td>
                    <td className="py-2 px-3 text-right text-text-primary">
                      {r.pii_score != null ? r.pii_score.toFixed(1) : "—"}
                    </td>
                    <td
                      className={`py-2 px-3 text-right ${anomalyCountColor(r.anomaly_count)}`}
                    >
                      {r.anomaly_count}
                    </td>
                    <td className="py-2 px-3 text-text-secondary">
                      {r.register || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <ReliabilityDisclosure disclosure={null} />
        </>
      )}

      {state.kind === "upload" && (
        <ReliabilityDisclosure disclosure={null} />
      )}
    </div>
  );
}
