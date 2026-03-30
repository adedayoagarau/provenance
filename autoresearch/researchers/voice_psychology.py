"""Voice & Psychology Researcher.

Deep analysis of what makes human writing psychologically distinct from
AI text: cognitive signatures, emotional patterns, personal voice.
"""

from typing import Any
import numpy as np
import pandas as pd

from . import BaseResearcher, compare_groups, cohens_d, TEXT_FEATURES


class VoicePsychologyResearcher(BaseResearcher):
    name = "voice_psychology"
    description = "Human voice psychology and cognitive signatures"

    # Features most relevant to voice/psychology
    VOICE_FEATURES = [
        "first_person_ratio", "hedge_word_ratio", "exclamation_rate",
        "question_rate", "sentence_length_std", "avg_paragraph_length",
        "discourse_marker_ratio", "passive_voice_ratio", "adverb_rate",
        "type_token_ratio", "hapax_ratio",
    ]

    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        human_df = df[df["label"] == "human"]
        ai_df = df[df["label"] == "ai"]

        # Voice feature comparison
        voice_findings = []
        for feature in self.VOICE_FEATURES:
            if feature in df.columns:
                result = compare_groups(df, feature)
                if result.get("status") == "ok":
                    voice_findings.append(result)
        voice_findings.sort(key=lambda f: abs(f.get("cohens_d", 0)), reverse=True)

        # Consistency analysis: humans are MORE variable
        consistency = {}
        for feature in ["sentence_length_std", "avg_sentence_length", "type_token_ratio"]:
            if feature in df.columns:
                human_std = human_df[feature].dropna().std()
                ai_std = ai_df[feature].dropna().std()
                consistency[feature] = {
                    "human_variance": float(human_std ** 2),
                    "ai_variance": float(ai_std ** 2),
                    "ratio": float(human_std / ai_std) if ai_std > 0 else 0,
                    "humans_more_variable": bool(human_std > ai_std),
                }

        # First person usage patterns
        first_person = {}
        if "first_person_ratio" in df.columns:
            first_person = {
                "human_mean": float(human_df["first_person_ratio"].dropna().mean()),
                "ai_mean": float(ai_df["first_person_ratio"].dropna().mean()),
                "human_uses_first_person_pct": float((human_df["first_person_ratio"] > 0).mean() * 100),
                "ai_uses_first_person_pct": float((ai_df["first_person_ratio"] > 0).mean() * 100),
            }

        # Hedging patterns
        hedging = {}
        if "hedge_word_ratio" in df.columns:
            hedging = {
                "human_mean": float(human_df["hedge_word_ratio"].dropna().mean()),
                "ai_mean": float(ai_df["hedge_word_ratio"].dropna().mean()),
            }

        # Register-specific voice differences
        register_voice = {}
        if "register" in df.columns:
            for reg in df["register"].dropna().unique()[:8]:
                reg_df = df[df["register"] == reg]
                if len(reg_df[reg_df["label"] == "human"]) >= 50:
                    if "first_person_ratio" in reg_df.columns:
                        result = compare_groups(reg_df, "first_person_ratio")
                        if result.get("status") == "ok":
                            register_voice[reg] = result["cohens_d"]

        return {
            "title": "Human Voice Psychology: What Makes Human Writing Human",
            "n_human": len(human_df),
            "n_ai": len(ai_df),
            "voice_findings": voice_findings,
            "consistency": consistency,
            "first_person": first_person,
            "hedging": hedging,
            "register_voice": register_voice,
        }

    def generate_memo(self, findings: dict[str, Any]) -> str:
        lines = []
        lines.append("## Research Question\n")
        lines.append("What psychological and cognitive markers distinguish human "
                     "writing from AI-generated text?\n")

        lines.append("## Voice Feature Analysis\n")
        lines.append(self._format_findings_table(findings["voice_findings"]))

        lines.append("\n## Key Psychological Insights\n")

        # Consistency
        lines.append("\n### Humans Are Productively Inconsistent\n")
        for feature, info in findings["consistency"].items():
            if info["humans_more_variable"]:
                lines.append(f"- **{feature}**: Human variance is {info['ratio']:.2f}x "
                            f"higher than AI (human={info['human_variance']:.4f}, "
                            f"ai={info['ai_variance']:.4f})")

        # First person
        if findings["first_person"]:
            fp = findings["first_person"]
            lines.append(f"\n### First Person Voice\n")
            lines.append(f"- Humans use first person in {fp['human_uses_first_person_pct']:.1f}% "
                        f"of texts vs {fp['ai_uses_first_person_pct']:.1f}% for AI")
            lines.append(f"- Human mean ratio: {fp['human_mean']:.4f} vs AI: {fp['ai_mean']:.4f}")

        # Hedging
        if findings["hedging"]:
            h = findings["hedging"]
            lines.append(f"\n### Hedging and Uncertainty\n")
            more = "more" if h["human_mean"] > h["ai_mean"] else "less"
            lines.append(f"- Humans hedge {more} than AI (human={h['human_mean']:.4f}, "
                        f"ai={h['ai_mean']:.4f})")

        lines.append("\n## Implications\n")
        lines.append("1. Variance-based features capture 'humanness' better than means\n")
        lines.append("2. First-person voice is a strong but register-dependent signal\n")
        lines.append("3. AI text is 'too consistent' -- this is a fundamental weakness\n")

        return "\n".join(lines)
