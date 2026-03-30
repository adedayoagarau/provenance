"""Frontier Ideas Researcher.

Forward-thinking analysis: novel approaches the field is missing,
theoretical limits, and unconventional detection strategies.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import BaseResearcher, compare_groups, cohens_d, TEXT_FEATURES


class FrontierIdeasResearcher(BaseResearcher):
    name = "frontier_ideas"
    description = "Novel research ideas and frontier approaches"

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        human_df = df[df["label"] == "human"]
        ai_df = df[df["label"] == "ai"]

        # Feature interaction effects
        interactions = []
        numeric_features = [f for f in TEXT_FEATURES[:10] if f in df.columns]
        for i, f1 in enumerate(numeric_features):
            for f2 in numeric_features[i+1:]:
                # Create interaction feature
                vals1 = df[f1].fillna(0)
                vals2 = df[f2].fillna(0)
                interaction = vals1 * vals2
                df_temp = df.copy()
                interaction_name = f"{f1}_x_{f2}"
                df_temp[interaction_name] = interaction
                result = compare_groups(df_temp, interaction_name)
                if result.get("status") == "ok" and abs(result["cohens_d"]) > 0.3:
                    interactions.append({
                        "features": (f1, f2),
                        "name": interaction_name,
                        "cohens_d": result["cohens_d"],
                    })
        interactions.sort(key=lambda x: abs(x["cohens_d"]), reverse=True)

        # Ratio features (novel combinations)
        ratios = []
        for f1 in numeric_features[:8]:
            for f2 in numeric_features[:8]:
                if f1 != f2:
                    v1 = df[f1].fillna(0)
                    v2 = df[f2].replace(0, np.nan).fillna(1)
                    ratio = v1 / v2
                    ratio_name = f"{f1}_over_{f2}"
                    df_temp = df.copy()
                    df_temp[ratio_name] = ratio
                    result = compare_groups(df_temp, ratio_name)
                    if result.get("status") == "ok" and abs(result["cohens_d"]) > 0.4:
                        ratios.append({
                            "name": ratio_name,
                            "cohens_d": result["cohens_d"],
                        })
        ratios.sort(key=lambda x: abs(x["cohens_d"]), reverse=True)

        # Consistency-based features (variance of features as meta-features)
        consistency_features = []
        if "register" in df.columns:
            for feature in numeric_features[:8]:
                human_by_reg = human_df.groupby("register")[feature].std()
                ai_by_reg = ai_df.groupby("register")[feature].std()
                if len(human_by_reg) >= 3 and len(ai_by_reg) >= 3:
                    consistency_features.append({
                        "feature": feature,
                        "human_cross_register_std": float(human_by_reg.mean()),
                        "ai_cross_register_std": float(ai_by_reg.mean()),
                    })

        # Ensemble diversity: which features are uncorrelated?
        feature_correlations = {}
        avail = [f for f in numeric_features[:10] if f in df.columns]
        if len(avail) >= 2:
            corr_matrix = df[avail].corr().abs()
            uncorrelated_pairs = []
            for i, f1 in enumerate(avail):
                for f2 in avail[i+1:]:
                    corr = corr_matrix.loc[f1, f2]
                    if corr < 0.3:
                        uncorrelated_pairs.append((f1, f2, float(corr)))
            feature_correlations = {
                "uncorrelated_pairs": uncorrelated_pairs[:10],
                "n_uncorrelated": len(uncorrelated_pairs),
            }

        return {
            "title": "Frontier Research: Novel Detection Approaches",
            "interaction_effects": interactions[:10],
            "ratio_features": ratios[:10],
            "consistency_features": consistency_features,
            "feature_correlations": feature_correlations,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("What novel feature combinations and unconventional approaches "
                     "can improve AI detection beyond standard methods?\n")

        lines.append("## Novel Feature Interactions\n")
        lines.append("Multiplicative interactions between features that are more "
                     "discriminative than either feature alone:\n")
        for inter in findings["interaction_effects"][:5]:
            lines.append(f"- **{inter['features'][0]} x {inter['features'][1]}**: "
                        f"d={inter['cohens_d']:+.3f}")

        lines.append("\n## Novel Ratio Features\n")
        for ratio in findings["ratio_features"][:5]:
            lines.append(f"- **{ratio['name']}**: d={ratio['cohens_d']:+.3f}")

        lines.append("\n## Ensemble Diversity\n")
        if findings["feature_correlations"]:
            n = findings["feature_correlations"]["n_uncorrelated"]
            lines.append(f"Found {n} uncorrelated feature pairs (r < 0.3) -- "
                        "ideal for ensemble diversity:\n")
            for f1, f2, corr in findings["feature_correlations"]["uncorrelated_pairs"][:5]:
                lines.append(f"- {f1} + {f2} (r={corr:.3f})")

        lines.append("\n## Forward-Looking Ideas\n")
        lines.append("1. **Feature interactions** unlock discriminative power "
                    "invisible to individual features\n")
        lines.append("2. **Ratio features** capture relational patterns in text structure\n")
        lines.append("3. **Consistency across registers** is a meta-feature: "
                    "humans adapt more, AI stays similar\n")
        lines.append("4. **Uncorrelated feature ensembles** maximize information gain\n")
        lines.append("5. **Theoretical limit**: as AI improves, detection must move from "
                    "surface statistics to deeper cognitive/structural patterns\n")

        return "\n".join(lines)
