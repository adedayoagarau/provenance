import { ReliabilityDisclosure as Disclosure } from "@/lib/types";

const CANNOT_TELL_YOU = [
  "Whether a specific person wrote this document — only whether the writing style is consistent with available evidence",
  "Whether any text was produced by a language model — stylometric analysis measures human writing patterns, not machine signatures",
  "The intent behind any detected anomalies — unusual patterns have many innocent explanations",
  "Whether the document is \"authentic\" or \"fraudulent\" — these are human judgments that require context this tool does not have",
];

export default function ReliabilityDisclosure({
  disclosure,
}: {
  disclosure: Disclosure | null;
}) {
  return (
    <div className="space-y-4 mt-8">
      {/* Reliability disclosure - ALWAYS visible, NEVER collapsed */}
      <div className="border border-border p-5">
        <div className="text-xs font-mono text-status-amber uppercase tracking-wide mb-3">
          {disclosure?.header || "Reliability & Limitations"}
        </div>

        {disclosure?.points && disclosure.points.length > 0 ? (
          <ul className="space-y-2">
            {disclosure.points.map((point, i) => (
              <li key={i} className="text-xs text-text-secondary leading-relaxed flex gap-2">
                <span className="text-text-tertiary shrink-0">—</span>
                {point}
              </li>
            ))}
          </ul>
        ) : (
          <ul className="space-y-2">
            <li className="text-xs text-text-secondary leading-relaxed flex gap-2">
              <span className="text-text-tertiary shrink-0">—</span>
              This tool analyzes writing style patterns and document metadata. It does not and
              cannot determine whether text was written by a human or machine.
            </li>
            <li className="text-xs text-text-secondary leading-relaxed flex gap-2">
              <span className="text-text-tertiary shrink-0">—</span>
              Stylometric analysis is a statistical technique with known error rates. Results
              should be treated as one input among many, not as definitive evidence.
            </li>
            <li className="text-xs text-text-secondary leading-relaxed flex gap-2">
              <span className="text-text-tertiary shrink-0">—</span>
              Short texts (under 500 words) produce less reliable results due to insufficient
              data for stable stylometric measurement.
            </li>
          </ul>
        )}

        {disclosure?.evidence_basis && (
          <p className="text-xs text-text-tertiary mt-3">{disclosure.evidence_basis}</p>
        )}
      </div>

      {/* "What this cannot tell you" - ALWAYS visible, NEVER collapsed */}
      <div className="border border-border p-5">
        <div className="text-xs font-mono text-status-amber uppercase tracking-wide mb-3">
          What This Cannot Tell You
        </div>
        <ul className="space-y-2">
          {CANNOT_TELL_YOU.map((item, i) => (
            <li key={i} className="text-xs text-text-secondary leading-relaxed flex gap-2">
              <span className="text-text-tertiary shrink-0">—</span>
              {item}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
