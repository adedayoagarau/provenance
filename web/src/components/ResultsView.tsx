"use client";

import { AnalysisResult } from "@/lib/types";
import ConstructionBadge from "./ConstructionBadge";
import RsidHeatmap from "./RsidHeatmap";
import AnomalyCard from "./AnomalyCard";
import SignalPanel from "./SignalPanel";
import ScorePanel from "./ScorePanel";
import ReliabilityDisclosure from "./ReliabilityDisclosure";

interface ResultsViewProps {
  fileName: string;
  fileSize: number;
  result: AnalysisResult;
  onBack: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function MetadataRow({ label, value }: { label: string; value: string | number | null | undefined }) {
  if (value == null || value === "") return null;
  return (
    <div className="flex items-baseline justify-between border-b border-border py-1.5">
      <span className="text-xs text-text-tertiary font-mono uppercase tracking-wide">
        {label}
      </span>
      <span className="text-sm font-mono text-text-primary">{value}</span>
    </div>
  );
}

export default function ResultsView({ fileName, fileSize, result, onBack }: ResultsViewProps) {
  const forensic = result.forensic_report;
  const docxProfile = forensic?.docx_profile;
  const metadata = docxProfile?.metadata || forensic?.document_metadata;
  const rsid = docxProfile?.rsid_analysis;
  const anomalyReport = result.anomaly_report;
  const baseline = result.baseline_report;
  const register = result.register;
  const acs = result.acs;
  const pii = result.pii;
  const evaluator = result.evaluator_report;
  const analysis = result.analysis_result;

  return (
    <div className="space-y-6">
      {/* Header bar */}
      <div className="flex items-center justify-between">
        <button
          onClick={onBack}
          className="text-xs font-mono text-text-tertiary hover:text-text-primary transition-colors"
        >
          &larr; New analysis
        </button>
        <span className="text-xs font-mono text-text-tertiary">
          {result.audit.timestamp}
        </span>
      </div>

      {/* Document header */}
      <div className="border border-border p-5">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="font-mono text-lg font-semibold text-text-primary">{fileName}</h1>
            <span className="text-xs font-mono text-text-tertiary">
              {formatBytes(fileSize)} &middot; {forensic?.format_detected || "unknown format"}
            </span>
          </div>
          {docxProfile && (
            <ConstructionBadge pattern={docxProfile.construction_pattern} />
          )}
        </div>

        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-x-6">
          <MetadataRow label="Words" value={analysis?.lexical?.word_count} />
          <MetadataRow label="Editor" value={metadata?.application} />
          <MetadataRow label="Author" value={metadata?.author} />
          <MetadataRow label="Created" value={metadata?.creation_date} />
          <MetadataRow
            label="Dev span"
            value={
              docxProfile?.creation_to_modification_hours != null
                ? `${docxProfile.creation_to_modification_hours.toFixed(1)}h`
                : null
            }
          />
          <MetadataRow
            label="Edit time"
            value={
              metadata?.custom?.TotalTime
                ? `${(parseFloat(metadata.custom.TotalTime) / 60).toFixed(1)}h`
                : null
            }
          />
        </div>

        {docxProfile?.assessment_text && (
          <p className="text-xs text-text-secondary mt-3 leading-relaxed">
            {docxProfile.assessment_text}
          </p>
        )}
      </div>

      {/* Process Evidence — RSID Heatmap */}
      {rsid && (
        <div className="border border-border p-5">
          <div className="text-xs font-mono text-text-secondary uppercase tracking-wide mb-4">
            Process Evidence
          </div>
          <RsidHeatmap rsid={rsid} />
        </div>
      )}

      {/* Anomaly Cards */}
      {anomalyReport && anomalyReport.flags.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-xs font-mono text-text-secondary uppercase tracking-wide">
              Anomaly Flags
            </div>
            <div className="flex items-center gap-3 text-xs font-mono">
              {anomalyReport.high_count > 0 && (
                <span className="text-status-red">{anomalyReport.high_count} high</span>
              )}
              {anomalyReport.medium_count > 0 && (
                <span className="text-status-amber">{anomalyReport.medium_count} medium</span>
              )}
              {anomalyReport.low_count > 0 && (
                <span className="text-status-blue">{anomalyReport.low_count} low</span>
              )}
            </div>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {anomalyReport.flags.map((flag, i) => (
              <AnomalyCard key={i} flag={flag} />
            ))}
          </div>
        </div>
      )}

      {/* Signal Breakdown */}
      {baseline && (
        <div className="border border-border p-5">
          {register && (
            <div className="text-xs text-text-tertiary mb-2">
              Detected register:{" "}
              <span className="text-text-secondary font-mono">{register.label}</span>
              {register.confidence && (
                <span className="text-text-tertiary">
                  {" "}({(register.confidence * 100).toFixed(0)}% confidence)
                </span>
              )}
              <span className="text-text-tertiary block mt-1">
                Signals marked &quot;expected&quot; are typical for this text type and should not be treated as anomalous.
              </span>
            </div>
          )}
          <SignalPanel baseline={baseline} registerLabel={register?.label} />
        </div>
      )}

      {/* Text Analysis Summary (when no baseline available) */}
      {!baseline && analysis && (
        <div className="border border-border p-5">
          <div className="text-xs font-mono text-text-secondary uppercase tracking-wide mb-3">
            Text Analysis Summary
          </div>
          <div className="grid grid-cols-2 md:grid-cols-3 gap-x-6 gap-y-1">
            <MetadataRow label="TTR" value={analysis.lexical.type_token_ratio?.toFixed(3)} />
            <MetadataRow label="MATTR" value={analysis.lexical.mattr?.toFixed(3)} />
            <MetadataRow label="Yule's K" value={analysis.lexical.yules_k?.toFixed(1)} />
            <MetadataRow label="Avg sentence" value={`${analysis.syntactic.avg_sentence_length?.toFixed(1)} words`} />
            <MetadataRow label="Passive voice" value={`${(analysis.syntactic.passive_voice_ratio * 100).toFixed(1)}%`} />
            <MetadataRow label="FK Grade" value={analysis.semantic.flesch_kincaid_grade?.toFixed(1)} />
            <MetadataRow label="Contractions" value={`${(analysis.stylometric.contraction_ratio * 100).toFixed(1)}%`} />
            <MetadataRow label="Hedges" value={`${(analysis.stylometric.hedge_word_density * 1000).toFixed(1)}/1k`} />
            <MetadataRow label="Func word ratio" value={`${(analysis.function_words.function_word_ratio * 100).toFixed(1)}%`} />
          </div>
        </div>
      )}

      {/* Scoring */}
      {(acs || pii) && (
        <div className="border border-border p-5">
          <ScorePanel acs={acs} pii={pii} evaluator={evaluator} />
        </div>
      )}

      {/* Reliability Disclosure — ALWAYS visible */}
      <ReliabilityDisclosure
        disclosure={evaluator?.reliability_disclosure || null}
      />
    </div>
  );
}
