"""Base researcher class and shared utilities for the Research Council.

Every researcher module inherits from BaseResearcher, which provides:
- Data loading and sampling from JSONL
- Feature computation pipeline
- Statistical test helpers (Cohen's d, t-tests, chi-squared)
- Memo generation scaffolding
"""

from __future__ import annotations

import hashlib
import json
import logging
import math
import os
import re
import sys
from abc import ABC, abstractmethod
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

import pandas as pd
import numpy as np

# Resolve project paths
_THIS_DIR = Path(__file__).resolve().parent
_AUTORESEARCH_DIR = _THIS_DIR.parent
sys.path.insert(0, str(_AUTORESEARCH_DIR.parent))

from autoresearch.config import (
    DATASET_PATH,
    RESEARCH_OUTPUT_DIR,
    QUICK_SAMPLE_SIZE,
    FULL_SAMPLE_SIZE,
    RANDOM_SEED,
    MIN_EFFECT_SIZE,
    ALPHA,
    MIN_GROUP_SIZE,
    TEXT_FEATURES,
)

logger = logging.getLogger("research_council")


# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------

def load_dataset(
    sample_size: int = 0,
    label_filter: Optional[list[str]] = None,
    model_filter: Optional[list[str]] = None,
    seed: int = RANDOM_SEED,
) -> pd.DataFrame:
    """Load the JSONL dataset, optionally sampling for performance.

    Parameters
    ----------
    sample_size : int
        If > 0, return a random sample of this size.  0 = full dataset.
    label_filter : list[str] | None
        If provided, keep only these labels (e.g. ["human", "ai"]).
    model_filter : list[str] | None
        If provided, keep only these models.
    seed : int
        Random seed for reproducible sampling.

    Returns
    -------
    pd.DataFrame with columns matching the JSONL schema.
    """
    path = DATASET_PATH
    if not path.exists():
        raise FileNotFoundError(
            f"Dataset not found at {path}. "
            "Please ensure training/data/raw/combined.jsonl exists."
        )

    logger.info("Loading dataset from %s ...", path)

    # For very large files, read in chunks
    chunks = []
    for chunk in pd.read_json(path, lines=True, chunksize=50_000):
        if label_filter:
            chunk = chunk[chunk["label"].isin(label_filter)]
        if model_filter:
            chunk = chunk[chunk["model"].isin(model_filter)]
        chunks.append(chunk)

    df = pd.concat(chunks, ignore_index=True)
    logger.info("Loaded %d rows (after filters).", len(df))

    if sample_size > 0 and len(df) > sample_size:
        df = df.sample(n=sample_size, random_state=seed)
        logger.info("Sampled down to %d rows.", len(df))

    return df


def quick_load(
    label_filter: Optional[list[str]] = None,
    model_filter: Optional[list[str]] = None,
) -> pd.DataFrame:
    """Load a quick sample for exploratory analysis."""
    return load_dataset(
        sample_size=QUICK_SAMPLE_SIZE,
        label_filter=label_filter,
        model_filter=model_filter,
    )


def full_load(
    label_filter: Optional[list[str]] = None,
    model_filter: Optional[list[str]] = None,
) -> pd.DataFrame:
    """Load a larger sample for final memo-quality analysis."""
    return load_dataset(
        sample_size=FULL_SAMPLE_SIZE,
        label_filter=label_filter,
        model_filter=model_filter,
    )


# ---------------------------------------------------------------------------
# Feature computation
# ---------------------------------------------------------------------------

_FUNCTION_WORDS = frozenset(
    "the of and to a in is it you that he was for on are with as i his they "
    "be at one have this from or had by but not what all were we when your can "
    "there use an each which she do how their if will up other about out many "
    "then them these so some her would make like him into time has look two "
    "more go see no way could my than first been call who its now find long "
    "down day did get come made may part".split()
)

_HEDGE_WORDS = frozenset(
    "perhaps maybe possibly somewhat apparently seemingly likely probably "
    "generally typically usually often sometimes occasionally tends might "
    "could may seem appear suggest indicate".split()
)

_DISCOURSE_MARKERS = frozenset(
    "however moreover furthermore additionally nevertheless nonetheless "
    "consequently therefore thus hence meanwhile indeed specifically "
    "particularly notably importantly significantly essentially basically "
    "actually certainly clearly obviously".split()
)


def _tokenize_words(text: str) -> list[str]:
    """Simple whitespace + punctuation tokenizer."""
    return re.findall(r"[a-zA-Z']+", text.lower())


def _tokenize_sentences(text: str) -> list[str]:
    """Split text into sentences."""
    sentences = re.split(r'(?<=[.!?])\s+', text.strip())
    return [s for s in sentences if len(s) > 0]


def compute_features(text: str) -> dict[str, float]:
    """Compute all TEXT_FEATURES for a single text document.

    Returns a dict keyed by feature name.  Missing / undefined features
    are returned as NaN so downstream analysis can handle gracefully.
    """
    features: dict[str, float] = {}
    nan = float("nan")

    words = _tokenize_words(text)
    sentences = _tokenize_sentences(text)
    paragraphs = [p.strip() for p in text.split("\n\n") if p.strip()]

    n_words = len(words)
    n_sentences = max(len(sentences), 1)
    n_paragraphs = max(len(paragraphs), 1)

    features["word_count"] = n_words
    features["sentence_count"] = n_sentences

    # Sentence length stats
    sent_lengths = [len(_tokenize_words(s)) for s in sentences]
    features["avg_sentence_length"] = np.mean(sent_lengths) if sent_lengths else nan
    features["sentence_length_std"] = np.std(sent_lengths, ddof=1) if len(sent_lengths) > 1 else nan

    # Word length
    word_lengths = [len(w) for w in words]
    features["avg_word_length"] = np.mean(word_lengths) if word_lengths else nan

    # Vocabulary richness
    word_freq = Counter(words)
    n_types = len(word_freq)
    features["type_token_ratio"] = n_types / max(n_words, 1)
    hapax = sum(1 for w, c in word_freq.items() if c == 1)
    features["hapax_ratio"] = hapax / max(n_types, 1)

    # Function word ratio
    func_count = sum(1 for w in words if w in _FUNCTION_WORDS)
    features["function_word_ratio"] = func_count / max(n_words, 1)

    # Punctuation
    all_chars = len(text)
    punct_count = sum(1 for c in text if c in '.,;:!?-()[]{}"\'/\\')
    features["punctuation_ratio"] = punct_count / max(all_chars, 1)
    features["comma_rate"] = text.count(",") / max(n_sentences, 1)
    features["semicolon_rate"] = text.count(";") / max(n_sentences, 1)
    features["question_rate"] = text.count("?") / max(n_sentences, 1)
    features["exclamation_rate"] = text.count("!") / max(n_sentences, 1)

    # Paragraph structure
    features["paragraph_count"] = n_paragraphs
    para_lengths = [len(_tokenize_words(p)) for p in paragraphs]
    features["avg_paragraph_length"] = np.mean(para_lengths) if para_lengths else nan

    # Entropy measures
    features["word_entropy"] = _shannon_entropy(word_freq, n_words)
    char_freq = Counter(text.lower())
    features["char_entropy"] = _shannon_entropy(char_freq, len(text))
    bigrams = [text[i:i+2].lower() for i in range(len(text) - 1)]
    bigram_freq = Counter(bigrams)
    features["bigram_entropy"] = _shannon_entropy(bigram_freq, len(bigrams)) if bigrams else nan

    # Vocabulary richness measures
    features["yule_k"] = _yule_k(word_freq, n_words)
    features["brunet_w"] = _brunet_w(n_words, n_types)
    features["honore_r"] = _honore_r(n_words, n_types, hapax)

    # Readability
    syllable_counts = [_count_syllables(w) for w in words]
    total_syllables = sum(syllable_counts)
    features["flesch_kincaid_grade"] = (
        0.39 * (n_words / max(n_sentences, 1))
        + 11.8 * (total_syllables / max(n_words, 1))
        - 15.59
    ) if n_words > 0 else nan
    features["automated_readability_index"] = (
        4.71 * (sum(len(w) for w in words) / max(n_words, 1))
        + 0.5 * (n_words / max(n_sentences, 1))
        - 21.43
    ) if n_words > 0 else nan

    # Discourse features
    features["conjunction_rate"] = sum(
        1 for w in words if w in {"and", "but", "or", "nor", "yet", "so", "for"}
    ) / max(n_words, 1)
    features["adverb_rate"] = sum(
        1 for w in words if w.endswith("ly") and len(w) > 3
    ) / max(n_words, 1)

    # Passive voice approximation (was/were + past participle pattern)
    passive_count = len(re.findall(
        r'\b(?:was|were|is|are|been|being)\s+\w+ed\b', text.lower()
    ))
    features["passive_voice_ratio"] = passive_count / max(n_sentences, 1)

    # First person usage
    first_person = sum(1 for w in words if w in {"i", "me", "my", "mine", "myself", "we", "us", "our", "ours"})
    features["first_person_ratio"] = first_person / max(n_words, 1)

    # Hedge words
    hedge_count = sum(1 for w in words if w in _HEDGE_WORDS)
    features["hedge_word_ratio"] = hedge_count / max(n_words, 1)

    # Discourse markers
    dm_count = sum(1 for w in words if w in _DISCOURSE_MARKERS)
    features["discourse_marker_ratio"] = dm_count / max(n_words, 1)

    return features


def compute_features_batch(df: pd.DataFrame, text_col: str = "text") -> pd.DataFrame:
    """Compute features for all rows in a DataFrame.

    Adds feature columns directly to the DataFrame (in-place modification
    of a copy).  Returns the augmented DataFrame.
    """
    logger.info("Computing features for %d texts ...", len(df))
    feature_rows = df[text_col].apply(compute_features)
    feature_df = pd.DataFrame(feature_rows.tolist(), index=df.index)
    result = pd.concat([df, feature_df], axis=1)
    logger.info("Feature computation complete.")
    return result


# ---------------------------------------------------------------------------
# Statistical helpers
# ---------------------------------------------------------------------------

def cohens_d(group_a: np.ndarray, group_b: np.ndarray) -> float:
    """Compute Cohen's d effect size between two groups."""
    n_a, n_b = len(group_a), len(group_b)
    if n_a < 2 or n_b < 2:
        return float("nan")
    mean_a, mean_b = np.nanmean(group_a), np.nanmean(group_b)
    var_a, var_b = np.nanvar(group_a, ddof=1), np.nanvar(group_b, ddof=1)
    pooled_std = math.sqrt(((n_a - 1) * var_a + (n_b - 1) * var_b) / (n_a + n_b - 2))
    if pooled_std == 0:
        return 0.0
    return (mean_a - mean_b) / pooled_std


def welch_t_test(group_a: np.ndarray, group_b: np.ndarray) -> tuple[float, float]:
    """Welch's t-test (unequal variance).  Returns (t_statistic, p_value)."""
    from scipy import stats
    a = group_a[~np.isnan(group_a)]
    b = group_b[~np.isnan(group_b)]
    if len(a) < 2 or len(b) < 2:
        return (float("nan"), float("nan"))
    stat, p = stats.ttest_ind(a, b, equal_var=False)
    return (float(stat), float(p))


def mann_whitney_u(group_a: np.ndarray, group_b: np.ndarray) -> tuple[float, float]:
    """Mann-Whitney U test. Returns (U_statistic, p_value)."""
    from scipy import stats
    a = group_a[~np.isnan(group_a)]
    b = group_b[~np.isnan(group_b)]
    if len(a) < 2 or len(b) < 2:
        return (float("nan"), float("nan"))
    stat, p = stats.mannwhitneyu(a, b, alternative="two-sided")
    return (float(stat), float(p))


def chi_squared_test(counts_a: dict, counts_b: dict) -> tuple[float, float]:
    """Chi-squared test on two frequency distributions.

    Returns (chi2_statistic, p_value).
    """
    from scipy import stats
    all_keys = sorted(set(counts_a) | set(counts_b))
    observed = np.array([[counts_a.get(k, 0) for k in all_keys],
                         [counts_b.get(k, 0) for k in all_keys]])
    # Remove columns where both are zero
    mask = observed.sum(axis=0) > 0
    observed = observed[:, mask]
    if observed.shape[1] < 2:
        return (float("nan"), float("nan"))
    stat, p, _, _ = stats.chi2_contingency(observed)
    return (float(stat), float(p))


def compare_groups(
    df: pd.DataFrame,
    feature: str,
    group_col: str = "label",
    group_a_val: str = "human",
    group_b_val: str = "ai",
) -> dict[str, Any]:
    """Compare a feature between two groups. Returns a finding dict."""
    a = df.loc[df[group_col] == group_a_val, feature].dropna().values
    b = df.loc[df[group_col] == group_b_val, feature].dropna().values

    if len(a) < MIN_GROUP_SIZE or len(b) < MIN_GROUP_SIZE:
        return {"feature": feature, "status": "insufficient_data",
                "n_a": len(a), "n_b": len(b)}

    d = cohens_d(a, b)
    t_stat, p_val = welch_t_test(a, b)

    return {
        "feature": feature,
        "status": "ok",
        "group_a": group_a_val,
        "group_b": group_b_val,
        "n_a": len(a),
        "n_b": len(b),
        "mean_a": float(np.nanmean(a)),
        "mean_b": float(np.nanmean(b)),
        "std_a": float(np.nanstd(a, ddof=1)),
        "std_b": float(np.nanstd(b, ddof=1)),
        "median_a": float(np.nanmedian(a)),
        "median_b": float(np.nanmedian(b)),
        "cohens_d": float(d),
        "abs_cohens_d": abs(float(d)),
        "t_statistic": float(t_stat),
        "p_value": float(p_val),
        "significant": p_val < ALPHA if not math.isnan(p_val) else False,
        "notable_effect": abs(d) >= MIN_EFFECT_SIZE if not math.isnan(d) else False,
    }


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------

def _shannon_entropy(freq: Counter, total: int) -> float:
    if total == 0:
        return 0.0
    entropy = 0.0
    for count in freq.values():
        if count > 0:
            p = count / total
            entropy -= p * math.log2(p)
    return entropy


def _yule_k(word_freq: Counter, n_words: int) -> float:
    """Yule's K measure of vocabulary richness."""
    if n_words == 0:
        return float("nan")
    freq_spectrum = Counter(word_freq.values())
    m2 = sum(i * i * fi for i, fi in freq_spectrum.items())
    k = 10000 * (m2 - n_words) / (n_words * n_words) if n_words > 1 else float("nan")
    return k


def _brunet_w(n_words: int, n_types: int) -> float:
    """Brunet's W measure."""
    if n_words <= 1 or n_types == 0:
        return float("nan")
    return n_words ** (n_types ** -0.172)


def _honore_r(n_words: int, n_types: int, hapax: int) -> float:
    """Honore's R statistic."""
    if n_words == 0 or n_types == 0:
        return float("nan")
    if hapax == n_types:
        hapax = n_types - 1  # avoid log(0)
    denom = 1 - (hapax / n_types) if n_types > 0 else 1
    if denom <= 0:
        return float("nan")
    return 100 * math.log(n_words) / denom


def _count_syllables(word: str) -> int:
    """Rough syllable count for English words."""
    word = word.lower().rstrip("e")
    count = len(re.findall(r'[aeiouy]+', word))
    return max(count, 1)


# ---------------------------------------------------------------------------
# Memo generation
# ---------------------------------------------------------------------------

def write_memo(
    title: str,
    content: str,
    researcher_name: str = "council",
) -> Path:
    """Write a research memo to the output directory.

    Returns the path to the written memo.
    """
    RESEARCH_OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    slug = re.sub(r"[^a-z0-9]+", "_", title.lower()).strip("_")[:60]
    filename = f"{timestamp}_{researcher_name}_{slug}.md"
    path = RESEARCH_OUTPUT_DIR / filename

    header = (
        f"# {title}\n\n"
        f"**Researcher:** {researcher_name}  \n"
        f"**Date:** {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}  \n"
        f"**Dataset:** {DATASET_PATH.name}  \n\n"
        f"---\n\n"
    )

    path.write_text(header + content, encoding="utf-8")
    logger.info("Memo written to %s", path)
    return path


# ---------------------------------------------------------------------------
# Base class
# ---------------------------------------------------------------------------

class BaseResearcher(ABC):
    """Abstract base class for all research council members."""

    name: str = "base"
    description: str = ""

    def __init__(self):
        self.logger = logging.getLogger(f"research_council.{self.name}")

    @abstractmethod
    def analyze(self, df: pd.DataFrame) -> dict[str, Any]:
        """Run analysis on the DataFrame. Return structured findings."""
        ...

    @abstractmethod
    def generate_memo(self, findings: dict[str, Any]) -> str:
        """Generate a markdown research memo from findings."""
        ...

    def run(self, sample_size: int = 0, **kwargs) -> Path:
        """Full pipeline: load data, analyze, write memo."""
        df = load_dataset(sample_size=sample_size, **kwargs)
        df = compute_features_batch(df)
        findings = self.analyze(df)
        memo_text = self.generate_memo(findings)
        return write_memo(
            title=findings.get("title", f"{self.name} Analysis"),
            content=memo_text,
            researcher_name=self.name,
        )

    def _format_finding(self, f: dict) -> str:
        """Format a single statistical finding as a bullet point."""
        if f.get("status") != "ok":
            return ""
        d = f["cohens_d"]
        direction = "higher" if d > 0 else "lower"
        sig = "***" if f["p_value"] < 0.001 else ("**" if f["p_value"] < 0.01 else ("*" if f["p_value"] < 0.05 else ""))
        pct_diff = ((f["mean_a"] - f["mean_b"]) / f["mean_b"] * 100) if f["mean_b"] != 0 else float("nan")

        return (
            f"- **{f['feature']}**: {f['group_a']} text is "
            f"{abs(pct_diff):.1f}% {direction} than {f['group_b']} "
            f"(d={d:+.3f}, p={f['p_value']:.2e}{sig}, "
            f"n={f['n_a']}+{f['n_b']})"
        )

    def _format_findings_table(self, findings_list: list[dict]) -> str:
        """Format a list of findings as a markdown table sorted by effect size."""
        ok = [f for f in findings_list if f.get("status") == "ok"]
        ok.sort(key=lambda f: abs(f.get("cohens_d", 0)), reverse=True)

        lines = [
            "| Feature | Mean (human) | Mean (AI) | Cohen's d | p-value | Notable |",
            "|---------|-------------|----------|----------|---------|---------|",
        ]
        for f in ok:
            notable = "YES" if f.get("notable_effect") else "no"
            lines.append(
                f"| {f['feature']} | {f['mean_a']:.4f} | {f['mean_b']:.4f} "
                f"| {f['cohens_d']:+.3f} | {f['p_value']:.2e} | {notable} |"
            )
        return "\n".join(lines)
