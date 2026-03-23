"use client";

import { useEffect, useState } from "react";

const STAGES = [
  "Extracting text content",
  "Running lexical analysis",
  "Running syntactic analysis",
  "Running semantic analysis",
  "Running stylometric analysis",
  "Analyzing function words",
  "Computing n-grams",
  "Classifying register",
  "Computing baselines",
  "Running forensic analysis",
  "Computing scores",
  "Generating report",
];

export default function ProgressIndicator({ fileName }: { fileName: string }) {
  const [stageIndex, setStageIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setStageIndex((i) => (i < STAGES.length - 1 ? i + 1 : i));
    }, 700);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="border border-border p-6">
      <div className="flex items-center justify-between mb-4">
        <span className="text-sm text-text-secondary">Analyzing</span>
        <span className="font-mono text-xs text-text-primary">{fileName}</span>
      </div>

      <div className="w-full h-1 bg-bg-tertiary mb-4">
        <div
          className="h-full bg-accent progress-bar-animate"
        />
      </div>

      <div className="font-mono text-xs text-text-tertiary">
        <span className="text-accent">&gt;</span> {STAGES[stageIndex]}
        <span className="animate-pulse">_</span>
      </div>
    </div>
  );
}
