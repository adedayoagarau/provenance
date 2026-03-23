import { BaselineComparison, RegisterBaselineReport } from "@/lib/types";

function assessmentColor(assessment: string): string {
  switch (assessment) {
    case "Expected":
      return "text-status-blue";
    case "Atypical":
      return "text-status-amber";
    case "Anomalous":
      return "text-status-red";
    default:
      return "text-text-tertiary";
  }
}

function assessmentBg(assessment: string): string {
  switch (assessment) {
    case "Expected":
      return "bg-status-blue-dim border-status-blue";
    case "Atypical":
      return "bg-status-amber-dim border-status-amber";
    case "Anomalous":
      return "bg-status-red-dim border-status-red";
    default:
      return "bg-status-gray-dim border-status-gray";
  }
}

function SignalRow({ comp }: { comp: BaselineComparison }) {
  const deviation = Math.abs(comp.z_score);
  const barWidth = Math.min(deviation / 3 * 100, 100);

  return (
    <div className="border border-border p-3">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-mono text-text-primary">
          {comp.metric_name}
        </span>
        <span
          className={`text-xs font-mono px-2 py-0.5 border ${assessmentBg(comp.assessment)}`}
        >
          {comp.assessment.toLowerCase()}
        </span>
      </div>

      <div className="flex items-center gap-3 mb-1.5">
        <div className="flex-1 h-1 bg-bg-tertiary">
          <div
            className={`h-full ${
              comp.assessment === "Anomalous"
                ? "bg-status-red"
                : comp.assessment === "Atypical"
                  ? "bg-status-amber"
                  : "bg-status-blue"
            }`}
            style={{ width: `${barWidth}%` }}
          />
        </div>
        <span className="text-xs font-mono text-text-tertiary w-16 text-right">
          z={comp.z_score.toFixed(2)}
        </span>
      </div>

      <div className="flex justify-between text-xs font-mono text-text-tertiary">
        <span>
          actual: <span className={assessmentColor(comp.assessment)}>{comp.actual_value.toFixed(3)}</span>
        </span>
        <span>
          expected: {comp.expected_mean.toFixed(3)} ±{comp.expected_stddev.toFixed(3)}
        </span>
      </div>

      {comp.explanation && (
        <div className="mt-1.5 text-xs text-text-tertiary">
          {comp.explanation}
        </div>
      )}
    </div>
  );
}

export default function SignalPanel({
  baseline,
  registerLabel,
}: {
  baseline: RegisterBaselineReport;
  registerLabel?: string;
}) {
  const { comparisons, anomalous_count, atypical_count, expected_count } = baseline;

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <div className="text-xs text-text-secondary font-mono uppercase tracking-wide">
          Signal Breakdown
          {registerLabel && (
            <span className="text-text-tertiary ml-2">
              — register: {registerLabel}
            </span>
          )}
        </div>
        <div className="flex items-center gap-3 text-xs font-mono">
          <span className="text-status-blue">{expected_count} expected</span>
          <span className="text-status-amber">{atypical_count} atypical</span>
          <span className="text-status-red">{anomalous_count} anomalous</span>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
        {comparisons.map((comp, i) => (
          <SignalRow key={i} comp={comp} />
        ))}
      </div>
    </div>
  );
}
