"""Logic & Psychology Research Domain.

Autonomous research into detecting reasoning patterns, persuasion strategies,
cognitive signatures, and psychological markers in text. This domain bridges
linguistic analysis with cognitive science.

Research directions:
- Argument structure detection (claim → evidence → conclusion)
- Logical fallacy identification
- Persuasion strategy classification (ethos/pathos/logos)
- Cognitive bias detection in writing
- Hedging and certainty language analysis
- Theory of Mind signals in text
- Decision-making patterns (risk aversion, confirmation bias)
- Genuine reasoning vs post-hoc rationalization
"""

import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import Hypothesis, ExperimentResult


# ─── Argument Structure Types ────────────────────────────────────────────────

@dataclass
class ArgumentStructure:
    """Detected argument structure in text."""
    claims: list[dict] = field(default_factory=list)
    evidence: list[dict] = field(default_factory=list)
    conclusions: list[dict] = field(default_factory=list)
    warrants: list[dict] = field(default_factory=list)  # Implicit assumptions
    rebuttals: list[dict] = field(default_factory=list)
    overall_pattern: str = ""  # "deductive", "inductive", "abductive", "none"


@dataclass
class PersuasionProfile:
    """Persuasion strategy analysis."""
    ethos_score: float = 0.0     # Appeal to authority/credibility
    pathos_score: float = 0.0    # Appeal to emotion
    logos_score: float = 0.0     # Appeal to logic/evidence
    dominant_strategy: str = ""
    rhetorical_devices: list[str] = field(default_factory=list)


@dataclass
class CognitiveProfile:
    """Cognitive signature analysis."""
    certainty_level: float = 0.0         # 0=very hedged, 1=very certain
    complexity_preference: float = 0.0    # Simple vs complex reasoning
    abstraction_level: float = 0.0        # Concrete vs abstract
    temporal_orientation: str = ""         # "past", "present", "future"
    risk_attitude: str = ""               # "risk-averse", "risk-neutral", "risk-seeking"
    detected_biases: list[str] = field(default_factory=list)
    theory_of_mind_signals: int = 0       # References to others' mental states


# ─── Marker Lexicons ────────────────────────────────────────────────────────

CLAIM_MARKERS = [
    "i argue", "i believe", "i contend", "we propose", "this suggests",
    "it is clear", "evidently", "the point is", "my position is",
    "we maintain", "i assert", "the thesis is", "our claim is",
]

EVIDENCE_MARKERS = [
    "for example", "for instance", "evidence shows", "research indicates",
    "studies suggest", "data shows", "according to", "as demonstrated",
    "the findings", "statistics show", "experiments reveal", "as shown by",
    "this is supported by", "consistent with",
]

CONCLUSION_MARKERS = [
    "therefore", "thus", "hence", "consequently", "as a result",
    "in conclusion", "it follows that", "we can conclude", "this means",
    "this implies", "the upshot is", "taken together",
]

HEDGE_WORDS = [
    "might", "could", "may", "perhaps", "possibly", "probably",
    "somewhat", "relatively", "arguably", "seemingly", "apparently",
    "it seems", "it appears", "to some extent", "in some cases",
    "tends to", "is likely", "suggests that",
]

CERTAINTY_WORDS = [
    "certainly", "definitely", "absolutely", "clearly", "obviously",
    "undoubtedly", "unquestionably", "without doubt", "inevitably",
    "always", "never", "every", "none", "must", "proven",
]

ETHOS_MARKERS = [
    "as an expert", "in my experience", "studies show", "research proves",
    "authorities agree", "experts say", "it is well established",
    "the consensus is", "peer-reviewed", "according to",
]

PATHOS_MARKERS = [
    "imagine", "feel", "suffering", "tragic", "beautiful", "horrifying",
    "inspiring", "heartbreaking", "devastating", "exciting", "alarming",
    "frightening", "moving", "touching", "powerful",
]

LOGOS_MARKERS = [
    "because", "since", "if...then", "given that", "follows from",
    "logically", "ratio", "percentage", "statistic", "correlation",
    "data", "evidence", "proof", "demonstrates",
]

FALLACY_PATTERNS = {
    "ad_hominem": [r"they\s+(?:are|were)\s+(?:just|merely|only)", r"what\s+(?:do|would)\s+they\s+know"],
    "straw_man": [r"(?:they|he|she)\s+(?:thinks?|believes?|wants?)\s+(?:that\s+)?(?:all|every|nothing)"],
    "false_dichotomy": [r"either\s+.+\s+or\s+.+\s+(?:nothing|no\s+other)", r"you(?:'re|\s+are)\s+either\s+with\s+us\s+or"],
    "appeal_to_authority": [r"(?:everyone|all\s+experts?)\s+(?:knows?|agrees?)"],
    "slippery_slope": [r"if\s+(?:we|they)\s+.+\s+(?:next|soon|eventually)\s+.+\s+will"],
    "circular_reasoning": [],  # Hard to detect with regex; needs deeper analysis
    "bandwagon": [r"(?:everyone|most\s+people|the\s+majority)\s+(?:thinks?|believes?|agrees?)"],
}

COGNITIVE_BIAS_MARKERS = {
    "confirmation_bias": ["confirms what", "as expected", "just as predicted", "proves my point"],
    "anchoring": ["starting from", "based on the initial", "the first estimate"],
    "availability_heuristic": ["i can think of", "comes to mind", "i remember when"],
    "sunk_cost": ["already invested", "come this far", "can't stop now", "too late to"],
    "framing_effect": [],  # Detected through structural analysis, not lexicon
}

THEORY_OF_MIND_MARKERS = [
    "they think", "she believes", "he feels", "they want", "she expects",
    "he hopes", "they fear", "from their perspective", "in their view",
    "they might think", "she would feel", "he probably wants",
    "put yourself in", "imagine being", "see it from",
]


class LogicPsychologyResearcher:
    """Autonomous researcher for logic and psychology analysis."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="logic-001",
                domain="logic_psychology",
                description="Argument structure detection using marker-based parsing "
                "with contextual disambiguation",
                approach="Identify claims, evidence, conclusions using lexical markers "
                "combined with sentence position and dependency patterns.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="logic-002",
                domain="logic_psychology",
                description="Persuasion strategy ratios (ethos/pathos/logos) are stable "
                "within-author and discriminative between authors",
                approach="Compute persuasion strategy scores and test whether the "
                "ratio is a reliable author feature.",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="logic-003",
                domain="logic_psychology",
                description="Certainty-to-hedging ratio differentiates AI from human text",
                approach="AI text tends toward moderate certainty everywhere. Humans "
                "show more extreme variation — very certain in some places, very "
                "hedged in others.",
                expected_impact="high",
                complexity="simple",
            ),
            Hypothesis(
                id="logic-004",
                domain="logic_psychology",
                description="Theory of Mind density correlates with human authorship",
                approach="Count references to others' mental states. Human text "
                "contains more ToM signals, especially in persuasive writing.",
                expected_impact="medium",
                complexity="simple",
            ),
            Hypothesis(
                id="logic-005",
                domain="logic_psychology",
                description="Logical fallacy patterns differ between AI and human text",
                approach="AI rarely commits fallacies (it's trained to be logical). "
                "Humans use fallacies naturally. Detect fallacy presence as a "
                "human-ness signal.",
                expected_impact="high",
                complexity="complex",
            ),
            Hypothesis(
                id="logic-006",
                domain="logic_psychology",
                description="Cognitive bias markers in text reveal human authorship",
                approach="Detect language patterns associated with cognitive biases "
                "(confirmation bias, anchoring, sunk cost). These are human "
                "cognitive signatures that AI doesn't naturally produce.",
                expected_impact="medium",
                complexity="moderate",
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
            results = []
            for text, label in texts:
                analysis = analyze_logic_psychology(text)
                results.append((analysis, label))

            metric_after = self._evaluate(results)
            delta = metric_after - metric_before

            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="logic_psychology",
                metric_before=metric_before, metric_after=metric_after,
                delta=delta, duration_seconds=time.time() - start,
                memory_mb=0.0,
                status="improvement" if delta > 0.001 else (
                    "regression" if delta < -0.001 else "null"
                ),
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="logic_psychology",
                metric_before=metric_before, metric_after=metric_before,
                delta=0.0, duration_seconds=time.time() - start,
                memory_mb=0.0, status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.0  # Bootstrap from zero

    def _load_texts(self, data_dir: Path) -> list:
        texts = []
        for fpath in data_dir.glob("**/*.txt"):
            text = fpath.read_text(errors="replace")
            label = "ai" if "/ai/" in str(fpath).lower() else "human"
            if len(text.split()) >= 200:
                texts.append((text, label))
        return texts

    def _evaluate(self, results: list) -> float:
        if not results:
            return 0.0
        # Evaluate argument detection quality
        scores = []
        for analysis, label in results:
            arg_score = len(analysis["arguments"].claims) + len(analysis["arguments"].evidence)
            scores.append(min(arg_score / 5.0, 1.0))  # Normalize
        return sum(scores) / len(scores)


# ─── Analysis Functions ──────────────────────────────────────────────────────

def analyze_logic_psychology(text: str) -> dict:
    """Run complete logic and psychology analysis on text."""
    sentences = re.split(r'(?<=[.!?])\s+', text)
    words = text.lower().split()

    return {
        "arguments": detect_argument_structure(sentences),
        "persuasion": analyze_persuasion(text, words),
        "cognitive": analyze_cognitive_profile(text, sentences, words),
        "fallacies": detect_fallacies(text),
        "certainty_profile": compute_certainty_profile(sentences),
    }


def detect_argument_structure(sentences: list[str]) -> ArgumentStructure:
    """Detect argument components in text."""
    structure = ArgumentStructure()
    text_lower = " ".join(sentences).lower()

    for i, sentence in enumerate(sentences):
        s_lower = sentence.lower()
        pos = i / max(len(sentences) - 1, 1)

        # Check for claims
        for marker in CLAIM_MARKERS:
            if marker in s_lower:
                structure.claims.append({
                    "text": sentence, "position": pos, "marker": marker
                })
                break

        # Check for evidence
        for marker in EVIDENCE_MARKERS:
            if marker in s_lower:
                structure.evidence.append({
                    "text": sentence, "position": pos, "marker": marker
                })
                break

        # Check for conclusions
        for marker in CONCLUSION_MARKERS:
            if marker in s_lower:
                structure.conclusions.append({
                    "text": sentence, "position": pos, "marker": marker
                })
                break

    # Determine overall pattern
    if structure.claims and structure.conclusions:
        claim_pos = sum(c["position"] for c in structure.claims) / len(structure.claims)
        concl_pos = sum(c["position"] for c in structure.conclusions) / len(structure.conclusions)
        if claim_pos < concl_pos:
            structure.overall_pattern = "deductive"  # General → specific
        else:
            structure.overall_pattern = "inductive"  # Specific → general
    elif structure.evidence and not structure.claims:
        structure.overall_pattern = "abductive"  # Evidence without explicit claims
    else:
        structure.overall_pattern = "none"

    return structure


def analyze_persuasion(text: str, words: list[str]) -> PersuasionProfile:
    """Classify persuasion strategies used in text."""
    profile = PersuasionProfile()
    text_lower = text.lower()
    word_count = max(len(words), 1)

    # Count markers for each strategy
    ethos_count = sum(1 for m in ETHOS_MARKERS if m in text_lower)
    pathos_count = sum(1 for m in PATHOS_MARKERS if m in text_lower)
    logos_count = sum(1 for m in LOGOS_MARKERS if m in text_lower)
    total = max(ethos_count + pathos_count + logos_count, 1)

    profile.ethos_score = ethos_count / total
    profile.pathos_score = pathos_count / total
    profile.logos_score = logos_count / total

    scores = {"ethos": profile.ethos_score, "pathos": profile.pathos_score, "logos": profile.logos_score}
    profile.dominant_strategy = max(scores, key=scores.get)

    # Detect rhetorical devices
    devices = []
    if re.search(r'\b(\w+)\b.*\b\1\b.*\b\1\b', text_lower[:500]):
        devices.append("repetition")
    if "?" in text and any(m in text_lower for m in ["isn't it", "don't you", "wouldn't"]):
        devices.append("rhetorical_question")
    if re.search(r'not\s+\w+\s+but\s+\w+', text_lower):
        devices.append("antithesis")
    profile.rhetorical_devices = devices

    return profile


def analyze_cognitive_profile(text: str, sentences: list[str], words: list[str]) -> CognitiveProfile:
    """Analyze cognitive patterns in text."""
    profile = CognitiveProfile()
    text_lower = text.lower()
    word_count = max(len(words), 1)

    # Certainty level
    hedge_count = sum(1 for h in HEDGE_WORDS if h in text_lower)
    certainty_count = sum(1 for c in CERTAINTY_WORDS if c in text_lower)
    total_cert = max(hedge_count + certainty_count, 1)
    profile.certainty_level = certainty_count / total_cert

    # Complexity preference (average sentence length as proxy)
    sent_lengths = [len(s.split()) for s in sentences]
    avg_len = sum(sent_lengths) / max(len(sent_lengths), 1)
    profile.complexity_preference = min(avg_len / 30.0, 1.0)  # Normalize to [0,1]

    # Abstraction level (concrete nouns vs abstract language)
    concrete_markers = ["the", "this", "that", "here", "there", "now", "today"]
    abstract_markers = ["concept", "theory", "principle", "notion", "idea", "essence"]
    concrete = sum(words.count(m) for m in concrete_markers)
    abstract = sum(words.count(m) for m in abstract_markers)
    total_abs = max(concrete + abstract, 1)
    profile.abstraction_level = abstract / total_abs

    # Temporal orientation
    past = sum(1 for w in words if w in ["was", "were", "had", "did", "went", "said"])
    present = sum(1 for w in words if w in ["is", "are", "has", "does", "go", "say"])
    future = sum(1 for w in words if w in ["will", "shall", "going to", "plan", "expect"])
    temps = {"past": past, "present": present, "future": future}
    profile.temporal_orientation = max(temps, key=temps.get)

    # Theory of Mind signals
    profile.theory_of_mind_signals = sum(1 for m in THEORY_OF_MIND_MARKERS if m in text_lower)

    # Cognitive biases
    for bias, markers in COGNITIVE_BIAS_MARKERS.items():
        if any(m in text_lower for m in markers):
            profile.detected_biases.append(bias)

    return profile


def detect_fallacies(text: str) -> list[dict]:
    """Detect potential logical fallacies in text."""
    text_lower = text.lower()
    detected = []

    for fallacy_name, patterns in FALLACY_PATTERNS.items():
        for pattern in patterns:
            matches = re.findall(pattern, text_lower)
            if matches:
                detected.append({
                    "fallacy": fallacy_name,
                    "count": len(matches),
                    "evidence": matches[:3],
                })
                break

    return detected


def compute_certainty_profile(sentences: list[str]) -> list[float]:
    """Compute certainty level per sentence.

    Returns a list of certainty scores. The *variance* of this profile
    is a key discriminator: humans vary more, AI is more uniform.
    """
    profile = []
    for sentence in sentences:
        s_lower = sentence.lower()
        hedge = sum(1 for h in HEDGE_WORDS if h in s_lower)
        certain = sum(1 for c in CERTAINTY_WORDS if c in s_lower)
        total = max(hedge + certain, 1)
        profile.append(certain / total)
    return profile
