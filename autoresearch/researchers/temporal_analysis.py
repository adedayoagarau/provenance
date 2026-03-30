"""Temporal Analysis Researcher.

Studies how AI-generated text evolves over time and across model generations.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import BaseResearcher, cohens_d, TEXT_FEATURES


class TemporalAnalysisResearcher(BaseResearcher):
    name = "temporal_analysis"
    description = "Temporal drift and model evolution analysis"

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        # Parse timestamps if available
        if "timestamp" in df.columns:
            df = df.copy()
            df["ts"] = pd.to_datetime(df["timestamp"], errors="coerce")
            df["month"] = df["ts"].dt.to_period("M")
            has_time = df["ts"].notna().sum() > 100
        else:
            has_time = False

        # Model generation comparison
        models = df[df["label"] == "ai"]["model"].dropna().unique().tolist()
        model_profiles = {}
        for model in models:
            model_df = df[(df["label"] == "ai") & (df["model"] == model)]
            if len(model_df) >= 50:
                profile = {}
                for feature in TEXT_FEATURES[:15]:
                    if feature in model_df.columns:
                        vals = model_df[feature].dropna()
                        if len(vals) >= 30:
                            profile[feature] = {
                                "mean": float(vals.mean()),
                                "std": float(vals.std()),
                            }
                model_profiles[model] = {"n": len(model_df), "features": profile}

        # Feature stability across time
        temporal_stability = {}
        if has_time:
            months = sorted(df["month"].dropna().unique())
            if len(months) >= 2:
                for feature in TEXT_FEATURES[:10]:
                    if feature in df.columns:
                        early = df[df["month"] <= months[len(months)//2]]
                        late = df[df["month"] > months[len(months)//2]]
                        ai_early = early.loc[early["label"] == "ai", feature].dropna().values
                        ai_late = late.loc[late["label"] == "ai", feature].dropna().values
                        if len(ai_early) >= 50 and len(ai_late) >= 50:
                            d = cohens_d(ai_early, ai_late)
                            temporal_stability[feature] = {
                                "drift": float(d),
                                "stable": abs(d) < 0.2,
                            }

        return {
            "title": "Temporal Analysis: How AI Text Evolves",
            "has_temporal_data": has_time,
            "n_models": len(models),
            "model_profiles": model_profiles,
            "temporal_stability": temporal_stability,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("How does AI-generated text change across models and over time?\n")

        lines.append("## Model Generation Profiles\n")
        for model, info in findings["model_profiles"].items():
            lines.append(f"\n### {model} (n={info['n']:,})\n")
            for feature, stats in list(info["features"].items())[:5]:
                lines.append(f"- {feature}: mean={stats['mean']:.4f}, std={stats['std']:.4f}")

        if findings["temporal_stability"]:
            lines.append("\n## Feature Stability Over Time\n")
            stable = [f for f, info in findings["temporal_stability"].items() if info["stable"]]
            drifting = [f for f, info in findings["temporal_stability"].items() if not info["stable"]]
            lines.append(f"- **Stable features** ({len(stable)}): {', '.join(stable[:5])}")
            lines.append(f"- **Drifting features** ({len(drifting)}): {', '.join(drifting[:5])}")

        lines.append("\n## Recommendations\n")
        lines.append("1. Build detection on temporally stable features\n")
        lines.append("2. Retrain models periodically as AI text evolves\n")
        lines.append("3. Monitor feature drift as a trigger for retraining\n")

        return "\n".join(lines)
