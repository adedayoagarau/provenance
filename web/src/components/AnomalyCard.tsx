import { AnomalyFlag, AnomalyLocation } from "@/lib/types";
import SeverityBadge from "./SeverityBadge";

function formatLocation(loc: AnomalyLocation): string {
  if (loc === "DocumentLevel") return "Document-wide";
  if (typeof loc === "object" && "ParagraphRange" in loc) {
    return `¶${loc.ParagraphRange.start}–${loc.ParagraphRange.end}`;
  }
  if (typeof loc === "object" && "Section" in loc) {
    return loc.Section;
  }
  return "—";
}

function severityBorderColor(severity: string): string {
  switch (severity) {
    case "High":
      return "border-l-status-red";
    case "Medium":
      return "border-l-status-amber";
    default:
      return "border-l-status-blue";
  }
}

export default function AnomalyCard({ flag }: { flag: AnomalyFlag }) {
  return (
    <div
      className={`border border-border border-l-2 ${severityBorderColor(flag.severity)} p-4`}
    >
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs text-text-tertiary uppercase">
            {flag.anomaly_type.replace(/([A-Z])/g, " $1").trim()}
          </span>
          <span className="font-mono text-xs text-text-tertiary">
            {formatLocation(flag.location)}
          </span>
        </div>
        <SeverityBadge severity={flag.severity} />
      </div>

      <p className="text-sm text-text-primary mb-2">{flag.description}</p>

      <div className="text-xs text-text-secondary mb-2">
        <span className="text-text-tertiary uppercase font-mono tracking-wide">
          Recommended:
        </span>{" "}
        {flag.recommended_action}
      </div>

      {flag.contributing_signals.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-2">
          {flag.contributing_signals.map((signal, i) => (
            <span
              key={i}
              className="text-xs font-mono px-1.5 py-0.5 bg-bg-tertiary text-text-tertiary border border-border"
            >
              {signal}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
