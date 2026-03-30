"""Evasion Resistance Researcher.

Studies how AI text survives humanization, paraphrasing, and other
evasion techniques. Identifies robust features that resist attack.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import BaseResearcher, compare_groups, cohens_d, TEXT_FEATURES


class EvasionResearcher(BaseResearcher):
    name = "evasion_research"
    description = "Evasion resistance and humanization analysis"

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        human_df = df[df["label"] == "human"]
        ai_df = df[df["label"] == "ai"]
        humanized_df = df[df["label"] == "humanized"]

        has_humanized = len(humanized_df) >= 100
        self.logger.info("Samples -- human: %d, ai: %d, humanized: %d",
                        len(human_df), len(ai_df), len(humanized_df))

        # AI vs Human baseline
        ai_vs_human = []
        for feature in TEXT_FEATURES:
            if feature in df.columns:
                result = compare_groups(df, feature, group_a_val="human", group_b_val="ai")
                if result.get("status") == "ok":
                    ai_vs_human.append(result)

        # Humanized vs Human (what still leaks?)
        humanized_vs_human = []
        if has_humanized:
            for feature in TEXT_FEATURES:
                if feature in df.columns:
                    a = human_df[feature].dropna().values
                    b = humanized_df[feature].dropna().values
                    if len(a) >= 100 and len(b) >= 50:
                        d = cohens_d(a, b)
                        humanized_vs_human.append({
                            "feature": feature,
                            "cohens_d_humanized_vs_human": float(d),
                        })

        # Signal retention: how much AI signal survives humanization?
        signal_retention = []
        if has_humanized:
            for f_ah in ai_vs_human:
                feature = f_ah["feature"]
                match = [f for f in humanized_vs_human if f["feature"] == feature]
                if match:
                    d_original = abs(f_ah["cohens_d"])
                    d_after = abs(match[0]["cohens_d_humanized_vs_human"])
                    retention = (d_after / d_original * 100) if d_original > 0 else 0
                    signal_retention.append({
                        "feature": feature,
                        "d_ai_vs_human": f_ah["cohens_d"],
                        "d_humanized_vs_human": match[0]["cohens_d_humanized_vs_human"],
                        "retention_pct": retention,
                        "robust": retention > 50,
                    })
            signal_retention.sort(key=lambda x: x["retention_pct"], reverse=True)

        # Prompt type analysis
        prompt_analysis = {}
        if "prompt_type" in df.columns:
            for pt in df["prompt_type"].dropna().unique()[:10]:
                pt_df = df[df["prompt_type"] == pt]
                if len(pt_df[pt_df["label"] == "ai"]) >= 50:
                    top_feature = ai_vs_human[0]["feature"] if ai_vs_human else TEXT_FEATURES[0]
                    if top_feature in pt_df.columns:
                        a = pt_df.loc[pt_df["label"] == "human", top_feature].dropna().values
                        b = pt_df.loc[pt_df["label"] == "ai", top_feature].dropna().values
                        if len(a) >= 50 and len(b) >= 50:
                            prompt_analysis[pt] = float(cohens_d(a, b))

        return {
            "title": "Evasion Resistance: What Survives Humanization",
            "n_human": len(human_df),
            "n_ai": len(ai_df),
            "n_humanized": len(humanized_df),
            "has_humanized": has_humanized,
            "ai_vs_human": ai_vs_human,
            "signal_retention": signal_retention,
            "robust_features": [s for s in signal_retention if s.get("robust")],
            "fragile_features": [s for s in signal_retention if not s.get("robust")],
            "prompt_analysis": prompt_analysis,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("Which AI detection features survive humanization attacks, "
                     "and what percentage of the AI signal is retained?\n")

        lines.append("## Dataset\n")
        lines.append(f"- Human: {findings['n_human']:,} | AI: {findings['n_ai']:,} "
                     f"| Humanized: {findings['n_humanized']:,}\n")

        if findings["has_humanized"] and findings["signal_retention"]:
            lines.append("## Signal Retention Analysis\n")
            lines.append("How much of the original AI signal survives humanization:\n")
            lines.append("| Feature | d (AI vs Human) | d (Humanized vs Human) | Retention | Robust? |")
            lines.append("|---------|----------------|----------------------|-----------|---------|")
            for s in findings["signal_retention"][:15]:
                robust = "YES" if s["robust"] else "no"
                lines.append(f"| {s['feature']} | {s['d_ai_vs_human']:+.3f} "
                            f"| {s['d_humanized_vs_human']:+.3f} "
                            f"| {s['retention_pct']:.0f}% | {robust} |")

            n_robust = len(findings["robust_features"])
            n_fragile = len(findings["fragile_features"])
            lines.append(f"\n**{n_robust} features are robust** (>50% signal retention)")
            lines.append(f"**{n_fragile} features are fragile** (<50% signal retention)\n")
        else:
            lines.append("## Note\n")
            lines.append("No humanized samples found. Add humanized text to the dataset "
                        "to enable evasion resistance analysis.\n")

        if findings["prompt_analysis"]:
            lines.append("## Prompt Type Analysis\n")
            lines.append("Detection difficulty by prompt strategy:\n")
            for pt, d in sorted(findings["prompt_analysis"].items(), key=lambda x: abs(x[1])):
                difficulty = "HARD" if abs(d) < 0.3 else ("MEDIUM" if abs(d) < 0.5 else "EASY")
                lines.append(f"- **{pt}**: d={d:+.3f} ({difficulty})")

        lines.append("\n## Recommendations\n")
        lines.append("1. Build detection ensembles using only robust features\n")
        lines.append("2. Anti-detection prompts need special attention in training data\n")
        lines.append("3. Fragile features are still useful for non-adversarial detection\n")

        return "\n".join(lines)
