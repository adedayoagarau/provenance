"""Adversarial Robustness Research Domain.

Autonomous research into making Provenance resistant to deliberate
evasion attempts. This is the arms race domain — attackers will try
to fool detection, and we must stay ahead.

Research directions:
- Humanizer tool detection and resistance
- Paraphrasing attack robustness
- Style transfer attack detection
- Synonym substitution pattern detection
- Back-translation artifact detection
- Prompt injection for AI text (making AI sound human)
- Robustness under text editing (Grammarly, ProWritingAid)
- Adversarial feature selection (features that survive attacks)
"""

import re
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


@dataclass
class AdversarialAnalysis:
    """Analysis of adversarial manipulation signals in text."""
    humanizer_probability: float = 0.0
    paraphrase_probability: float = 0.0
    style_transfer_probability: float = 0.0
    back_translation_probability: float = 0.0
    editing_tool_probability: float = 0.0
    manipulation_type: str = "none"      # Most likely manipulation
    confidence: float = 0.0
    artifact_locations: list[dict] = field(default_factory=list)


# ─── Humanizer Artifact Markers ──────────────────────────────────────────────

# Humanizer tools tend to produce these patterns:
HUMANIZER_ARTIFACTS = {
    # Over-synonymization: unusual synonym choices
    "synonym_oddities": [
        r'\b(utilize|employ|leverage)\b.*\b(use|utilize|employ)\b',  # Synonym cycling
    ],
    # Broken collocations: words that don't naturally go together
    "collocation_breaks": [
        r'\b(make|do)\s+(a\s+)?(big|large|huge)\s+(impact|difference|effort)\b',
    ],
    # Unnatural hedging insertion
    "forced_hedging": [
        r'\b(sort\s+of|kind\s+of|in\s+a\s+way|to\s+some\s+extent)\b',
    ],
    # Filler insertion (trying to sound more human)
    "filler_insertion": [
        r'\b(honestly|frankly|basically|essentially|actually|literally)\b',
    ],
}

# Back-translation artifacts
BACK_TRANSLATION_MARKERS = [
    r'\b(it\s+is\s+that|there\s+is\s+a\s+fact)\b',  # Awkward existential constructions
    r'\b(the\s+\w+\s+of\s+the\s+\w+\s+of)\b',       # Excessive prepositional chains
]

# Style transfer artifacts
STYLE_TRANSFER_MARKERS = [
    # Inconsistent register within a sentence
    r'\b(furthermore|moreover)\b.*\b(gonna|wanna|kinda)\b',
    r'\b(LOL|lmao|tbh)\b.*\b(notwithstanding|aforementioned)\b',
]


class AdversarialRobustnessResearcher:
    """Autonomous researcher for adversarial robustness."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="adv-001",
                domain="adversarial_robustness",
                description="Humanizer tools produce detectable collocation breaks",
                approach="When tools synonymize text, they break natural word "
                "co-occurrences. Measure pointwise mutual information (PMI) of "
                "word pairs and flag anomalous low-PMI bigrams.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="adv-002",
                domain="adversarial_robustness",
                description="Back-translation creates characteristic syntactic patterns",
                approach="Round-trip translation produces unnatural existential "
                "constructions, excessive determiners, and inverted word order. "
                "These are detectable with simple pattern matching.",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="adv-003",
                domain="adversarial_robustness",
                description="Style transfer attacks leave register inconsistency signals",
                approach="Detect within-document register violations: formal and "
                "informal language mixing in ways that real humans don't produce.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="adv-004",
                domain="adversarial_robustness",
                description="Features based on deep character patterns survive "
                "paraphrasing attacks better than word-level features",
                approach="Test character 4-gram and 5-gram features against "
                "paraphrased text. These capture morphological habits that "
                "paraphrasers don't touch.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="adv-005",
                domain="adversarial_robustness",
                description="Ensemble of attack-specific detectors outperforms "
                "single general-purpose detector",
                approach="Train separate detectors for each attack type and combine "
                "them. Each specialist catches what the generalist misses.",
                expected_impact="high",
                complexity="complex",
            ),
        ]

    def run_experiment(
        self, hypothesis: Hypothesis, data_dir: Path, time_budget: int = 300
    ) -> ExperimentResult:
        import time
        start = time.time()
        metric_before = self.get_current_baseline(data_dir)

        try:
            texts = self._load_adversarial_texts(data_dir)
            correct = 0
            total = 0

            for text, manipulation_type in texts:
                analysis = analyze_adversarial(text)
                if analysis.manipulation_type == manipulation_type:
                    correct += 1
                elif manipulation_type == "none" and analysis.confidence < 0.3:
                    correct += 1  # Correctly identified as not manipulated
                total += 1

            metric_after = correct / max(total, 1)
            delta = metric_after - metric_before

            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="adversarial_robustness",
                metric_before=metric_before, metric_after=metric_after,
                delta=delta, duration_seconds=time.time() - start,
                memory_mb=0.0,
                status="improvement" if delta > 0.001 else (
                    "regression" if delta < -0.001 else "null"
                ),
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="adversarial_robustness",
                metric_before=metric_before, metric_after=metric_before,
                delta=0.0, duration_seconds=time.time() - start,
                memory_mb=0.0, status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.0

    def _load_adversarial_texts(self, data_dir: Path) -> list:
        texts = []
        for fpath in data_dir.glob("**/*.txt"):
            text = fpath.read_text(errors="replace")
            # Determine manipulation type from path
            path_str = str(fpath).lower()
            if "humanized" in path_str:
                texts.append((text, "humanized"))
            elif "paraphrase" in path_str:
                texts.append((text, "paraphrased"))
            elif "transfer" in path_str:
                texts.append((text, "style_transfer"))
            elif "backtranslat" in path_str:
                texts.append((text, "back_translation"))
            else:
                texts.append((text, "none"))
        return texts


# ─── Analysis Functions ──────────────────────────────────────────────────────

def analyze_adversarial(text: str) -> AdversarialAnalysis:
    """Detect adversarial manipulation in text."""
    analysis = AdversarialAnalysis()
    text_lower = text.lower()

    # Check humanizer artifacts
    humanizer_score = 0
    for category, patterns in HUMANIZER_ARTIFACTS.items():
        for pattern in patterns:
            matches = re.findall(pattern, text_lower)
            humanizer_score += len(matches)
    analysis.humanizer_probability = min(humanizer_score / 10.0, 1.0)

    # Check back-translation artifacts
    bt_score = 0
    for pattern in BACK_TRANSLATION_MARKERS:
        bt_score += len(re.findall(pattern, text_lower))
    analysis.back_translation_probability = min(bt_score / 5.0, 1.0)

    # Check style transfer artifacts
    st_score = 0
    for pattern in STYLE_TRANSFER_MARKERS:
        st_score += len(re.findall(pattern, text_lower))
    analysis.style_transfer_probability = min(st_score / 3.0, 1.0)

    # Check collocation quality (PMI proxy)
    analysis.paraphrase_probability = compute_collocation_anomaly(text)

    # Determine most likely manipulation
    scores = {
        "humanized": analysis.humanizer_probability,
        "paraphrased": analysis.paraphrase_probability,
        "style_transfer": analysis.style_transfer_probability,
        "back_translation": analysis.back_translation_probability,
    }
    best = max(scores, key=scores.get)
    if scores[best] > 0.3:
        analysis.manipulation_type = best
        analysis.confidence = scores[best]
    else:
        analysis.manipulation_type = "none"
        analysis.confidence = 1.0 - max(scores.values())

    return analysis


def compute_collocation_anomaly(text: str) -> float:
    """Detect unnatural word co-occurrences (proxy for PMI anomaly).

    Paraphrased text often has unexpected word pairs because synonyms
    don't carry the same collocational patterns.
    """
    words = text.lower().split()
    if len(words) < 20:
        return 0.0

    # Common collocations that should appear together
    COMMON_COLLOCATIONS = {
        ("make", "decision"), ("take", "action"), ("pay", "attention"),
        ("come", "conclusion"), ("draw", "attention"), ("raise", "question"),
        ("play", "role"), ("run", "risk"), ("set", "example"),
    }

    # Check for broken collocations (words that should co-occur but don't)
    bigrams = [(words[i], words[i + 1]) for i in range(len(words) - 1)]
    bigram_set = set(bigrams)

    # Look for unusual substitutions
    anomaly_score = 0
    for w1, w2 in COMMON_COLLOCATIONS:
        # If we see a synonym of w1 followed by w2, it might be paraphrased
        # This is a simplified heuristic
        pass

    # Simpler: measure how many bigrams are repeated (natural text has more repetition)
    bigram_counts = Counter(bigrams)
    unique_ratio = len(bigram_counts) / max(len(bigrams), 1)
    # Very high unique ratio suggests unnatural text (no natural repetition)
    return max(0, (unique_ratio - 0.85) * 5)
