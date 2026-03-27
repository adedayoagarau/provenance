#!/usr/bin/env python3
"""Feature extraction pipeline for the Provenance ML training dataset.

Calls `provenance detect` and `provenance analyze` on each document to
extract detection features (7 tier-1 + tier-2 + advanced) and 740+
stylometric features. Caches all results to HDF5.

Usage:
    python features.py --input data/raw/combined.jsonl
    python features.py --input data/raw/combined.jsonl --sample 100
    python features.py --input data/raw/combined.jsonl --workers 8

See ML_PIPELINE_IMPLEMENTATION_GUIDE.md Part 2 for feature details.
"""

import argparse
import json
import logging
import subprocess
import sys
import tempfile
from multiprocessing import Pool, cpu_count
from pathlib import Path

import h5py
import numpy as np
from tqdm import tqdm

import config

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)

# ─── Tier-1 Detection Feature Names ──────────────────────────────────────────

TIER1_FEATURES = [
    "burstiness_coefficient",
    "zipf_deviation",
    "hedge_ratio",
    "autocorrelation_lag1",
    "pos_trigram_entropy",
    "diversity_length_correlation",
    "repetition_position_delta",
]

# Additional features from tier-2 and advanced tiers
TIER2_FEATURES = [
    "passive_clustering",
    "transition_density",
    "paragraph_length_cv",
    "sentence_opening_diversity",
    "nested_clause_ratio",
    "comma_splice_ratio",
]

ADVANCED_FEATURES = [
    "vocab_sophistication_slope",
    "register_consistency",
    "initial_adverb_ratio",
    "modal_density",
    "punctuation_diversity",
    "enumeration_ratio",
    "paragraph_transition_overlap",
]

ALL_DETECTION_FEATURES = TIER1_FEATURES + TIER2_FEATURES + ADVANCED_FEATURES


def _run_provenance_detect(text: str) -> dict | None:
    """Run `provenance detect` on text and return parsed JSON features."""
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".txt", delete=False, encoding="utf-8"
        ) as f:
            f.write(text)
            tmp_path = f.name

        result = subprocess.run(
            [config.PROVENANCE_BIN, "detect", "--file", tmp_path, "--format", "json"],
            capture_output=True,
            text=True,
            timeout=30,
        )

        Path(tmp_path).unlink(missing_ok=True)

        if result.returncode != 0:
            logger.warning("detect returned %d: %s", result.returncode, result.stderr[:500])
            return None

        if not result.stdout.strip():
            logger.warning("detect returned empty stdout. stderr: %s", result.stderr[:500])
            return None

        # The provenance binary prints INFO log lines to stdout before
        # the JSON output. Strip everything before the first '{'.
        stdout = result.stdout
        json_start = stdout.find("{")
        if json_start == -1:
            logger.warning("detect: no JSON found in stdout: %s", stdout[:200])
            return None

        data = json.loads(stdout[json_start:])

        # The CLI wraps output in a multi-mode envelope:
        # {"mode": "AiDetection", "detection": {"score": {...}, "features": {...}}}
        # Unwrap to get the detection result directly.
        if "detection" in data:
            return data["detection"]
        return data

    except (subprocess.TimeoutExpired, json.JSONDecodeError, OSError) as e:
        logger.warning("detect failed: %s", e)
        return None


def _run_provenance_analyze(text: str) -> dict | None:
    """Run `provenance analyze` on text and return parsed JSON features."""
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".txt", delete=False, encoding="utf-8"
        ) as f:
            f.write(text)
            tmp_path = f.name

        result = subprocess.run(
            [config.PROVENANCE_BIN, "analyze", "--file", tmp_path, "--format", "json"],
            capture_output=True,
            text=True,
            timeout=60,
        )

        Path(tmp_path).unlink(missing_ok=True)

        if result.returncode != 0:
            logger.warning("analyze returned %d: %s", result.returncode, result.stderr[:500])
            return None

        if not result.stdout.strip():
            logger.warning("analyze returned empty stdout. stderr: %s", result.stderr[:500])
            return None

        # Strip INFO log lines before the JSON
        stdout = result.stdout
        json_start = stdout.find("{")
        if json_start == -1:
            logger.warning("analyze: no JSON found in stdout: %s", stdout[:200])
            return None

        return json.loads(stdout[json_start:])

    except (subprocess.TimeoutExpired, json.JSONDecodeError, OSError) as e:
        logger.warning("analyze failed: %s", e)
        return None


def _extract_detection_features(detect_result: dict) -> dict[str, float]:
    """Parse detection JSON into a flat feature dict."""
    features = {}

    # Tier 1 features from the 'features' key
    raw = detect_result.get("features", {})

    if b := raw.get("burstiness"):
        features["burstiness_coefficient"] = b.get("coefficient", float("nan"))

    if z := raw.get("zipf"):
        features["zipf_deviation"] = z.get("deviation", float("nan"))

    if h := raw.get("hedge_ratio"):
        features["hedge_ratio"] = h.get("ratio", float("nan"))

    if a := raw.get("autocorrelation"):
        features["autocorrelation_lag1"] = a.get("lag1_autocorrelation", float("nan"))

    if p := raw.get("pos_entropy"):
        features["pos_trigram_entropy"] = p.get("trigram_entropy", float("nan"))

    if i := raw.get("interaction"):
        features["diversity_length_correlation"] = i.get("diversity_length_correlation", float("nan"))
        features["repetition_position_delta"] = i.get("repetition_position_delta", float("nan"))

    # Tier 2
    if t2 := raw.get("tier2"):
        for key in TIER2_FEATURES:
            features[key] = t2.get(key, float("nan"))

    # Advanced
    if adv := raw.get("advanced"):
        for key in ADVANCED_FEATURES:
            features[key] = adv.get(key, float("nan"))

    return features


def _extract_stylometric_features(analyze_result: dict) -> dict[str, float]:
    """Parse analyze JSON into a flat feature dict for stylometric features."""
    features = {}

    # The analyze output contains feature vectors under various keys
    # depending on the analysis mode. Extract all numeric values.
    analysis = analyze_result.get("analysis", analyze_result)

    # Function word frequencies
    if fw := analysis.get("function_words"):
        for word, freq in fw.items():
            features[f"fw_{word}"] = float(freq)

    # Lexical metrics
    if lex := analysis.get("lexical"):
        for key, val in lex.items():
            if isinstance(val, (int, float)):
                features[f"lex_{key}"] = float(val)

    # Syntactic metrics
    if syn := analysis.get("syntactic"):
        for key, val in syn.items():
            if isinstance(val, (int, float)):
                features[f"syn_{key}"] = float(val)

    # Readability
    if read := analysis.get("readability"):
        for key, val in read.items():
            if isinstance(val, (int, float)):
                features[f"read_{key}"] = float(val)

    # Character n-grams
    for n in [2, 3, 4, 5]:
        key = f"char_{n}grams"
        if ngrams := analysis.get(key):
            for gram, freq in list(ngrams.items())[:100]:
                features[f"c{n}g_{gram}"] = float(freq)

    # Word bigrams
    if wbg := analysis.get("word_bigrams"):
        for gram, freq in list(wbg.items())[:100]:
            features[f"wbg_{gram}"] = float(freq)

    return features


def extract_single(doc: dict) -> dict[str, float] | None:
    """Extract all features from a single document.

    Returns a flat dict mapping feature names to values, or None on failure.
    """
    text = doc.get("text", "")
    word_count = len(text.split())
    if word_count < 500:
        print(f"  SKIP: doc has {word_count} words (need 500+)", flush=True)
        return None

    print(f"  Processing doc ({word_count} words, source={doc.get('source', '?')})...", flush=True)
    features = {}

    # Detection features (tier-1 through advanced)
    detect_result = _run_provenance_detect(text)
    if detect_result:
        det_features = _extract_detection_features(detect_result)
        features.update(det_features)
        print(f"    detect: {len(det_features)} features", flush=True)
    else:
        print(f"    detect: FAILED", flush=True)

    # Stylometric features (740+)
    analyze_result = _run_provenance_analyze(text)
    if analyze_result:
        sty_features = _extract_stylometric_features(analyze_result)
        features.update(sty_features)
        print(f"    analyze: {len(sty_features)} features", flush=True)
    else:
        print(f"    analyze: FAILED", flush=True)

    if not features:
        print(f"    RESULT: no features extracted", flush=True)
        return None

    print(f"    RESULT: {len(features)} total features", flush=True)
    return features


def _worker(args: tuple) -> tuple[int, dict[str, float] | None, str, int]:
    """Multiprocessing worker for parallel feature extraction."""
    idx, doc = args
    features = extract_single(doc)
    label = doc.get("label", "unknown")
    word_count = doc.get("word_count", 0)
    return idx, features, label, word_count


def extract_all(
    input_path: Path,
    output_path: Path,
    workers: int = 4,
    sample: int | None = None,
):
    """Extract features from all documents and save to HDF5.

    Args:
        input_path: Path to JSONL input file.
        output_path: Path to output HDF5 file.
        workers: Number of parallel workers.
        sample: If set, only process this many documents.
    """
    # Load documents
    logger.info("Loading documents from %s...", input_path)
    all_docs = []
    with open(input_path, "r") as f:
        for line in f:
            try:
                all_docs.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    # Filter to docs with enough words for provenance (minimum 500)
    all_docs = [d for d in all_docs if len(d.get("text", "").split()) >= 500]
    logger.info("Found %d documents with 500+ words", len(all_docs))

    if sample and len(all_docs) > sample:
        import random
        all_docs = random.sample(all_docs, sample)

    docs = all_docs
    logger.info("Processing %d documents", len(docs))

    # Extract features in parallel
    logger.info("Extracting features with %d workers...", workers)
    all_features: list[dict[str, float]] = [{}] * len(docs)
    labels: list[str] = [""] * len(docs)
    word_counts: list[int] = [0] * len(docs)
    failed = 0

    with Pool(workers) as pool:
        tasks = [(i, doc) for i, doc in enumerate(docs)]
        for idx, features, label, wc in tqdm(
            pool.imap_unordered(_worker, tasks),
            total=len(tasks),
            desc="Extracting features",
        ):
            if features is None:
                failed += 1
            else:
                all_features[idx] = features
                labels[idx] = label
                word_counts[idx] = wc

    logger.info("Extraction complete. %d succeeded, %d failed.", len(docs) - failed, failed)

    # Build unified feature name list (union of all observed features)
    all_names: set[str] = set()
    for f in all_features:
        all_names.update(f.keys())

    feature_names = sorted(all_names)
    logger.info("Total unique features: %d", len(feature_names))

    # Build feature matrix
    n_docs = len(docs)
    n_features = len(feature_names)
    matrix = np.full((n_docs, n_features), np.nan, dtype=np.float64)

    name_to_idx = {name: i for i, name in enumerate(feature_names)}
    for i, features in enumerate(all_features):
        for name, val in features.items():
            matrix[i, name_to_idx[name]] = val

    # Remove rows that are all NaN (failed extractions)
    valid_mask = ~np.all(np.isnan(matrix), axis=1)
    matrix = matrix[valid_mask]
    valid_labels = [l for l, v in zip(labels, valid_mask) if v]
    valid_wc = [w for w, v in zip(word_counts, valid_mask) if v]

    logger.info("Final matrix shape: %s", matrix.shape)

    # Save to HDF5
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with h5py.File(output_path, "w") as hf:
        hf.create_dataset("features", data=matrix, compression="gzip")
        hf.create_dataset("labels", data=np.array(valid_labels, dtype="S20"))
        hf.create_dataset("word_counts", data=np.array(valid_wc, dtype=np.int32))
        hf.attrs["feature_names"] = json.dumps(feature_names)
        hf.attrs["n_documents"] = matrix.shape[0]
        hf.attrs["n_features"] = matrix.shape[1]

    # Also save feature names as JSON for downstream use
    names_path = output_path.parent / "all_feature_names.json"
    with open(names_path, "w") as f:
        json.dump(feature_names, f, indent=2)

    logger.info("Saved to %s (%d docs, %d features)", output_path, matrix.shape[0], matrix.shape[1])

    return matrix, feature_names, valid_labels


def main():
    parser = argparse.ArgumentParser(
        description="Extract features from documents using provenance CLI."
    )
    parser.add_argument(
        "--input", required=True, help="Input JSONL file"
    )
    parser.add_argument(
        "--output", default=str(config.FEATURES_DIR / "features.h5"),
        help="Output HDF5 file",
    )
    parser.add_argument(
        "--workers", type=int, default=min(4, cpu_count()),
        help="Number of parallel workers",
    )
    parser.add_argument(
        "--sample", type=int, default=None,
        help="Only process this many documents (for testing)",
    )
    args = parser.parse_args()

    extract_all(
        Path(args.input),
        Path(args.output),
        workers=args.workers,
        sample=args.sample,
    )


if __name__ == "__main__":
    main()
