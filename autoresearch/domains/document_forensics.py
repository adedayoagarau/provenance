"""Document Forensics Research Domain.

Autonomous research into deep document-level forensics that go beyond
text analysis to examine how documents were constructed, edited, and
modified over time.

Research directions:
- Edit pattern analysis (typing rhythm, revision strategies)
- Multi-author document detection
- Copy-paste boundary detection
- Document assembly forensics (pieced together from multiple sources)
- Metadata consistency verification
- Formatting forensics (style inheritance, template detection)
- Timeline reconstruction from edit history
- Ghost-writing detection
"""

import re
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

from . import Hypothesis, ExperimentResult


@dataclass
class DocumentForensicsResult:
    """Deep forensics analysis of a document's construction."""
    multi_author_probability: float = 0.0
    copy_paste_boundaries: list[dict] = field(default_factory=list)
    style_discontinuities: list[dict] = field(default_factory=list)
    assembly_score: float = 0.0        # Probability of multi-source assembly
    ghost_writing_indicators: list[str] = field(default_factory=list)
    editing_pattern: str = ""           # "linear", "non-linear", "assembled"
    consistency_score: float = 0.0      # Overall internal consistency


class DocumentForensicsResearcher:
    """Autonomous researcher for document forensics."""

    def get_hypotheses(self) -> list[Hypothesis]:
        return [
            Hypothesis(
                id="doc-001",
                domain="document_forensics",
                description="Style discontinuity detection at paragraph boundaries "
                "reveals multi-author documents",
                approach="Compare stylometric feature vectors between consecutive "
                "paragraphs. Large jumps in the feature space indicate different "
                "authors or AI-assisted sections.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="doc-002",
                domain="document_forensics",
                description="Copy-paste from AI sources creates detectable boundary "
                "artifacts",
                approach="At copy-paste boundaries, there are often subtle style "
                "shifts: vocabulary level changes, sentence structure jumps, "
                "topic coherence breaks without proper transitions.",
                expected_impact="high",
                complexity="moderate",
            ),
            Hypothesis(
                id="doc-003",
                domain="document_forensics",
                description="Ghost-writing leaves characteristic structural patterns",
                approach="Ghost-written documents have high vocabulary sophistication "
                "but low personal style markers. Detect the absence of personal "
                "voice as a ghost-writing signal.",
                expected_impact="medium",
                complexity="moderate",
            ),
            Hypothesis(
                id="doc-004",
                domain="document_forensics",
                description="Paragraph ordering analysis reveals whether text was "
                "written linearly or assembled",
                approach="Analyze topic flow and reference chains. Linearly written "
                "text has natural forward references and topic development. "
                "Assembled text has more abrupt topic shifts.",
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
            texts = self._load_documents(data_dir)
            results = []
            for text, is_multi_author in texts:
                analysis = analyze_document_construction(text)
                predicted = analysis.multi_author_probability > 0.5
                results.append(predicted == is_multi_author)

            accuracy = sum(results) / max(len(results), 1)
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="document_forensics",
                metric_before=0.5, metric_after=accuracy,
                delta=accuracy - 0.5, duration_seconds=time.time() - start,
                memory_mb=0.0,
                status="improvement" if accuracy > 0.55 else "null",
            )
        except Exception as e:
            return ExperimentResult(
                hypothesis_id=hypothesis.id, domain="document_forensics",
                metric_before=0.5, metric_after=0.5, delta=0.0,
                duration_seconds=time.time() - start, memory_mb=0.0,
                status="error", notes=str(e)[:200],
            )

    def get_current_baseline(self, data_dir: Path) -> float:
        return 0.5

    def _load_documents(self, data_dir: Path) -> list:
        docs = []
        for fpath in data_dir.glob("**/*.txt"):
            text = fpath.read_text(errors="replace")
            is_multi = "multi" in str(fpath).lower() or "assembled" in str(fpath).lower()
            docs.append((text, is_multi))
        return docs


def analyze_document_construction(text: str) -> DocumentForensicsResult:
    """Analyze how a document was constructed."""
    result = DocumentForensicsResult()
    paragraphs = [p.strip() for p in text.split("\n\n") if p.strip()]

    if len(paragraphs) < 2:
        result.consistency_score = 1.0
        result.editing_pattern = "linear"
        return result

    # Detect style discontinuities between paragraphs
    discontinuities = detect_style_discontinuities(paragraphs)
    result.style_discontinuities = discontinuities

    # Multi-author probability based on discontinuity severity
    if discontinuities:
        max_severity = max(d["severity"] for d in discontinuities)
        avg_severity = sum(d["severity"] for d in discontinuities) / len(discontinuities)
        result.multi_author_probability = min(avg_severity * 2, 1.0)
    else:
        result.multi_author_probability = 0.0

    # Copy-paste boundary detection
    result.copy_paste_boundaries = detect_copy_paste_boundaries(paragraphs)

    # Overall consistency
    result.consistency_score = 1.0 - result.multi_author_probability

    # Editing pattern
    if result.multi_author_probability > 0.6:
        result.editing_pattern = "assembled"
    elif len(result.copy_paste_boundaries) > 2:
        result.editing_pattern = "non-linear"
    else:
        result.editing_pattern = "linear"

    return result


def detect_style_discontinuities(paragraphs: list[str]) -> list[dict]:
    """Detect style jumps between consecutive paragraphs."""
    discontinuities = []

    for i in range(len(paragraphs) - 1):
        para_a = paragraphs[i]
        para_b = paragraphs[i + 1]

        words_a = para_a.lower().split()
        words_b = para_b.lower().split()

        if len(words_a) < 10 or len(words_b) < 10:
            continue

        # Compare avg sentence lengths
        sents_a = [s for s in para_a.split(".") if s.strip()]
        sents_b = [s for s in para_b.split(".") if s.strip()]
        avg_len_a = len(words_a) / max(len(sents_a), 1)
        avg_len_b = len(words_b) / max(len(sents_b), 1)

        # Compare vocabulary richness
        ttr_a = len(set(words_a)) / len(words_a)
        ttr_b = len(set(words_b)) / len(words_b)

        # Style distance
        len_diff = abs(avg_len_a - avg_len_b) / max(avg_len_a, avg_len_b, 1)
        ttr_diff = abs(ttr_a - ttr_b)

        severity = (len_diff + ttr_diff) / 2

        if severity > 0.2:  # Threshold for reporting
            discontinuities.append({
                "position": i,
                "severity": severity,
                "details": {
                    "sentence_length_shift": avg_len_b - avg_len_a,
                    "vocabulary_shift": ttr_b - ttr_a,
                },
            })

    return discontinuities


def detect_copy_paste_boundaries(paragraphs: list[str]) -> list[dict]:
    """Detect potential copy-paste boundaries in text."""
    boundaries = []

    for i in range(len(paragraphs) - 1):
        para_a = paragraphs[i]
        para_b = paragraphs[i + 1]

        # Check for missing transitions
        transition_words = {"however", "moreover", "furthermore", "additionally",
                          "nevertheless", "therefore", "consequently", "meanwhile",
                          "similarly", "in contrast"}
        first_word = para_b.split()[0].lower() if para_b.split() else ""
        has_transition = first_word in transition_words

        # Check for topic coherence break
        words_a = set(para_a.lower().split())
        words_b = set(para_b.lower().split())
        # Remove common stop words for overlap calculation
        stop_words = {"the", "a", "an", "is", "are", "was", "were", "in", "on", "at",
                     "to", "for", "of", "and", "or", "but", "not", "with", "this", "that"}
        content_a = words_a - stop_words
        content_b = words_b - stop_words
        overlap = len(content_a & content_b) / max(len(content_a | content_b), 1)

        if overlap < 0.05 and not has_transition:
            boundaries.append({
                "position": i,
                "topic_overlap": overlap,
                "has_transition": has_transition,
                "confidence": 1.0 - overlap,
            })

    return boundaries
