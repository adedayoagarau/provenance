"""AI Detection Research Domain.

Autonomous research into detecting AI-generated text with high accuracy
while minimizing false positives. Targets evasion-resistant detection
that works across all major language models.

Research directions:
- Statistical feature engineering (entropy, perplexity proxies, burstiness)
- Adversarial robustness (humanizers, paraphrasers, style transfer)
- Cross-model generalization (GPT, Claude, Llama, Mistral, Gemma signatures)
- False positive reduction for NNES, edited text, domain-specific jargon
- Paragraph-level mixed-document detection
- Temporal adaptation (new models produce different patterns over time)
"""

import json
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


@dataclass
class DetectionFeatureSet:
    """A configurable set of detection features and their weights."""

    # Tier 1: High effect-size features (from Provenance's detection module)
    burstiness_weight: float = 0.20
    zipf_deviation_weight: float = 0.18
    hedge_ratio_weight: float = 0.12
    autocorrelation_weight: float = 0.10
    pos_entropy_weight: float = 0.08

    # Tier 2: Medium effect-size features
    passive_clustering_weight: float = 0.05
    transition_density_weight: float = 0.05
    paragraph_cv_weight: float = 0.04
    sentence_opening_diversity_weight: float = 0.04
    nested_clause_weight: float = 0.03
    comma_splice_weight: float = 0.02

    # Tier 3: Experimental features (research targets)
    vocabulary_sophistication_weight: float = 0.03
    register_consistency_weight: float = 0.02
    modal_density_weight: float = 0.02
    enumeration_ratio_weight: float = 0.02

    # --- New experimental features for research ---

    # Entropy-based features
    token_entropy_rate: float = 0.0       # Shannon entropy of token sequences
    conditional_entropy: float = 0.0      # H(token | previous tokens)
    entropy_variance: float = 0.0         # Variance of per-sentence entropy

    # Perplexity proxy features (without needing a reference LM)
    surprisal_distribution: float = 0.0   # Skewness of word surprisal
    rare_word_placement: float = 0.0      # Where unusual words appear in sentences
    collocation_novelty: float = 0.0      # How predictable word pairs are

    # Temporal/sequential features
    information_density_curve: float = 0.0  # How info density changes over text
    topic_drift_rate: float = 0.0          # How quickly topics shift
    argument_arc_shape: float = 0.0        # Shape of the argument trajectory

    # Adversarial detection
    humanizer_artifact_score: float = 0.0  # Detect humanizer tool artifacts
    style_consistency_score: float = 0.0   # Detect style grafting
    watermark_residual: float = 0.0        # Detect LLM watermark remnants


class AIDetectionResearcher:
    """Autonomous researcher for AI detection improvements."""

    def __init__(self, provenance_bin: str, test_suite_dir: str):
        self.provenance_bin = Path(provenance_bin)
        self.test_suite_dir = Path(test_suite_dir)
        self.feature_set = DetectionFeatureSet()

    def get_hypotheses(self) -> list[Hypothesis]:
        """Generate research hypotheses ranked by expected impact."""
        return [
            Hypothesis(
                id="det-001",
                domain="ai_detection",
                description="Entropy variance as discriminator: AI text has lower "
                "per-sentence entropy variance than human text",
                approach="Compute Shannon entropy per sentence, measure variance "
                "across document. AI text should show more uniform entropy.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="det-002",
                domain="ai_detection",
                description="Information density curve shape distinguishes AI from human",
                approach="Measure information density (unique content words / total words) "
                "in sliding windows. Human text has more irregular density curves.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="det-003",
                domain="ai_detection",
                description="Rare word placement patterns differ between AI and human",
                approach="Map where in sentences rare/unusual words appear. Humans "
                "place rare words more freely; AI tends toward safer positions.",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="det-004",
                domain="ai_detection",
                description="Collocation novelty: humans create more unexpected word pairs",
                approach="Score bigram predictability against corpus statistics. "
                "Human text should have higher collocation novelty.",
                expected_impact="medium",
                complexity="moderate",
            ),
            Hypothesis(
                id="det-005",
                domain="ai_detection",
                description="Humanizer artifacts leave detectable statistical fingerprints",
                approach="Analyze texts processed by humanizer tools for characteristic "
                "patterns: over-synonymization, unnatural synonym placement, "
                "broken collocations.",
                expected_impact="high",
                complexity="complex",
            ),
            Hypothesis(
                id="det-006",
                domain="ai_detection",
                description="Cross-model ensemble: different LLMs leave distinct signatures",
                approach="Train separate feature extractors for GPT, Claude, Llama, "
                "Mistral outputs. Ensemble their signals for model-agnostic detection.",
                expected_impact="high",
                complexity="complex",
            ),
            Hypothesis(
                id="det-007",
                domain="ai_detection",
                description="Paragraph-level detection for mixed human+AI documents",
                approach="Apply sliding window detection with paragraph boundaries. "
                "Score transitions between human and AI sections.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="det-008",
                domain="ai_detection",
                description="Conditional entropy (bigram predictability) as a feature",
                approach="Compute H(word_n | word_{n-1}) across document. AI text "
                "should have lower conditional entropy (more predictable).",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="det-009",
                domain="ai_detection",
                description="Topic drift rate: AI maintains topics more consistently",
                approach="Measure cosine distance between consecutive paragraph "
                "topic vectors. Human text drifts more organically.",
                expected_impact="medium",
                complexity="moderate",
            ),
            Hypothesis(
                id="det-010",
                domain="ai_detection",
                description="Watermark residual detection for steganographic LLM watermarks",
                approach="Detect statistical patterns left by LLM watermarking "
                "schemes (token distribution biases in specific positions).",
                expected_impact="medium",
                complexity="complex",
            ),
        ]

    def run_experiment(
        self, hypothesis: Hypothesis, data_dir: Path, time_budget: int = 300
    ) -> ExperimentResult:
        """Run a single experiment for the given hypothesis."""
        start_time = time.time()
        metric_before = self.get_current_baseline(data_dir)

        try:
            # Run the Provenance detection on test documents
            results = self._run_detection_suite(data_dir, time_budget)
            metric_after = self._compute_f1(results)
            delta = metric_after - metric_before
            duration = time.time() - start_time

            status = "improvement" if delta > 0.001 else (
                "regression" if delta < -0.001 else "null"
            )

            return ExperimentResult(
                hypothesis_id=hypothesis.id,
                domain="ai_detection",
                metric_before=metric_before,
                metric_after=metric_after,
                delta=delta,
                duration_seconds=duration,
                memory_mb=self._get_memory_usage(),
                status=status,
                details={
                    "f1": metric_after,
                    "fpr": results.get("fpr", -1),
                    "tpr": results.get("tpr", -1),
                    "total_samples": results.get("total", 0),
                },
                notes=f"Tested: {hypothesis.description[:80]}",
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id,
                domain="ai_detection",
                metric_before=metric_before,
                metric_after=metric_before,
                delta=0.0,
                duration_seconds=time.time() - start_time,
                memory_mb=self._get_memory_usage(),
                status="error",
                notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        """Compute the current F1 score on the test set."""
        results = self._run_detection_suite(data_dir, time_budget=120)
        return self._compute_f1(results)

    def _run_detection_suite(self, data_dir: Path, time_budget: int) -> dict:
        """Run Provenance detection on all test documents."""
        results = {"tp": 0, "fp": 0, "tn": 0, "fn": 0, "total": 0, "errors": 0}

        test_files = list(data_dir.glob("**/*.txt")) + list(data_dir.glob("**/*.md"))
        deadline = time.time() + time_budget

        for fpath in test_files:
            if time.time() > deadline:
                break

            # Determine ground truth from directory structure or filename
            is_ai = self._get_ground_truth(fpath)
            if is_ai is None:
                continue

            try:
                output = subprocess.run(
                    [str(self.provenance_bin), "detect", "--file", str(fpath),
                     "--format", "json"],
                    capture_output=True, text=True, timeout=30,
                )
                if output.returncode == 0:
                    detection = json.loads(output.stdout)
                    predicted_ai = detection.get("score", {}).get("adjusted_score", 0.5) > 0.5

                    if is_ai and predicted_ai:
                        results["tp"] += 1
                    elif is_ai and not predicted_ai:
                        results["fn"] += 1
                    elif not is_ai and predicted_ai:
                        results["fp"] += 1
                    else:
                        results["tn"] += 1
                    results["total"] += 1
                else:
                    results["errors"] += 1
            except (subprocess.TimeoutExpired, json.JSONDecodeError):
                results["errors"] += 1

        # Compute rates
        tp, fp, tn, fn = results["tp"], results["fp"], results["tn"], results["fn"]
        results["tpr"] = tp / max(tp + fn, 1)
        results["fpr"] = fp / max(fp + tn, 1)
        return results

    def _compute_f1(self, results: dict) -> float:
        """Compute F1 score from confusion matrix."""
        tp, fp, fn = results["tp"], results["fp"], results["fn"]
        precision = tp / max(tp + fp, 1)
        recall = tp / max(tp + fn, 1)
        if precision + recall == 0:
            return 0.0
        return 2 * (precision * recall) / (precision + recall)

    def _get_ground_truth(self, fpath: Path) -> Optional[bool]:
        """Determine if a file is AI-generated based on path conventions."""
        path_str = str(fpath).lower()
        if "/ai/" in path_str or "_ai_" in path_str or path_str.endswith("_ai.txt"):
            return True
        if "/human/" in path_str or "_human_" in path_str:
            return False
        # Check for metadata sidecar
        meta_path = fpath.with_suffix(".meta.json")
        if meta_path.exists():
            try:
                meta = json.loads(meta_path.read_text())
                return meta.get("label") == "ai"
            except (json.JSONDecodeError, KeyError):
                pass
        return None

    def _get_memory_usage(self) -> float:
        """Get current process memory in MB."""
        try:
            import resource
            usage = resource.getrusage(resource.RUSAGE_SELF)
            return usage.ru_maxrss / 1024  # Convert KB to MB on Linux
        except Exception:
            return 0.0


# ─── Feature Research Functions ──────────────────────────────────────────────
# These are the functions the agent modifies during experiments.

def compute_entropy_variance(sentences: list[str]) -> float:
    """Compute variance of per-sentence Shannon entropy.

    AI text tends to have more uniform entropy across sentences.
    Human text shows higher variance — some sentences are information-dense,
    others are connective tissue.
    """
    import math
    from collections import Counter

    entropies = []
    for sentence in sentences:
        words = sentence.lower().split()
        if len(words) < 3:
            continue
        counts = Counter(words)
        total = len(words)
        entropy = -sum(
            (c / total) * math.log2(c / total)
            for c in counts.values()
        )
        entropies.append(entropy)

    if len(entropies) < 2:
        return 0.0

    mean_h = sum(entropies) / len(entropies)
    variance = sum((h - mean_h) ** 2 for h in entropies) / (len(entropies) - 1)
    return variance


def compute_information_density_curve(text: str, window_size: int = 50) -> list[float]:
    """Compute information density in sliding windows.

    Returns a list of density values. The shape of this curve
    differs between human and AI text.
    """
    words = text.split()
    if len(words) < window_size:
        return [len(set(words)) / max(len(words), 1)]

    densities = []
    for i in range(0, len(words) - window_size + 1, window_size // 2):
        window = words[i:i + window_size]
        unique = len(set(w.lower() for w in window))
        densities.append(unique / window_size)

    return densities


def compute_rare_word_position_score(sentences: list[str], vocab_freqs: dict) -> float:
    """Score where rare words appear in sentences.

    Humans place rare words more freely throughout sentences.
    AI tends to cluster them in predictable positions (often mid-sentence
    or as the subject).
    """
    if not vocab_freqs:
        return 0.0

    median_freq = sorted(vocab_freqs.values())[len(vocab_freqs) // 2]
    position_scores = []

    for sentence in sentences:
        words = sentence.split()
        if len(words) < 5:
            continue
        for i, word in enumerate(words):
            freq = vocab_freqs.get(word.lower(), 0)
            if freq < median_freq * 0.1:  # Rare word
                normalized_pos = i / len(words)
                position_scores.append(normalized_pos)

    if not position_scores:
        return 0.5  # Neutral

    # High variance = human-like (rare words spread across positions)
    mean_pos = sum(position_scores) / len(position_scores)
    if len(position_scores) < 2:
        return mean_pos
    variance = sum((p - mean_pos) ** 2 for p in position_scores) / (len(position_scores) - 1)
    return variance


def compute_conditional_entropy(words: list[str]) -> float:
    """Compute bigram conditional entropy H(w_n | w_{n-1}).

    Lower conditional entropy = more predictable = more AI-like.
    """
    import math
    from collections import Counter

    if len(words) < 10:
        return 0.0

    bigrams = [(words[i].lower(), words[i + 1].lower()) for i in range(len(words) - 1)]
    unigram_counts = Counter(w.lower() for w in words)
    bigram_counts = Counter(bigrams)
    total_bigrams = len(bigrams)

    entropy = 0.0
    for (w1, w2), count in bigram_counts.items():
        p_bigram = count / total_bigrams
        p_conditional = count / unigram_counts[w1]
        if p_conditional > 0:
            entropy -= p_bigram * math.log2(p_conditional)

    return entropy
