"""Tone & Emotion Analysis Research Domain.

Autonomous research into classifying emotional register, formality levels,
irony, sarcasm, and tonal shifts within documents.

Research directions:
- Multi-label tone classification (formal, casual, urgent, sarcastic, etc.)
- Tonal shift detection within documents
- Irony and sarcasm detection through contradiction and context
- Register-aware tone baselines
- Emotion trajectory mapping across paragraphs
- Micro-expression equivalents in text (word choice under stress)
- Formality gradient (not binary, but a spectrum)
- Cultural tone variation awareness
"""

import math
import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


# ─── Tone Categories ────────────────────────────────────────────────────────

TONE_LABELS = [
    "formal", "informal", "academic", "conversational", "professional",
    "urgent", "calm", "assertive", "tentative", "sarcastic",
    "enthusiastic", "neutral", "critical", "empathetic", "authoritative",
]

# ─── Tone Lexicons ──────────────────────────────────────────────────────────

FORMALITY_MARKERS = {
    "high": [
        "furthermore", "moreover", "nevertheless", "notwithstanding",
        "hereby", "thereof", "pursuant", "accordingly", "henceforth",
        "aforementioned", "hereinafter", "whereas",
    ],
    "low": [
        "gonna", "wanna", "kinda", "gotta", "y'all", "ain't",
        "yeah", "nah", "lol", "omg", "tbh", "imo", "btw",
        "super", "totally", "literally", "basically",
    ],
}

URGENCY_MARKERS = [
    "immediately", "urgent", "critical", "asap", "now", "deadline",
    "hurry", "emergency", "time-sensitive", "must", "imperative",
    "right away", "without delay", "at once",
]

ENTHUSIASM_MARKERS = [
    "amazing", "incredible", "fantastic", "wonderful", "excellent",
    "brilliant", "outstanding", "remarkable", "extraordinary",
    "love", "thrilled", "excited", "delighted", "passionate",
]

CRITICAL_MARKERS = [
    "however", "problematic", "flawed", "inadequate", "insufficient",
    "fails to", "overlooks", "ignores", "questionable", "concerning",
    "disappointing", "misguided", "fundamentally",
]

EMPATHY_MARKERS = [
    "i understand", "i can see", "that must be", "it's natural to",
    "you might feel", "it's okay to", "i appreciate",
    "that sounds", "i hear you", "i recognize",
]

SARCASM_INDICATORS = {
    "contradiction_markers": [
        "oh sure", "right", "yeah right", "of course", "obviously",
        "brilliant", "genius", "wonderful", "great job", "shocking",
    ],
    "exaggeration_markers": [
        "absolutely", "completely", "totally", "utterly", "perfectly",
        "clearly", "obviously",
    ],
    "punctuation_patterns": [
        r'\.{3,}',      # Ellipsis (extended)
        r'\?!+',         # Question + exclamation
        r'!{2,}',        # Multiple exclamation
        r'"[^"]{3,20}"', # Short quoted phrases (air quotes)
    ],
}


@dataclass
class ToneScore:
    """Multi-label tone classification result."""
    label: str
    confidence: float
    evidence: list[str] = field(default_factory=list)


@dataclass
class TonalShift:
    """A detected shift in tone within a document."""
    position: float          # Normalized position [0, 1]
    from_tone: str
    to_tone: str
    magnitude: float         # How dramatic the shift is [0, 1]
    trigger_sentence: str    # The sentence where the shift occurs


@dataclass
class EmotionTrajectory:
    """Emotion trajectory across a document."""
    valence_curve: list[float] = field(default_factory=list)    # Positive/negative
    arousal_curve: list[float] = field(default_factory=list)    # Calm/excited
    dominance_curve: list[float] = field(default_factory=list)  # Submissive/dominant
    overall_arc: str = ""    # "rising", "falling", "arc", "flat", "volatile"


@dataclass
class ToneAnalysisResult:
    """Complete tone analysis of a document."""
    primary_tones: list[ToneScore] = field(default_factory=list)
    formality_score: float = 0.5       # 0=very informal, 1=very formal
    urgency_score: float = 0.0
    sarcasm_probability: float = 0.0
    tonal_shifts: list[TonalShift] = field(default_factory=list)
    emotion_trajectory: Optional[EmotionTrajectory] = None
    consistency_score: float = 0.0     # How consistent the tone is throughout


class ToneAnalysisResearcher:
    """Autonomous researcher for tone and emotion analysis."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="tone-001",
                domain="tone_analysis",
                description="Formality gradient scoring using lexical density and "
                "sentence structure complexity",
                approach="Combine formal/informal marker ratios with average sentence "
                "length, passive voice rate, and Latinate vocabulary percentage.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="tone-002",
                domain="tone_analysis",
                description="Sarcasm detection through semantic contradiction: when "
                "positive words appear in negative contexts",
                approach="Detect contradictions between sentiment of individual words "
                "and the overall sentiment of surrounding context. Sarcasm creates "
                "a measurable sentiment inversion.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="tone-003",
                domain="tone_analysis",
                description="Tonal shift detection via sliding window tone vectors",
                approach="Compute tone feature vectors for overlapping windows. "
                "Detect large cosine distance jumps between consecutive windows "
                "as tonal shifts.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="tone-004",
                domain="tone_analysis",
                description="Emotion trajectory arc classification (rising, falling, "
                "arc, volatile) as a document-level feature",
                approach="Map valence scores across paragraphs and classify the "
                "overall shape. Different authors and registers produce "
                "characteristic arc shapes.",
                expected_impact="medium",
                complexity="moderate",
            ),
            Hypothesis(
                id="tone-005",
                domain="tone_analysis",
                description="AI text has more uniform tone than human text",
                approach="Measure tone consistency score (variance of per-paragraph "
                "tone vectors). AI maintains steady tone; humans fluctuate.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="tone-006",
                domain="tone_analysis",
                description="Micro-expressions in text: word choice shifts under stress "
                "or urgency reveal authentic human writing",
                approach="Detect localized vocabulary shifts that indicate emotional "
                "state changes. Under stress, sentence lengths shorten, concrete "
                "words increase, hedging decreases.",
                expected_impact="medium",
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
            texts = self._load_texts(data_dir)
            correct = 0
            total = 0

            for text, expected_tone in texts:
                result = analyze_tone(text)
                if result.primary_tones and result.primary_tones[0].label == expected_tone:
                    correct += 1
                total += 1

            metric_after = correct / max(total, 1)
            delta = metric_after - metric_before

            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="tone_analysis",
                metric_before=metric_before, metric_after=metric_after,
                delta=delta, duration_seconds=time.time() - start,
                memory_mb=0.0,
                status="improvement" if delta > 0.001 else (
                    "regression" if delta < -0.001 else "null"
                ),
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="tone_analysis",
                metric_before=metric_before, metric_after=metric_before,
                delta=0.0, duration_seconds=time.time() - start,
                memory_mb=0.0, status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.0

    def _load_texts(self, data_dir: Path) -> list:
        texts = []
        for register_dir in data_dir.iterdir():
            if register_dir.is_dir():
                tone = register_dir.name  # Use directory name as tone label
                for fpath in register_dir.glob("*.txt"):
                    text = fpath.read_text(errors="replace")
                    if len(text.split()) >= 100:
                        texts.append((text, tone))
        return texts


# ─── Tone Analysis Functions ────────────────────────────────────────────────

def analyze_tone(text: str) -> ToneAnalysisResult:
    """Run complete tone analysis on text."""
    result = ToneAnalysisResult()
    words = text.lower().split()
    sentences = re.split(r'(?<=[.!?])\s+', text)
    paragraphs = [p.strip() for p in text.split("\n\n") if p.strip()]

    # Formality scoring
    result.formality_score = compute_formality(text, words)

    # Multi-label tone classification
    result.primary_tones = classify_tones(text, words, sentences)

    # Urgency scoring
    result.urgency_score = compute_urgency(text, words)

    # Sarcasm detection
    result.sarcasm_probability = detect_sarcasm(text, sentences)

    # Tonal shifts
    if len(paragraphs) >= 3:
        result.tonal_shifts = detect_tonal_shifts(paragraphs)

    # Emotion trajectory
    if len(paragraphs) >= 2:
        result.emotion_trajectory = compute_emotion_trajectory(paragraphs)

    # Tone consistency
    result.consistency_score = compute_tone_consistency(paragraphs)

    return result


def compute_formality(text: str, words: list[str]) -> float:
    """Compute formality score on a 0-1 scale."""
    text_lower = text.lower()
    word_count = max(len(words), 1)

    # Count formal and informal markers
    formal_count = sum(1 for m in FORMALITY_MARKERS["high"] if m in text_lower)
    informal_count = sum(1 for m in FORMALITY_MARKERS["low"] if m in text_lower)

    # Sentence length factor (longer = more formal)
    sentences = re.split(r'[.!?]+', text)
    avg_sent_len = sum(len(s.split()) for s in sentences) / max(len(sentences), 1)
    length_factor = min(avg_sent_len / 25.0, 1.0)

    # Contraction rate (contractions = less formal)
    contractions = len(re.findall(r"\b\w+'(?:t|s|re|ve|ll|d|m)\b", text_lower))
    contraction_rate = contractions / word_count

    # Combine signals
    marker_score = (formal_count + 1) / (formal_count + informal_count + 2)
    formality = (
        0.40 * marker_score
        + 0.25 * length_factor
        + 0.20 * (1.0 - min(contraction_rate * 10, 1.0))
        + 0.15 * (1.0 if "!" not in text else 0.5)
    )
    return max(0.0, min(1.0, formality))


def classify_tones(text: str, words: list[str], sentences: list[str]) -> list[ToneScore]:
    """Classify text into multiple tone labels with confidence."""
    text_lower = text.lower()
    scores = []

    # Enthusiasm
    enth_count = sum(1 for m in ENTHUSIASM_MARKERS if m in text_lower)
    if enth_count > 0:
        scores.append(ToneScore("enthusiastic", min(enth_count / 5.0, 1.0)))

    # Critical
    crit_count = sum(1 for m in CRITICAL_MARKERS if m in text_lower)
    if crit_count > 0:
        scores.append(ToneScore("critical", min(crit_count / 5.0, 1.0)))

    # Empathetic
    emp_count = sum(1 for m in EMPATHY_MARKERS if m in text_lower)
    if emp_count > 0:
        scores.append(ToneScore("empathetic", min(emp_count / 3.0, 1.0)))

    # Assertive vs tentative (from certainty/hedge ratio)
    hedge_count = sum(1 for w in words if w in ["might", "could", "perhaps", "maybe", "possibly"])
    certain_count = sum(1 for w in words if w in ["must", "clearly", "obviously", "certainly"])
    if certain_count > hedge_count:
        scores.append(ToneScore("assertive", min(certain_count / 5.0, 1.0)))
    elif hedge_count > certain_count:
        scores.append(ToneScore("tentative", min(hedge_count / 5.0, 1.0)))

    # Formal/informal from formality score
    formality = compute_formality(text, words)
    if formality > 0.7:
        scores.append(ToneScore("formal", formality))
    elif formality < 0.3:
        scores.append(ToneScore("informal", 1.0 - formality))

    # Neutral fallback
    if not scores:
        scores.append(ToneScore("neutral", 0.8))

    # Sort by confidence
    scores.sort(key=lambda s: s.confidence, reverse=True)
    return scores[:5]


def compute_urgency(text: str, words: list[str]) -> float:
    """Score urgency level of text."""
    text_lower = text.lower()
    urgency_count = sum(1 for m in URGENCY_MARKERS if m in text_lower)

    # Exclamation marks add urgency
    excl = text.count("!")
    caps_words = sum(1 for w in text.split() if w.isupper() and len(w) > 2)

    score = (urgency_count * 0.15 + excl * 0.05 + caps_words * 0.03)
    return min(score, 1.0)


def detect_sarcasm(text: str, sentences: list[str]) -> float:
    """Detect sarcasm probability through contradiction and exaggeration."""
    text_lower = text.lower()
    signals = 0.0
    total_checks = 0

    # Check contradiction markers
    for marker in SARCASM_INDICATORS["contradiction_markers"]:
        if marker in text_lower:
            signals += 1
    total_checks += len(SARCASM_INDICATORS["contradiction_markers"])

    # Check punctuation patterns
    for pattern in SARCASM_INDICATORS["punctuation_patterns"]:
        if re.search(pattern, text):
            signals += 0.5
    total_checks += len(SARCASM_INDICATORS["punctuation_patterns"])

    # Sentiment inversion: positive words in negative context
    # (simplified: positive words followed by "not" or negation)
    pos_words = {"great", "wonderful", "amazing", "brilliant", "fantastic", "excellent"}
    for sentence in sentences:
        s_lower = sentence.lower()
        has_pos = any(w in s_lower for w in pos_words)
        has_neg = any(w in s_lower for w in ["not", "never", "no", "don't", "doesn't", "isn't"])
        if has_pos and has_neg:
            signals += 0.3

    return min(signals / max(total_checks * 0.3, 1), 1.0)


def detect_tonal_shifts(paragraphs: list[str]) -> list[TonalShift]:
    """Detect shifts in tone between paragraphs."""
    shifts = []
    prev_tone = None

    for i, para in enumerate(paragraphs):
        words = para.lower().split()
        if len(words) < 10:
            continue

        tones = classify_tones(para, words, re.split(r'[.!?]+', para))
        if not tones:
            continue

        current_tone = tones[0].label
        if prev_tone and current_tone != prev_tone:
            position = i / max(len(paragraphs) - 1, 1)
            # Magnitude based on how different the tones are
            magnitude = 0.5  # Default moderate shift
            if {prev_tone, current_tone} & {"sarcastic", "enthusiastic"}:
                magnitude = 0.8  # More dramatic shifts
            shifts.append(TonalShift(
                position=position,
                from_tone=prev_tone,
                to_tone=current_tone,
                magnitude=magnitude,
                trigger_sentence=para[:100],
            ))
        prev_tone = current_tone

    return shifts


def compute_emotion_trajectory(paragraphs: list[str]) -> EmotionTrajectory:
    """Map emotion trajectory across paragraphs."""
    trajectory = EmotionTrajectory()

    positive_words = {"good", "great", "happy", "love", "excellent", "wonderful", "beautiful", "joy"}
    negative_words = {"bad", "terrible", "hate", "awful", "horrible", "ugly", "sad", "pain", "fear"}
    high_arousal = {"exciting", "urgent", "critical", "amazing", "shocking", "furious", "thrilling"}
    low_arousal = {"calm", "peaceful", "quiet", "gentle", "subtle", "mild", "soft"}

    for para in paragraphs:
        words = set(para.lower().split())

        pos = len(words & positive_words)
        neg = len(words & negative_words)
        valence = (pos - neg) / max(pos + neg, 1)
        trajectory.valence_curve.append(valence)

        hi = len(words & high_arousal)
        lo = len(words & low_arousal)
        arousal = (hi - lo) / max(hi + lo, 1)
        trajectory.arousal_curve.append(arousal)

    # Classify arc shape
    if len(trajectory.valence_curve) >= 3:
        first_third = sum(trajectory.valence_curve[:len(trajectory.valence_curve)//3])
        last_third = sum(trajectory.valence_curve[-len(trajectory.valence_curve)//3:])
        mid = sum(trajectory.valence_curve[len(trajectory.valence_curve)//3:-len(trajectory.valence_curve)//3])

        variance = _variance(trajectory.valence_curve)
        if variance > 0.3:
            trajectory.overall_arc = "volatile"
        elif first_third < last_third - 0.2:
            trajectory.overall_arc = "rising"
        elif first_third > last_third + 0.2:
            trajectory.overall_arc = "falling"
        elif mid > first_third and mid > last_third:
            trajectory.overall_arc = "arc"
        else:
            trajectory.overall_arc = "flat"

    return trajectory


def compute_tone_consistency(paragraphs: list[str]) -> float:
    """Compute how consistent the tone is across paragraphs.

    AI text typically has higher consistency (more uniform tone).
    Human text has more natural variation.
    """
    if len(paragraphs) < 2:
        return 1.0

    formality_scores = []
    for para in paragraphs:
        words = para.lower().split()
        if len(words) >= 10:
            formality_scores.append(compute_formality(para, words))

    if len(formality_scores) < 2:
        return 1.0

    variance = _variance(formality_scores)
    # Low variance = high consistency
    consistency = 1.0 - min(variance * 10, 1.0)
    return max(0.0, consistency)


def _variance(values: list[float]) -> float:
    """Compute sample variance."""
    if len(values) < 2:
        return 0.0
    mean = sum(values) / len(values)
    return sum((v - mean) ** 2 for v in values) / (len(values) - 1)
