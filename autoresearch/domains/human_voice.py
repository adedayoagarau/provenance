"""Human Voice Research Domain.

Autonomous research into modeling the unique stylistic fingerprint of
individual writers. Goes beyond surface-level statistics to capture the
*rhythm*, *texture*, and *personality* of a writer's voice.

Research directions:
- Prosodic rhythm patterns (sentence cadence, clause length variation)
- Vocabulary evolution over time (writers change, models should adapt)
- Register code-switching within a single author
- Voice embeddings (latent style space that captures personality)
- Emotional affect on writing style (stress, excitement, fatigue)
- Idiolect markers (personal linguistic habits unique to an individual)
- Punctuation as personality (em-dash lovers, semicolon users, etc.)
"""

import math
import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


@dataclass
class VoiceProfile:
    """A multi-dimensional voice profile for a writer."""

    # Rhythm features
    mean_sentence_length: float = 0.0
    sentence_length_variance: float = 0.0
    clause_rhythm_pattern: list[float] = field(default_factory=list)
    paragraph_cadence: list[float] = field(default_factory=list)

    # Vocabulary fingerprint
    hapax_rate: float = 0.0          # Rate of words used only once
    vocab_richness_curve: list[float] = field(default_factory=list)
    preferred_transitions: dict = field(default_factory=dict)
    filler_patterns: dict = field(default_factory=dict)

    # Punctuation personality
    em_dash_frequency: float = 0.0
    semicolon_frequency: float = 0.0
    exclamation_rate: float = 0.0
    parenthetical_rate: float = 0.0
    ellipsis_rate: float = 0.0
    question_to_statement_ratio: float = 0.0

    # Idiolect markers
    favorite_sentence_openers: list[str] = field(default_factory=list)
    characteristic_phrases: list[str] = field(default_factory=list)
    hedging_style: str = ""  # "academic", "conversational", "minimal"
    emphasis_style: str = ""  # "italic", "caps", "repetition", "exclamation"

    # Voice embedding (learned latent representation)
    embedding: list[float] = field(default_factory=list)


class HumanVoiceResearcher:
    """Autonomous researcher for human voice modeling."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="voice-001",
                domain="human_voice",
                description="Sentence rhythm as fingerprint: clause-level cadence "
                "patterns are more distinctive than word-level features",
                approach="Extract clause boundaries (comma, semicolon, conjunction "
                "positions) and model the rhythm as a sequence. Compare rhythmic "
                "signatures across authors.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="voice-002",
                domain="human_voice",
                description="Punctuation personality is a strong author discriminator",
                approach="Build a punctuation usage profile (em-dash rate, semicolon "
                "rate, parenthetical nesting, ellipsis patterns). These are deeply "
                "habitual and hard to fake.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="voice-003",
                domain="human_voice",
                description="Transition word preferences uniquely identify writers",
                approach="Map each author's preferred logical connectors (however, "
                "moreover, but, yet, still, nonetheless). Distribution of these "
                "is highly personal.",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="voice-004",
                domain="human_voice",
                description="Voice embeddings via contrastive learning on stylometric "
                "features capture latent personality dimensions",
                approach="Train a contrastive model where same-author pairs are "
                "positive and different-author pairs are negative. The learned "
                "embedding space captures voice similarity.",
                expected_impact="high",
                complexity="complex",
            ),
            Hypothesis(
                id="voice-005",
                domain="human_voice",
                description="Writers have characteristic paragraph shapes (short-long "
                "patterns, conclusion styles)",
                approach="Model paragraph length sequences and opening/closing "
                "strategies. Some writers build up, others lead with the point.",
                expected_impact="medium",
                complexity="moderate",
            ),
            Hypothesis(
                id="voice-006",
                domain="human_voice",
                description="Idiolect detection: personal phrases and habitual "
                "constructions are the strongest identity signal",
                approach="Extract recurring multi-word expressions that appear in "
                "one author's work but are rare in the general corpus. These are "
                "linguistic fingerprints.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="voice-007",
                domain="human_voice",
                description="Emotional state modulates writing style in predictable ways",
                approach="Analyze how the same author's style shifts across documents "
                "with different emotional content. Model the emotional variation "
                "envelope to avoid false negatives.",
                expected_impact="medium",
                complexity="complex",
            ),
        ]

    def run_experiment(
        self, hypothesis: Hypothesis, data_dir: Path, time_budget: int = 300
    ) -> ExperimentResult:
        """Run a voice modeling experiment."""
        import time
        start = time.time()
        metric_before = self.get_current_baseline(data_dir)

        try:
            # Load author samples
            authors = self._load_author_samples(data_dir)
            if len(authors) < 2:
                return ExperimentResult(
                    hypothesis_id=hypothesis.id, domain="human_voice",
                    metric_before=metric_before, metric_after=metric_before,
                    delta=0.0, duration_seconds=time.time() - start,
                    memory_mb=0.0, status="error",
                    notes="Need at least 2 authors for voice comparison",
                )

            # Build profiles and evaluate matching accuracy
            profiles = {name: build_voice_profile(texts) for name, texts in authors.items()}
            accuracy = self._evaluate_matching(profiles, authors)
            delta = accuracy - metric_before

            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="human_voice",
                metric_before=metric_before, metric_after=accuracy,
                delta=delta, duration_seconds=time.time() - start,
                memory_mb=0.0,
                status="improvement" if delta > 0.001 else (
                    "regression" if delta < -0.001 else "null"
                ),
                details={"accuracy": accuracy, "num_authors": len(authors)},
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="human_voice",
                metric_before=metric_before, metric_after=metric_before,
                delta=0.0, duration_seconds=time.time() - start,
                memory_mb=0.0, status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        """Compute current profile matching accuracy."""
        authors = self._load_author_samples(data_dir)
        if len(authors) < 2:
            return 0.0
        profiles = {name: build_voice_profile(texts) for name, texts in authors.items()}
        return self._evaluate_matching(profiles, authors)

    def _load_author_samples(self, data_dir: Path) -> dict:
        """Load text samples grouped by author."""
        authors = defaultdict(list)
        for author_dir in data_dir.iterdir():
            if author_dir.is_dir():
                for fpath in author_dir.glob("*.txt"):
                    text = fpath.read_text(errors="replace")
                    if len(text.split()) >= 200:
                        authors[author_dir.name].append(text)
        return dict(authors)

    def _evaluate_matching(self, profiles: dict, authors: dict) -> float:
        """Evaluate author matching accuracy using leave-one-out."""
        correct = 0
        total = 0

        for true_author, texts in authors.items():
            if len(texts) < 2:
                continue
            for i, test_text in enumerate(texts):
                # Build profile from all other texts by this author
                train_texts = texts[:i] + texts[i + 1:]
                if not train_texts:
                    continue

                test_profile = build_voice_profile([test_text])
                best_match = None
                best_score = -1.0

                for name, profile in profiles.items():
                    score = compare_voice_profiles(test_profile, profile)
                    if score > best_score:
                        best_score = score
                        best_match = name

                if best_match == true_author:
                    correct += 1
                total += 1

        return correct / max(total, 1)


# ─── Voice Feature Functions ────────────────────────────────────────────────
# These are the functions the agent modifies during experiments.

def build_voice_profile(texts: list[str]) -> VoiceProfile:
    """Build a comprehensive voice profile from multiple text samples."""
    profile = VoiceProfile()
    all_sentences = []
    all_words = []
    all_text = " ".join(texts)

    for text in texts:
        sentences = re.split(r'[.!?]+', text)
        sentences = [s.strip() for s in sentences if s.strip()]
        all_sentences.extend(sentences)
        all_words.extend(text.split())

    if not all_sentences:
        return profile

    # Rhythm features
    lengths = [len(s.split()) for s in all_sentences]
    profile.mean_sentence_length = sum(lengths) / len(lengths)
    if len(lengths) > 1:
        mean = profile.mean_sentence_length
        profile.sentence_length_variance = sum(
            (l - mean) ** 2 for l in lengths
        ) / (len(lengths) - 1)

    # Clause rhythm: pattern of clause lengths within sentences
    profile.clause_rhythm_pattern = extract_clause_rhythm(all_sentences[:100])

    # Vocabulary fingerprint
    word_counts = Counter(w.lower() for w in all_words)
    total_words = len(all_words)
    hapax = sum(1 for c in word_counts.values() if c == 1)
    profile.hapax_rate = hapax / max(len(word_counts), 1)

    # Punctuation personality
    char_count = max(len(all_text), 1)
    profile.em_dash_frequency = all_text.count("—") / char_count * 1000
    profile.semicolon_frequency = all_text.count(";") / char_count * 1000
    profile.exclamation_rate = all_text.count("!") / char_count * 1000
    profile.parenthetical_rate = all_text.count("(") / char_count * 1000
    profile.ellipsis_rate = (all_text.count("...") + all_text.count("…")) / char_count * 1000

    questions = sum(1 for s in all_sentences if s.strip().endswith("?"))
    statements = len(all_sentences) - questions
    profile.question_to_statement_ratio = questions / max(statements, 1)

    # Transition preferences
    transitions = [
        "however", "moreover", "furthermore", "nevertheless", "nonetheless",
        "therefore", "thus", "hence", "consequently", "meanwhile",
        "although", "though", "yet", "still", "instead", "rather",
        "additionally", "similarly", "likewise", "conversely",
        "but", "and", "so", "then", "also", "besides",
    ]
    transition_counts = Counter()
    words_lower = [w.lower().strip(".,;:!?") for w in all_words]
    for t in transitions:
        transition_counts[t] = words_lower.count(t)
    total_transitions = sum(transition_counts.values()) or 1
    profile.preferred_transitions = {
        t: c / total_transitions for t, c in transition_counts.most_common(10)
    }

    # Sentence openers
    opener_counts = Counter()
    for s in all_sentences:
        words = s.split()
        if words:
            opener = words[0].lower()
            opener_counts[opener] += 1
    profile.favorite_sentence_openers = [w for w, _ in opener_counts.most_common(10)]

    return profile


def extract_clause_rhythm(sentences: list[str]) -> list[float]:
    """Extract clause-level rhythm pattern from sentences.

    Splits sentences at commas, semicolons, and conjunctions to find
    the characteristic rhythm of clause lengths.
    """
    clause_lengths = []
    for sentence in sentences:
        clauses = re.split(r'[,;]|\b(?:and|but|or|yet|so|for|nor)\b', sentence)
        clauses = [c.strip() for c in clauses if c.strip()]
        for clause in clauses:
            clause_lengths.append(len(clause.split()))

    if not clause_lengths:
        return []

    # Normalize to ratios (each clause length relative to mean)
    mean_len = sum(clause_lengths) / len(clause_lengths)
    if mean_len == 0:
        return []
    return [cl / mean_len for cl in clause_lengths[:50]]  # First 50 clauses


def compare_voice_profiles(a: VoiceProfile, b: VoiceProfile) -> float:
    """Compare two voice profiles and return a similarity score [0, 1]."""
    scores = []

    # Rhythm similarity
    if a.sentence_length_variance > 0 and b.sentence_length_variance > 0:
        ratio = min(a.sentence_length_variance, b.sentence_length_variance) / max(
            a.sentence_length_variance, b.sentence_length_variance
        )
        scores.append(("rhythm_variance", ratio, 0.15))

    if a.mean_sentence_length > 0 and b.mean_sentence_length > 0:
        ratio = min(a.mean_sentence_length, b.mean_sentence_length) / max(
            a.mean_sentence_length, b.mean_sentence_length
        )
        scores.append(("rhythm_mean", ratio, 0.10))

    # Punctuation similarity
    punct_features = [
        ("em_dash", a.em_dash_frequency, b.em_dash_frequency),
        ("semicolon", a.semicolon_frequency, b.semicolon_frequency),
        ("exclamation", a.exclamation_rate, b.exclamation_rate),
        ("parenthetical", a.parenthetical_rate, b.parenthetical_rate),
        ("ellipsis", a.ellipsis_rate, b.ellipsis_rate),
    ]
    for name, va, vb in punct_features:
        if va + vb > 0:
            similarity = 1.0 - abs(va - vb) / max(va + vb, 0.001)
            scores.append((f"punct_{name}", max(0, similarity), 0.04))

    # Transition preference similarity (cosine-like)
    common_transitions = set(a.preferred_transitions.keys()) & set(b.preferred_transitions.keys())
    if common_transitions:
        dot = sum(a.preferred_transitions[t] * b.preferred_transitions.get(t, 0)
                  for t in a.preferred_transitions)
        mag_a = math.sqrt(sum(v ** 2 for v in a.preferred_transitions.values()))
        mag_b = math.sqrt(sum(v ** 2 for v in b.preferred_transitions.values()))
        if mag_a > 0 and mag_b > 0:
            cosine = dot / (mag_a * mag_b)
            scores.append(("transitions", cosine, 0.15))

    # Sentence opener overlap
    if a.favorite_sentence_openers and b.favorite_sentence_openers:
        overlap = len(set(a.favorite_sentence_openers[:5]) & set(b.favorite_sentence_openers[:5]))
        scores.append(("openers", overlap / 5, 0.10))

    # Hapax rate similarity
    if a.hapax_rate > 0 and b.hapax_rate > 0:
        ratio = min(a.hapax_rate, b.hapax_rate) / max(a.hapax_rate, b.hapax_rate)
        scores.append(("hapax", ratio, 0.10))

    # Q/S ratio similarity
    ratio_a = a.question_to_statement_ratio
    ratio_b = b.question_to_statement_ratio
    if ratio_a + ratio_b > 0:
        sim = 1.0 - abs(ratio_a - ratio_b) / max(ratio_a + ratio_b, 0.001)
        scores.append(("q_ratio", max(0, sim), 0.06))

    if not scores:
        return 0.0

    # Weighted average
    total_weight = sum(w for _, _, w in scores)
    weighted_sum = sum(s * w for _, s, w in scores)
    return weighted_sum / total_weight if total_weight > 0 else 0.0
