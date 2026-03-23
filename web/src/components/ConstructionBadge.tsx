import { ConstructionPattern } from "@/lib/types";

const PATTERN_CONFIG: Record<ConstructionPattern, { label: string; className: string }> = {
  Organic: { label: "Organic", className: "pattern-organic" },
  Hybrid: { label: "Hybrid", className: "pattern-hybrid" },
  BulkInsertion: { label: "Bulk Insertion", className: "pattern-bulkinsertion" },
  Insufficient: { label: "Insufficient Data", className: "pattern-insufficient" },
};

export default function ConstructionBadge({ pattern }: { pattern: ConstructionPattern }) {
  const config = PATTERN_CONFIG[pattern] || PATTERN_CONFIG.Insufficient;
  return (
    <span
      className={`inline-block px-3 py-1 text-xs font-mono font-semibold uppercase tracking-wider ${config.className}`}
    >
      {config.label}
    </span>
  );
}
