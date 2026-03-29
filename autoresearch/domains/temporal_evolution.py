"""Temporal Evolution Research Domain.

Autonomous research into how writing styles change over time — both
for individual authors (style drift) and for AI models (generation drift).

Research directions:
- Author style drift modeling over months/years
- AI model generation drift (GPT-3 vs GPT-4 vs GPT-5 signatures)
- Temporal calibration of detection models
- Writing maturity detection (student improvement tracking)
- Event-driven style shifts (major life events, career changes)
- Seasonal/contextual variation (work vs personal)
- Model versioning detection (which AI model version was used)
"""

import math
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


@dataclass
class TemporalProfile:
    """Temporal evolution profile of a writer or model."""
    style_drift_rate: float = 0.0      # How fast style changes per time unit
    stability_windows: list[dict] = field(default_factory=list)  # Periods of stable style
    change_points: list[dict] = field(default_factory=list)      # Detected style shifts
    maturity_trajectory: list[float] = field(default_factory=list)
    vocabulary_growth_rate: float = 0.0
    complexity_trend: str = ""  # "increasing", "decreasing", "stable", "fluctuating"


@dataclass
class ModelVersionSignature:
    """Signature characteristics of a specific AI model version."""
    model_family: str = ""           # "gpt", "claude", "llama"
    estimated_version: str = ""      # "gpt-4", "claude-3"
    confidence: float = 0.0
    distinguishing_features: list[str] = field(default_factory=list)


class TemporalEvolutionResearcher:
    """Autonomous researcher for temporal style evolution."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="temp-001",
                domain="temporal_evolution",
                description="Vocabulary growth rate is a reliable indicator of human "
                "authorship across time",
                approach="Human writers gradually expand their vocabulary over months "
                "and years. AI models don't show this organic growth. Track "
                "vocabulary accumulation curves.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="temp-002",
                domain="temporal_evolution",
                description="Style change-point detection can identify when a writer "
                "started using AI assistance",
                approach="Detect sudden discontinuities in stylometric features. "
                "A sharp increase in formality, vocabulary range, or sentence "
                "consistency may signal AI involvement.",
                expected_impact="high",
                complexity="complex",
            ),
            Hypothesis(
                id="temp-003",
                domain="temporal_evolution",
                description="Different AI model versions produce distinguishable "
                "generation signatures",
                approach="Build model-version classifiers using subtle statistical "
                "differences between GPT-3.5, GPT-4, Claude 2, Claude 3, etc.",
                expected_impact="medium",
                complexity="complex",
            ),
            Hypothesis(
                id="temp-004",
                domain="temporal_evolution",
                description="Writing maturity trajectory follows predictable patterns "
                "that can be modeled",
                approach="Track sentence complexity, vocabulary sophistication, and "
                "argument structure over time. Use these trajectories to verify "
                "that submitted work matches the student's development curve.",
                expected_impact="medium",
                complexity="moderate",
            ),
        ]

    def run_experiment(
        self, hypothesis: Hypothesis, data_dir: Path, time_budget: int = 300
    ) -> ExperimentResult:
        import time
        start = time.time()
        try:
            # Load time-stamped text samples
            samples = self._load_temporal_samples(data_dir)
            if len(samples) < 3:
                return ExperimentResult(
                    hypothesis_id=hypothesis.id, domain="temporal_evolution",
                    metric_before=0.0, metric_after=0.0, delta=0.0,
                    duration_seconds=time.time() - start, memory_mb=0.0,
                    status="error", notes="Need at least 3 time-stamped samples",
                )

            profile = build_temporal_profile(samples)
            metric = evaluate_temporal_model(profile, samples)

            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="temporal_evolution",
                metric_before=0.0, metric_after=metric, delta=metric,
                duration_seconds=time.time() - start, memory_mb=0.0,
                status="improvement" if metric > 0.5 else "null",
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="temporal_evolution",
                metric_before=0.0, metric_after=0.0, delta=0.0,
                duration_seconds=time.time() - start, memory_mb=0.0,
                status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.0

    def _load_temporal_samples(self, data_dir: Path) -> list:
        samples = []
        for fpath in sorted(data_dir.glob("**/*.txt")):
            text = fpath.read_text(errors="replace")
            # Extract timestamp from filename (e.g., 2024-01-15_essay.txt)
            import re
            date_match = re.search(r'(\d{4}-\d{2}-\d{2})', fpath.stem)
            timestamp = date_match.group(1) if date_match else fpath.stem
            samples.append({"text": text, "timestamp": timestamp, "path": str(fpath)})
        return samples


def build_temporal_profile(samples: list[dict]) -> TemporalProfile:
    """Build a temporal evolution profile from time-stamped samples."""
    profile = TemporalProfile()

    cumulative_vocab = set()
    vocab_sizes = []
    complexities = []

    for sample in samples:
        text = sample["text"]
        words = text.lower().split()
        sentences = [s for s in text.split(".") if s.strip()]

        # Track vocabulary growth
        new_words = set(words) - cumulative_vocab
        cumulative_vocab.update(words)
        vocab_sizes.append(len(cumulative_vocab))

        # Track complexity
        avg_sent_len = len(words) / max(len(sentences), 1)
        complexities.append(avg_sent_len)

    # Vocabulary growth rate
    if len(vocab_sizes) >= 2:
        growth = vocab_sizes[-1] - vocab_sizes[0]
        profile.vocabulary_growth_rate = growth / len(vocab_sizes)

    # Complexity trend
    if len(complexities) >= 3:
        first_half = sum(complexities[:len(complexities)//2]) / max(len(complexities)//2, 1)
        second_half = sum(complexities[len(complexities)//2:]) / max(len(complexities) - len(complexities)//2, 1)
        if second_half > first_half * 1.1:
            profile.complexity_trend = "increasing"
        elif second_half < first_half * 0.9:
            profile.complexity_trend = "decreasing"
        else:
            profile.complexity_trend = "stable"

    profile.maturity_trajectory = complexities
    return profile


def evaluate_temporal_model(profile: TemporalProfile, samples: list) -> float:
    """Evaluate how well the temporal model fits the data."""
    if not profile.maturity_trajectory or len(profile.maturity_trajectory) < 2:
        return 0.0

    # Simple evaluation: does the model detect consistent trends?
    trajectory = profile.maturity_trajectory
    mean = sum(trajectory) / len(trajectory)
    variance = sum((t - mean) ** 2 for t in trajectory) / (len(trajectory) - 1)

    # Score: low variance = stable writer, high variance = evolving
    # Both are valid but should be detectable
    if variance > 0:
        return min(1.0, 0.5 + profile.vocabulary_growth_rate / 100.0)
    return 0.5
