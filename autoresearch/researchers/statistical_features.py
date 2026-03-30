"""Statistical Features Researcher.

Deep statistical analysis of features that discriminate AI from human text.
Computes effect sizes, distributions, and correlations across 1M+ samples.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import (
    BaseResearcher, compare_groups, TEXT_FEATURES,
    load_dataset, compute_features_batch,
)


class StatisticalFeaturesResearcher(BaseResearcher):
    name = "statistical_features"
    description = "Core AI vs Human statistical feature analysis"

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        self.logger.info("Analyzing %d samples across %d features", len(df), len(TEXT_FEATURES))

        comparisons = []
        for feature in TEXT_FEATURES:
            if feature in df.columns:
                result = compare_groups(df, feature)
                comparisons.append(result)

        # Sort by absolute effect size
        ok = [c for c in comparisons if c.get("status") == "ok"]
        ok.sort(key=lambda c: abs(c.get("cohens_d", 0)), reverse=True)

        # Top discriminators
        notable = [c for c in ok if c.get("notable_effect")]

        # Distribution stats by label
        label_counts = df["label"].value_counts().to_dict()

        # Per-register breakdown for top features
        register_breakdown = {}
        if "register" in df.columns and len(notable) > 0:
            top_feature = notable[0]["feature"]
            for reg in df["register"].dropna().unique()[:10]:
                reg_df = df[df["register"] == reg]
                if len(reg_df) > 200:
                    result = compare_groups(reg_df, top_feature)
                    if result.get("status") == "ok":
                        register_breakdown[reg] = result["cohens_d"]

        return {
            "title": "AI vs Human Text: Statistical Feature Analysis",
            "n_samples": len(df),
            "label_counts": label_counts,
            "all_comparisons": ok,
            "notable_features": notable,
            "n_notable": len(notable),
            "n_total_features": len(TEXT_FEATURES),
            "register_breakdown": register_breakdown,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("Which measurable text features most reliably discriminate "
                     "AI-generated text from human-written text, and what are their effect sizes?\n")

        lines.append("## Dataset\n")
        lines.append(f"- **Total samples analyzed:** {findings['n_samples']:,}")
        for label, count in findings["label_counts"].items():
            lines.append(f"- **{label}:** {count:,} samples")

        lines.append("\n## Key Findings\n")
        lines.append(f"Of {findings['n_total_features']} features analyzed, "
                     f"**{findings['n_notable']}** showed notable effect sizes (|d| >= 0.2).\n")

        lines.append("### Top Discriminative Features\n")
        lines.append(self._format_findings_table(findings["all_comparisons"]))

        lines.append("\n### Most Powerful Discriminators\n")
        for f in findings["notable_features"][:5]:
            lines.append(self._format_finding(f))

        if findings["register_breakdown"]:
            lines.append("\n### Register-Specific Analysis\n")
            lines.append("Effect size of the top feature across registers:\n")
            for reg, d in sorted(findings["register_breakdown"].items(), key=lambda x: abs(x[1]), reverse=True):
                lines.append(f"- **{reg}:** d={d:+.3f}")

        lines.append("\n## Implications for AI Detection\n")
        if findings["notable_features"]:
            top = findings["notable_features"][0]
            lines.append(f"The strongest discriminator is **{top['feature']}** "
                        f"with d={top['cohens_d']:+.3f}. ")
            lines.append("Features with |d| > 0.5 are strong candidates for "
                        "production classifiers. Features with 0.2 < |d| < 0.5 "
                        "add value in ensemble models.\n")

        lines.append("## Recommendations\n")
        lines.append("1. Prioritize features with |d| > 0.5 for the detection pipeline\n")
        lines.append("2. Investigate register-specific thresholds for improved accuracy\n")
        lines.append("3. Features with large effect sizes but high variance may need "
                    "sample-size-aware confidence scoring\n")

        return "\n".join(lines)
