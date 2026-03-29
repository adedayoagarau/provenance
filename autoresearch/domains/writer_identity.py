"""Writer Identity Research Domain.

Autonomous research into building rich identity models that capture how
a person *thinks*, not just how they *type*. The goal is to reduce Equal
Error Rate (EER) in authorship verification tasks.

Research directions:
- Deep stylometric embeddings (character-level, sub-word patterns)
- Adversarial robustness against style transfer attacks
- Temporal identity modeling (how writers evolve over years)
- Multi-document identity verification (consistency across samples)
- Cross-register identity (same person writing formally vs informally)
- Minimal-text identification (verify with very short documents)
- Identity confidence calibration
"""

import math
import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


@dataclass
class IdentityEmbedding:
    """Multi-dimensional identity representation."""
    # Lexical identity
    vocabulary_fingerprint: list[float] = field(default_factory=list)
    word_frequency_signature: list[float] = field(default_factory=list)

    # Syntactic identity
    sentence_structure_pattern: list[float] = field(default_factory=list)
    clause_complexity_signature: list[float] = field(default_factory=list)

    # Character-level identity
    char_ngram_signature: list[float] = field(default_factory=list)
    punctuation_pattern: list[float] = field(default_factory=list)

    # Semantic identity
    topic_preference_vector: list[float] = field(default_factory=list)
    abstraction_profile: list[float] = field(default_factory=list)

    # Cognitive identity
    reasoning_style_vector: list[float] = field(default_factory=list)
    certainty_profile: list[float] = field(default_factory=list)


@dataclass
class VerificationResult:
    """Result of an authorship verification comparison."""
    score: float           # Similarity score [0, 1]
    same_author: bool      # Decision
    confidence: float      # How confident we are in the decision
    contributing_features: list[dict] = field(default_factory=list)


class WriterIdentityResearcher:
    """Autonomous researcher for writer identity modeling."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="id-001",
                domain="writer_identity",
                description="Character trigram frequency profiles are more robust "
                "identity features than word-level features",
                approach="Extract top-K character trigrams per author. These capture "
                "spelling habits, morphological preferences, and typing patterns "
                "that are harder to consciously modify.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="id-002",
                domain="writer_identity",
                description="Function word ratios are the single most discriminative "
                "feature set for authorship verification",
                approach="Compute ratios between pairs of function words (the/a, "
                "but/however, this/that). These ratios are deeply habitual and "
                "largely unconscious.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="id-003",
                domain="writer_identity",
                description="Cross-register normalization improves identity matching "
                "when author writes in different styles",
                approach="Normalize features by register-specific baselines before "
                "comparison. This removes register effects and isolates the "
                "author's personal signal.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="id-004",
                domain="writer_identity",
                description="Sentence-initial word distributions are a strong identity "
                "signal that resists conscious modification",
                approach="Build probability distribution over sentence-starting words. "
                "This captures habitual thought-organization patterns.",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="id-005",
                domain="writer_identity",
                description="Minimal-text identification: reliable verification from "
                "as few as 200 words using character-level features",
                approach="Focus on character n-grams and punctuation patterns which "
                "remain discriminative even in very short texts.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="id-006",
                domain="writer_identity",
                description="Temporal identity drift: model how an author's style "
                "evolves and account for it in verification",
                approach="Track feature trajectories over time-stamped documents. "
                "Use trend-aware comparison that expects gradual drift.",
                expected_impact="medium",
                complexity="complex",
            ),
            Hypothesis(
                id="id-007",
                domain="writer_identity",
                description="Adversarial robustness: identity features that survive "
                "deliberate style obfuscation",
                approach="Test which features remain discriminative after adversarial "
                "paraphrasing, synonym substitution, and sentence restructuring.",
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
            authors = self._load_author_samples(data_dir)
            if len(authors) < 2:
                return ExperimentResult(
                    hypothesis_id=hypothesis.id, domain="writer_identity",
                    metric_before=metric_before, metric_after=metric_before,
                    delta=0.0, duration_seconds=time.time() - start,
                    memory_mb=0.0, status="error",
                    notes="Need at least 2 authors",
                )

            eer = self._compute_eer(authors)
            delta = metric_before - eer  # Lower EER is better

            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="writer_identity",
                metric_before=metric_before, metric_after=eer,
                delta=delta, duration_seconds=time.time() - start,
                memory_mb=0.0,
                status="improvement" if delta > 0.001 else (
                    "regression" if delta < -0.001 else "null"
                ),
                details={"eer": eer, "num_authors": len(authors)},
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="writer_identity",
                metric_before=metric_before, metric_after=metric_before,
                delta=0.0, duration_seconds=time.time() - start,
                memory_mb=0.0, status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.20  # Start with 20% EER baseline

    def _load_author_samples(self, data_dir: Path) -> dict:
        authors = defaultdict(list)
        for author_dir in data_dir.iterdir():
            if author_dir.is_dir():
                for fpath in author_dir.glob("*.txt"):
                    text = fpath.read_text(errors="replace")
                    if len(text.split()) >= 200:
                        authors[author_dir.name].append(text)
        return dict(authors)

    def _compute_eer(self, authors: dict) -> float:
        """Compute Equal Error Rate from verification pairs."""
        genuine_scores = []
        impostor_scores = []

        author_names = list(authors.keys())
        for i, name_a in enumerate(author_names):
            texts_a = authors[name_a]
            if len(texts_a) < 2:
                continue

            # Genuine pairs (same author)
            for j in range(len(texts_a)):
                for k in range(j + 1, len(texts_a)):
                    emb_j = build_identity_embedding(texts_a[j])
                    emb_k = build_identity_embedding(texts_a[k])
                    score = compare_identities(emb_j, emb_k)
                    genuine_scores.append(score)

            # Impostor pairs (different authors)
            for name_b in author_names[i + 1:]:
                texts_b = authors[name_b]
                if texts_b:
                    emb_a = build_identity_embedding(texts_a[0])
                    emb_b = build_identity_embedding(texts_b[0])
                    score = compare_identities(emb_a, emb_b)
                    impostor_scores.append(score)

        if not genuine_scores or not impostor_scores:
            return 0.5

        # Find EER: threshold where FAR = FRR
        thresholds = sorted(set(genuine_scores + impostor_scores))
        best_eer = 1.0
        for threshold in thresholds:
            frr = sum(1 for s in genuine_scores if s < threshold) / len(genuine_scores)
            far = sum(1 for s in impostor_scores if s >= threshold) / len(impostor_scores)
            eer = (frr + far) / 2
            if abs(frr - far) < abs(best_eer - 0.5) * 2 + 0.01:
                best_eer = eer

        return best_eer


# ─── Identity Feature Functions ──────────────────────────────────────────────

def build_identity_embedding(text: str) -> IdentityEmbedding:
    """Build a multi-dimensional identity embedding from text."""
    embedding = IdentityEmbedding()
    words = text.split()
    words_lower = [w.lower() for w in words]

    # Character trigram signature
    embedding.char_ngram_signature = compute_char_ngram_profile(text)

    # Function word ratios
    embedding.word_frequency_signature = compute_function_word_ratios(words_lower)

    # Sentence structure pattern
    sentences = re.split(r'(?<=[.!?])\s+', text)
    embedding.sentence_structure_pattern = compute_sentence_structure(sentences)

    # Punctuation pattern
    embedding.punctuation_pattern = compute_punctuation_profile(text)

    # Vocabulary fingerprint
    embedding.vocabulary_fingerprint = compute_vocab_fingerprint(words_lower)

    # Certainty profile
    embedding.certainty_profile = compute_certainty_vector(sentences)

    return embedding


def compute_char_ngram_profile(text: str, n: int = 3, top_k: int = 50) -> list[float]:
    """Extract top-K character n-gram frequency profile."""
    text_clean = re.sub(r'\s+', ' ', text.lower())
    ngrams = Counter()
    for i in range(len(text_clean) - n + 1):
        ngrams[text_clean[i:i + n]] += 1

    total = sum(ngrams.values()) or 1
    top = ngrams.most_common(top_k)
    return [count / total for _, count in top]


def compute_function_word_ratios(words: list[str]) -> list[float]:
    """Compute ratios between pairs of function words."""
    FUNCTION_WORDS = [
        "the", "a", "an", "this", "that", "these", "those",
        "in", "on", "at", "by", "for", "with", "from", "to",
        "and", "but", "or", "yet", "so", "nor",
        "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did",
        "will", "would", "shall", "should", "may", "might", "can", "could",
        "not", "no", "if", "then", "than", "as", "of",
    ]
    counts = Counter(w for w in words if w in FUNCTION_WORDS)
    total = sum(counts.values()) or 1

    # Return normalized frequencies
    return [counts.get(fw, 0) / total for fw in FUNCTION_WORDS]


def compute_sentence_structure(sentences: list[str]) -> list[float]:
    """Extract sentence structure signature."""
    if not sentences:
        return []

    lengths = [len(s.split()) for s in sentences if s.strip()]
    if not lengths:
        return []

    mean_len = sum(lengths) / len(lengths)
    # Normalize lengths relative to mean
    normalized = [l / max(mean_len, 1) for l in lengths[:30]]

    # Add structural features
    features = [
        mean_len / 30.0,                                    # Average length (normalized)
        _variance(lengths) / max(mean_len ** 2, 1),         # Coefficient of variation squared
        sum(1 for l in lengths if l < 5) / len(lengths),    # Short sentence rate
        sum(1 for l in lengths if l > 25) / len(lengths),   # Long sentence rate
    ]
    return features + normalized


def compute_punctuation_profile(text: str) -> list[float]:
    """Build a punctuation usage fingerprint."""
    char_count = max(len(text), 1)
    punctuation_chars = [
        ".", ",", ";", ":", "!", "?", "-", "—", "(", ")",
        '"', "'", "...", "…",
    ]
    return [text.count(p) / char_count * 100 for p in punctuation_chars]


def compute_vocab_fingerprint(words: list[str], bins: int = 20) -> list[float]:
    """Create a vocabulary frequency distribution fingerprint."""
    if not words:
        return [0.0] * bins

    counts = Counter(words)
    total = len(words)

    # Create frequency histogram
    freq_values = sorted(counts.values(), reverse=True)
    bin_size = max(len(freq_values) // bins, 1)
    fingerprint = []
    for i in range(bins):
        start = i * bin_size
        end = min(start + bin_size, len(freq_values))
        if start < len(freq_values):
            bin_sum = sum(freq_values[start:end]) / total
            fingerprint.append(bin_sum)
        else:
            fingerprint.append(0.0)

    return fingerprint


def compute_certainty_vector(sentences: list[str]) -> list[float]:
    """Compute per-sentence certainty as an identity feature."""
    hedge = {"might", "could", "perhaps", "maybe", "possibly", "probably", "somewhat"}
    certain = {"must", "clearly", "obviously", "certainly", "definitely", "always", "never"}

    scores = []
    for s in sentences[:50]:  # Cap at 50 sentences
        words = set(s.lower().split())
        h = len(words & hedge)
        c = len(words & certain)
        if h + c > 0:
            scores.append(c / (h + c))
        else:
            scores.append(0.5)
    return scores


def compare_identities(a: IdentityEmbedding, b: IdentityEmbedding) -> float:
    """Compare two identity embeddings. Returns similarity [0, 1]."""
    similarities = []
    weights = []

    # Character n-gram similarity (strongest signal)
    if a.char_ngram_signature and b.char_ngram_signature:
        sim = _cosine_similarity(a.char_ngram_signature, b.char_ngram_signature)
        similarities.append(sim)
        weights.append(0.25)

    # Function word ratios
    if a.word_frequency_signature and b.word_frequency_signature:
        sim = _cosine_similarity(a.word_frequency_signature, b.word_frequency_signature)
        similarities.append(sim)
        weights.append(0.20)

    # Sentence structure
    if a.sentence_structure_pattern and b.sentence_structure_pattern:
        min_len = min(len(a.sentence_structure_pattern), len(b.sentence_structure_pattern))
        sim = _cosine_similarity(
            a.sentence_structure_pattern[:min_len],
            b.sentence_structure_pattern[:min_len],
        )
        similarities.append(sim)
        weights.append(0.15)

    # Punctuation
    if a.punctuation_pattern and b.punctuation_pattern:
        sim = _cosine_similarity(a.punctuation_pattern, b.punctuation_pattern)
        similarities.append(sim)
        weights.append(0.15)

    # Vocabulary fingerprint
    if a.vocabulary_fingerprint and b.vocabulary_fingerprint:
        sim = _cosine_similarity(a.vocabulary_fingerprint, b.vocabulary_fingerprint)
        similarities.append(sim)
        weights.append(0.15)

    # Certainty profile
    if a.certainty_profile and b.certainty_profile:
        min_len = min(len(a.certainty_profile), len(b.certainty_profile))
        sim = _cosine_similarity(a.certainty_profile[:min_len], b.certainty_profile[:min_len])
        similarities.append(sim)
        weights.append(0.10)

    if not similarities:
        return 0.0

    total_weight = sum(weights)
    return sum(s * w for s, w in zip(similarities, weights)) / total_weight


def _cosine_similarity(a: list[float], b: list[float]) -> float:
    """Compute cosine similarity between two vectors."""
    if len(a) != len(b) or not a:
        min_len = min(len(a), len(b))
        if min_len == 0:
            return 0.0
        a, b = a[:min_len], b[:min_len]

    dot = sum(x * y for x, y in zip(a, b))
    mag_a = math.sqrt(sum(x ** 2 for x in a))
    mag_b = math.sqrt(sum(x ** 2 for x in b))

    if mag_a == 0 or mag_b == 0:
        return 0.0
    return max(0.0, dot / (mag_a * mag_b))


def _variance(values: list) -> float:
    if len(values) < 2:
        return 0.0
    mean = sum(values) / len(values)
    return sum((v - mean) ** 2 for v in values) / (len(values) - 1)
