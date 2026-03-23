import { AuthorshipConfidenceScore, ProcessIntegrityIndex, EvaluatorReport } from "@/lib/types";

function ScoreGauge({ value, low, high, label }: { value: number; low: number; high: number; label: string }) {
  const clampedValue = Math.max(0, Math.min(100, value));

  return (
    <div className="border border-border p-4">
      <div className="text-xs font-mono text-text-tertiary uppercase tracking-wide mb-3">
        {label}
      </div>
      <div className="flex items-end gap-2 mb-2">
        <span className="font-mono text-3xl font-bold text-text-primary">
          {clampedValue.toFixed(0)}
        </span>
        <span className="font-mono text-sm text-text-tertiary mb-1">/100</span>
      </div>
      <div className="w-full h-2 bg-bg-tertiary relative">
        {/* Confidence interval range */}
        <div
          className="absolute h-full bg-status-blue-dim"
          style={{
            left: `${Math.max(0, low)}%`,
            width: `${Math.min(100, high) - Math.max(0, low)}%`,
          }}
        />
        {/* Score marker */}
        <div
          className="absolute h-full w-0.5 bg-accent"
          style={{ left: `${clampedValue}%` }}
        />
      </div>
      <div className="flex justify-between mt-1 text-xs font-mono text-text-tertiary">
        <span>{low.toFixed(0)}</span>
        <span>±{((high - low) / 2).toFixed(0)} margin</span>
        <span>{high.toFixed(0)}</span>
      </div>
    </div>
  );
}

function IntegrityLevel({ pii }: { pii: ProcessIntegrityIndex }) {
  const levelColors: Record<string, string> = {
    Strong: "text-status-blue border-status-blue bg-status-blue-dim",
    Moderate: "text-status-amber border-status-amber bg-status-amber-dim",
    Limited: "text-status-amber border-status-amber bg-status-amber-dim",
    Insufficient: "text-status-gray border-status-gray bg-status-gray-dim",
  };

  return (
    <div className="border border-border p-4">
      <div className="text-xs font-mono text-text-tertiary uppercase tracking-wide mb-3">
        Process Integrity Index
      </div>
      <div className="flex items-end gap-2 mb-2">
        <span className="font-mono text-3xl font-bold text-text-primary">
          {pii.score.toFixed(0)}
        </span>
        <span className="font-mono text-sm text-text-tertiary mb-1">/100</span>
      </div>
      <div
        className={`inline-block px-2 py-0.5 text-xs font-mono border ${levelColors[pii.level] || levelColors.Insufficient}`}
      >
        {pii.level}
      </div>
      <p className="text-xs text-text-secondary mt-2">{pii.guidance}</p>

      {/* Component breakdown */}
      <div className="mt-3 space-y-1">
        {[
          { label: "Metadata", value: pii.components.metadata_completeness, max: 25 },
          { label: "RSID richness", value: pii.components.rsid_richness, max: 30 },
          { label: "Formatting", value: pii.components.formatting_quality, max: 20 },
          { label: "Revision evidence", value: pii.components.revision_evidence, max: 15 },
          { label: "Additional", value: pii.components.additional_evidence, max: 10 },
        ].map((c) => (
          <div key={c.label} className="flex items-center gap-2 text-xs font-mono">
            <span className="text-text-tertiary w-28 shrink-0">{c.label}</span>
            <div className="flex-1 h-1 bg-bg-tertiary">
              <div
                className="h-full bg-accent"
                style={{ width: `${(c.value / c.max) * 100}%` }}
              />
            </div>
            <span className="text-text-secondary w-12 text-right">
              {c.value.toFixed(1)}/{c.max}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function ScorePanel({
  acs,
  pii,
  evaluator,
}: {
  acs: AuthorshipConfidenceScore | null;
  pii: ProcessIntegrityIndex | null;
  evaluator: EvaluatorReport | null;
}) {
  return (
    <div>
      <div className="text-xs text-text-secondary font-mono uppercase tracking-wide mb-3">
        Scoring
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {acs && (
          <ScoreGauge
            value={acs.score}
            low={acs.score_low}
            high={acs.score_high}
            label="Authorship Confidence Score (ACS)"
          />
        )}
        {pii && <IntegrityLevel pii={pii} />}
      </div>

      {evaluator && (
        <div className="mt-3 border border-border p-4">
          <div className="text-xs font-mono text-text-tertiary uppercase tracking-wide mb-2">
            Assessment
          </div>
          <p className="text-sm text-text-secondary">{evaluator.executive_summary}</p>

          {evaluator.authorship_assessment && (
            <div className="mt-3">
              <p className="text-sm text-text-primary">
                {evaluator.authorship_assessment.interpretation}
              </p>
              {evaluator.authorship_assessment.caveats.length > 0 && (
                <ul className="mt-2 space-y-1">
                  {evaluator.authorship_assessment.caveats.map((c, i) => (
                    <li key={i} className="text-xs text-text-tertiary">
                      — {c}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
