"""Model Signatures Researcher.

Analyzes how different AI models (Llama3, Mistral, Gemma2, etc.) leave
distinct statistical fingerprints in generated text.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import BaseResearcher, compare_groups, cohens_d, TEXT_FEATURES


class ModelSignaturesResearcher(BaseResearcher):
    name = "model_signatures"
    description = "Per-model fingerprinting analysis"

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        ai_df = df[df["label"] == "ai"].copy()
        models = [m for m in ai_df["model"].dropna().unique() if len(ai_df[ai_df["model"] == m]) >= 100]
        self.logger.info("Analyzing %d models: %s", len(models), models)

        human_df = df[df["label"] == "human"]

        # Per-model vs human comparison
        model_vs_human = {}
        for model in models:
            model_df = ai_df[ai_df["model"] == model]
            model_findings = []
            for feature in TEXT_FEATURES:
                if feature in df.columns:
                    a = human_df[feature].dropna().values
                    b = model_df[feature].dropna().values
                    if len(a) >= 100 and len(b) >= 100:
                        d = cohens_d(a, b)
                        model_findings.append({"feature": feature, "cohens_d": float(d),
                                               "mean_human": float(np.nanmean(a)),
                                               "mean_model": float(np.nanmean(b))})
            model_findings.sort(key=lambda f: abs(f.get("cohens_d", 0)), reverse=True)
            model_vs_human[model] = {
                "n_samples": len(model_df),
                "top_features": model_findings[:10],
                "avg_effect_size": np.mean([abs(f["cohens_d"]) for f in model_findings]) if model_findings else 0,
            }

        # Cross-model comparison: which model is hardest to detect?
        detectability = [(m, info["avg_effect_size"]) for m, info in model_vs_human.items()]
        detectability.sort(key=lambda x: x[1])

        # Unique signatures: features where models differ from each other
        model_unique = {}
        for model in models:
            model_data = ai_df[ai_df["model"] == model]
            other_data = ai_df[ai_df["model"] != model]
            unique_features = []
            for feature in TEXT_FEATURES[:15]:
                if feature in df.columns:
                    a = model_data[feature].dropna().values
                    b = other_data[feature].dropna().values
                    if len(a) >= 50 and len(b) >= 50:
                        d = cohens_d(a, b)
                        if abs(d) >= 0.3:
                            unique_features.append({"feature": feature, "cohens_d": float(d)})
            model_unique[model] = unique_features

        return {
            "title": "AI Model Fingerprinting: Per-Model Detection Signatures",
            "n_models": len(models),
            "models": models,
            "model_vs_human": model_vs_human,
            "detectability_ranking": detectability,
            "model_unique_signatures": model_unique,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("Do different AI models leave distinct statistical fingerprints, "
                     "and which models are hardest to detect?\n")

        lines.append(f"## Models Analyzed: {findings['n_models']}\n")
        for model in findings["models"]:
            info = findings["model_vs_human"][model]
            lines.append(f"- **{model}**: {info['n_samples']:,} samples, "
                        f"avg effect size = {info['avg_effect_size']:.3f}")

        lines.append("\n## Detectability Ranking (hardest to easiest)\n")
        for model, score in findings["detectability_ranking"]:
            difficulty = "HARD" if score < 0.3 else ("MEDIUM" if score < 0.5 else "EASY")
            lines.append(f"- **{model}**: avg |d| = {score:.3f} ({difficulty})")

        lines.append("\n## Per-Model Top Discriminators\n")
        for model, info in findings["model_vs_human"].items():
            lines.append(f"\n### {model}\n")
            for f in info["top_features"][:5]:
                lines.append(f"- {f['feature']}: d={f['cohens_d']:+.3f} "
                            f"(human={f['mean_human']:.4f}, {model}={f['mean_model']:.4f})")

        lines.append("\n## Unique Model Signatures\n")
        for model, features in findings["model_unique_signatures"].items():
            if features:
                lines.append(f"\n**{model}** distinguishes itself by:")
                for f in features[:3]:
                    lines.append(f"  - {f['feature']}: d={f['cohens_d']:+.3f}")

        lines.append("\n## Recommendations\n")
        lines.append("1. Build model-specific detection thresholds for higher accuracy\n")
        lines.append("2. The hardest-to-detect model should drive threshold calibration\n")
        lines.append("3. Unique signatures enable model identification, not just AI detection\n")

        return "\n".join(lines)
