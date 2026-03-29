"""Cross-Linguistic & Cultural Research Domain.

Autonomous research into handling multilingual text, NNES (Non-Native
English Speakers), code-switching, and cultural writing patterns.

Research directions:
- NNES false positive reduction
- L1 (first language) interference pattern detection
- Code-switching detection and handling
- Cultural writing convention awareness
- Multilingual authorship verification
- Translation artifact detection
- Register norms across cultures
"""

import re
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

from . import Hypothesis, ExperimentResult


@dataclass
class LinguisticProfile:
    """Cross-linguistic analysis of a text."""
    nnes_probability: float = 0.0
    estimated_l1: str = ""               # Estimated first language
    l1_confidence: float = 0.0
    code_switching_detected: bool = False
    code_switching_languages: list[str] = field(default_factory=list)
    cultural_markers: list[str] = field(default_factory=list)
    translation_probability: float = 0.0


# ─── L1 Interference Patterns ───────────────────────────────────────────────
# These are characteristic errors/patterns produced by speakers of different L1s

L1_PATTERNS = {
    "mandarin": {
        "article_errors": [r'\b(a|an|the)\s+(a|an|the)\b', r'(?<!\b(?:a|an|the)\s)\b\w+\s+(?:is|are|was)\b'],
        "aspect_markers": [r'\balready\s+\w+ed\b'],
        "topic_comment": [r'^[A-Z]\w+,\s+(?:it|they|he|she)\b'],
    },
    "spanish": {
        "adjective_order": [r'\b\w+\s+(big|small|red|blue|old|new)\b'],  # Post-nominal adjectives
        "ser_estar": [r'\bis\s+being\s+\w+\b'],
        "double_negation": [r'\bnot\s+\w+\s+no\b'],
    },
    "arabic": {
        "coordination_heavy": [r'\band\s+\w+\s+and\s+\w+\s+and\b'],
        "pronoun_resumption": [r'\bwhich\s+\w+\s+it\b', r'\bthat\s+\w+\s+they\b'],
    },
    "japanese": {
        "passive_overuse": [],  # Detected statistically, not with regex
        "topic_particle": [r'^(?:As\s+for|Regarding|About)\s+\w+,'],
        "hedge_overuse": [r'\b(?:maybe|perhaps|I\s+think)\b.*\b(?:maybe|perhaps|I\s+think)\b'],
    },
    "french": {
        "false_friends": [r'\b(actually|eventually|sympathetic|resume)\b'],
        "preposition_errors": [r'\b(depend|interested|consist)\s+(?!of|in|on)\w+\b'],
    },
    "hindi": {
        "progressive_overuse": [r'\bis\s+\w+ing\s+(?:since|from)\b'],
        "preposition_confusion": [r'\b(in|on|at)\s+(?:the\s+)?(?:morning|evening|night)\b'],
    },
}


class CrossLinguisticResearcher:
    """Autonomous researcher for cross-linguistic analysis."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="ling-001",
                domain="cross_linguistic",
                description="L1 interference patterns can identify a writer's native "
                "language with high accuracy",
                approach="Build L1-specific pattern detectors for the top 10 world "
                "languages. Use characteristic grammar patterns, article usage, "
                "and preposition errors as signals.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="ling-002",
                domain="cross_linguistic",
                description="NNES writers show consistent article usage patterns that "
                "are personal (not just L1-based)",
                approach="Within an L1 group, individual NNES writers develop "
                "idiosyncratic article error patterns. These can be used as "
                "identity features rather than just noise.",
                expected_impact="medium",
                complexity="moderate",
            ),
            Hypothesis(
                id="ling-003",
                domain="cross_linguistic",
                description="AI detection calibration per L1 group reduces false "
                "positives for NNES writers",
                approach="Build L1-specific detection thresholds. NNES writing "
                "triggers some AI detection features (low burstiness, unusual "
                "word choices) that need group-specific baselines.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="ling-004",
                domain="cross_linguistic",
                description="Code-switching patterns are an identity signal",
                approach="Writers who switch between languages do so in personal, "
                "habitual ways. Detect and characterize code-switching as an "
                "authorship feature.",
                expected_impact="medium",
                complexity="complex",
            ),
        ]

    def run_experiment(
        self, hypothesis: Hypothesis, data_dir: Path, time_budget: int = 300
    ) -> ExperimentResult:
        import time
        start = time.time()
        try:
            texts = self._load_texts(data_dir)
            correct = 0
            total = 0
            for text, expected_l1 in texts:
                profile = analyze_cross_linguistic(text)
                if profile.estimated_l1 == expected_l1:
                    correct += 1
                total += 1
            metric = correct / max(total, 1)
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="cross_linguistic",
                metric_before=0.0, metric_after=metric, delta=metric,
                duration_seconds=time.time() - start, memory_mb=0.0,
                status="improvement" if metric > 0.5 else "null",
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="cross_linguistic",
                metric_before=0.0, metric_after=0.0, delta=0.0,
                duration_seconds=time.time() - start, memory_mb=0.0,
                status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.0

    def _load_texts(self, data_dir: Path) -> list:
        texts = []
        for fpath in data_dir.glob("**/*.txt"):
            text = fpath.read_text(errors="replace")
            # Infer L1 from directory name
            l1 = fpath.parent.name.lower()
            texts.append((text, l1))
        return texts


def analyze_cross_linguistic(text: str) -> LinguisticProfile:
    """Analyze text for cross-linguistic features."""
    profile = LinguisticProfile()
    text_lower = text.lower()

    # Check L1 interference patterns
    l1_scores = {}
    for l1, categories in L1_PATTERNS.items():
        score = 0
        for category, patterns in categories.items():
            for pattern in patterns:
                matches = re.findall(pattern, text_lower)
                score += len(matches)
        l1_scores[l1] = score

    if l1_scores:
        best_l1 = max(l1_scores, key=l1_scores.get)
        if l1_scores[best_l1] > 2:
            profile.estimated_l1 = best_l1
            total = sum(l1_scores.values()) or 1
            profile.l1_confidence = l1_scores[best_l1] / total
            profile.nnes_probability = min(l1_scores[best_l1] / 10.0, 1.0)

    return profile
