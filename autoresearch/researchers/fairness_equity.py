"""Fairness & Equity Researcher.

Analyzes detection bias across sources, registers, and text characteristics
to identify where AI detection may unfairly flag human text.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import BaseResearcher, compare_groups, cohens_d, TEXT_FEATURES


class FairnessEquityResearcher(BaseResearcher):
    name = "fairness_equity"
    description = "Fairness, NNES protection, and bias analysis"

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        human_df = df[df["label"] == "human"]
        ai_df = df[df["label"] == "ai"]

        # Source-specific analysis: do some human sources look more "AI-like"?
        source_risk = {}
        if "source" in df.columns:
            for source in human_df["source"].dropna().unique():
                source_df = human_df[human_df["source"] == source]
                if len(source_df) >= 50:
                    # How similar is this source to AI text?
                    similarities = []
                    for feature in TEXT_FEATURES[:10]:
                        if feature in df.columns:
                            human_src = source_df[feature].dropna().values
                            ai_vals = ai_df[feature].dropna().values
                            if len(human_src) >= 30 and len(ai_vals) >= 100:
                                d = cohens_d(human_src, ai_vals)
                                similarities.append(abs(float(d)))
                    if similarities:
                        source_risk[source] = {
                            "n_samples": len(source_df),
                            "avg_distance_from_ai": float(np.mean(similarities)),
                            "false_positive_risk": "HIGH" if np.mean(similarities) < 0.3 else (
                                "MEDIUM" if np.mean(similarities) < 0.5 else "LOW"),
                        }

        # Register-specific false positive risk
        register_risk = {}
        if "register" in df.columns:
            for reg in df["register"].dropna().unique():
                reg_human = df[(df["register"] == reg) & (df["label"] == "human")]
                reg_ai = df[(df["register"] == reg) & (df["label"] == "ai")]
                if len(reg_human) >= 50 and len(reg_ai) >= 50:
                    effects = []
                    for feature in TEXT_FEATURES[:10]:
                        if feature in df.columns:
                            a = reg_human[feature].dropna().values
                            b = reg_ai[feature].dropna().values
                            if len(a) >= 30 and len(b) >= 30:
                                effects.append(abs(float(cohens_d(a, b))))
                    if effects:
                        register_risk[reg] = {
                            "avg_effect": float(np.mean(effects)),
                            "detection_difficulty": "HARD" if np.mean(effects) < 0.3 else "MODERATE",
                        }

        # Word count analysis: detection reliability by text length
        length_bins = [(50, 200), (200, 500), (500, 1000), (1000, 2000), (2000, 10000)]
        length_reliability = {}
        if "word_count" in df.columns:
            for lo, hi in length_bins:
                bin_df = df[(df["word_count"] >= lo) & (df["word_count"] < hi)]
                if len(bin_df[bin_df["label"] == "human"]) >= 50:
                    effects = []
                    for feature in TEXT_FEATURES[:10]:
                        if feature in bin_df.columns:
                            result = compare_groups(bin_df, feature)
                            if result.get("status") == "ok":
                                effects.append(abs(result["cohens_d"]))
                    if effects:
                        length_reliability[f"{lo}-{hi}"] = {
                            "avg_effect": float(np.mean(effects)),
                            "n_samples": len(bin_df),
                        }

        return {
            "title": "Fairness Audit: Detection Bias and False Positive Risk",
            "source_risk": source_risk,
            "register_risk": register_risk,
            "length_reliability": length_reliability,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("Where does AI detection risk false positives, and which "
                     "populations or text types are most at risk?\n")

        lines.append("## Source-Specific False Positive Risk\n")
        if findings["source_risk"]:
            lines.append("| Source | Samples | Distance from AI | Risk Level |")
            lines.append("|--------|---------|-----------------|------------|")
            for src, info in sorted(findings["source_risk"].items(),
                                   key=lambda x: x[1]["avg_distance_from_ai"]):
                lines.append(f"| {src} | {info['n_samples']:,} | "
                            f"{info['avg_distance_from_ai']:.3f} | "
                            f"{info['false_positive_risk']} |")

        lines.append("\n## Register-Specific Detection Difficulty\n")
        if findings["register_risk"]:
            for reg, info in sorted(findings["register_risk"].items(),
                                   key=lambda x: x[1]["avg_effect"]):
                lines.append(f"- **{reg}**: avg effect = {info['avg_effect']:.3f} "
                            f"({info['detection_difficulty']})")

        lines.append("\n## Detection Reliability by Text Length\n")
        if findings["length_reliability"]:
            for length_bin, info in findings["length_reliability"].items():
                lines.append(f"- **{length_bin} words**: avg effect = "
                            f"{info['avg_effect']:.3f} (n={info['n_samples']:,})")

        lines.append("\n## Recommendations\n")
        lines.append("1. Flag high-risk sources for manual review rather than auto-classification\n")
        lines.append("2. Use register-specific thresholds to reduce false positives\n")
        lines.append("3. Set minimum word count requirements for reliable detection\n")
        lines.append("4. Report confidence intervals, not just binary predictions\n")

        return "\n".join(lines)
