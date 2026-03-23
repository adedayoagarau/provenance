export default function SeverityBadge({ severity }: { severity: "Low" | "Medium" | "High" }) {
  const cls =
    severity === "High"
      ? "severity-high"
      : severity === "Medium"
        ? "severity-medium"
        : "severity-low";

  return (
    <span className={`inline-block px-2 py-0.5 text-xs font-mono uppercase tracking-wide ${cls}`}>
      {severity}
    </span>
  );
}
