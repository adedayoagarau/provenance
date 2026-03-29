#!/usr/bin/env python3
"""Provenance AutoResearch Evaluation Harness.

This file is READ-ONLY during experiments. It provides standardized
evaluation metrics that the experiment runner uses to score domain
improvements. The agent never modifies this file.

Evaluation principles:
- All metrics computed on held-out test data
- Reproducible with fixed random seeds
- Focus on hardest cases, not average performance
- Fair comparison across experiments (same splits)
"""

import json
import math
import os
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).parent.parent))

from autoresearch.config import (
    PROVENANCE_BIN, TEST_SUITE_DIR, RANDOM_SEED,
    SPLIT_RATIOS, CV_FOLDS,
)


# ─── Data Splitting ──────────────────────────────────────────────────────────

def deterministic_split(items: list, seed: int = RANDOM_SEED) -> dict:
    """Split items into train/val/test with fixed seed.

    Uses a simple deterministic hash-based split so the same items
    always end up in the same split, regardless of order.
    """
    train, val, test = [], [], []

    for item in items:
        # Hash the item path for deterministic assignment
        h = hash(str(item) + str(seed)) % 100
        if h < SPLIT_RATIOS["train"] * 100:
            train.append(item)
        elif h < (SPLIT_RATIOS["train"] + SPLIT_RATIOS["validation"]) * 100:
            val.append(item)
        else:
            test.append(item)

    return {"train": train, "validation": val, "test": test}


# ─── Metric Functions ────────────────────────────────────────────────────────

def compute_f1(tp: int, fp: int, fn: int) -> float:
    """Compute F1 score."""
    precision = tp / max(tp + fp, 1)
    recall = tp / max(tp + fn, 1)
    if precision + recall == 0:
        return 0.0
    return 2 * (precision * recall) / (precision + recall)


def compute_auroc(scores: list[tuple[float, bool]]) -> float:
    """Compute Area Under ROC Curve.

    Args:
        scores: List of (predicted_score, is_positive) tuples
    """
    if not scores:
        return 0.5

    sorted_scores = sorted(scores, key=lambda x: x[0], reverse=True)
    total_pos = sum(1 for _, y in scores if y)
    total_neg = len(scores) - total_pos

    if total_pos == 0 or total_neg == 0:
        return 0.5

    tp = 0
    fp = 0
    auc = 0.0
    prev_fpr = 0.0

    for score, is_pos in sorted_scores:
        if is_pos:
            tp += 1
        else:
            fp += 1

        tpr = tp / total_pos
        fpr = fp / total_neg

        if fpr != prev_fpr:
            auc += tpr * (fpr - prev_fpr)
            prev_fpr = fpr

    return auc


def compute_eer(genuine_scores: list[float], impostor_scores: list[float]) -> float:
    """Compute Equal Error Rate.

    Returns the threshold where False Accept Rate = False Reject Rate.
    """
    if not genuine_scores or not impostor_scores:
        return 0.5

    all_scores = sorted(set(genuine_scores + impostor_scores))
    best_eer = 1.0

    for threshold in all_scores:
        frr = sum(1 for s in genuine_scores if s < threshold) / len(genuine_scores)
        far = sum(1 for s in impostor_scores if s >= threshold) / len(impostor_scores)

        # EER is where FAR ≈ FRR
        current_eer = (frr + far) / 2
        if abs(frr - far) < abs(best_eer * 2 - 1) + 0.01:
            best_eer = current_eer

    return best_eer


def compute_accuracy(predictions: list[bool], labels: list[bool]) -> float:
    """Simple accuracy metric."""
    if not predictions:
        return 0.0
    correct = sum(1 for p, l in zip(predictions, labels) if p == l)
    return correct / len(predictions)


def compute_confusion_matrix(predictions: list[bool], labels: list[bool]) -> dict:
    """Compute confusion matrix."""
    tp = sum(1 for p, l in zip(predictions, labels) if p and l)
    fp = sum(1 for p, l in zip(predictions, labels) if p and not l)
    tn = sum(1 for p, l in zip(predictions, labels) if not p and not l)
    fn = sum(1 for p, l in zip(predictions, labels) if not p and l)
    return {"tp": tp, "fp": fp, "tn": tn, "fn": fn}


def compute_ece(confidences: list[float], accuracies: list[bool], n_bins: int = 10) -> float:
    """Compute Expected Calibration Error.

    Measures how well predicted probabilities match actual frequencies.
    """
    if not confidences:
        return 0.0

    bins = [[] for _ in range(n_bins)]
    for conf, acc in zip(confidences, accuracies):
        bin_idx = min(int(conf * n_bins), n_bins - 1)
        bins[bin_idx].append((conf, float(acc)))

    ece = 0.0
    total = len(confidences)
    for bin_items in bins:
        if not bin_items:
            continue
        avg_conf = sum(c for c, _ in bin_items) / len(bin_items)
        avg_acc = sum(a for _, a in bin_items) / len(bin_items)
        ece += len(bin_items) / total * abs(avg_conf - avg_acc)

    return ece


# ─── Domain-Specific Evaluators ──────────────────────────────────────────────

def evaluate_ai_detection(data_dir: Path) -> dict:
    """Evaluate AI detection performance using Provenance CLI."""
    results = {"tp": 0, "fp": 0, "tn": 0, "fn": 0, "scores": []}

    test_files = list(data_dir.glob("**/*.txt"))
    split = deterministic_split(test_files)

    for fpath in split["test"]:
        # Determine ground truth
        is_ai = _is_ai_text(fpath)
        if is_ai is None:
            continue

        try:
            output = subprocess.run(
                [str(PROVENANCE_BIN), "detect", "--file", str(fpath), "--format", "json"],
                capture_output=True, text=True, timeout=30,
            )
            if output.returncode == 0:
                detection = json.loads(output.stdout)
                score = detection.get("score", {}).get("adjusted_score", 0.5)
                predicted_ai = score > 0.5

                if is_ai and predicted_ai: results["tp"] += 1
                elif is_ai and not predicted_ai: results["fn"] += 1
                elif not is_ai and predicted_ai: results["fp"] += 1
                else: results["tn"] += 1

                results["scores"].append((score, is_ai))
        except Exception:
            pass

    tp, fp, tn, fn = results["tp"], results["fp"], results["tn"], results["fn"]
    return {
        "f1_score": compute_f1(tp, fp, fn),
        "fpr": fp / max(fp + tn, 1),
        "tpr": tp / max(tp + fn, 1),
        "auroc": compute_auroc(results["scores"]),
        "total_samples": tp + fp + tn + fn,
    }


def evaluate_voice_matching(data_dir: Path) -> dict:
    """Evaluate human voice profile matching accuracy."""
    from autoresearch.domains.human_voice import build_voice_profile, compare_voice_profiles
    from collections import defaultdict

    authors = defaultdict(list)
    for author_dir in data_dir.iterdir():
        if author_dir.is_dir():
            for fpath in author_dir.glob("*.txt"):
                text = fpath.read_text(errors="replace")
                if len(text.split()) >= 200:
                    authors[author_dir.name].append(text)

    if len(authors) < 2:
        return {"profile_accuracy": 0.0, "num_authors": 0}

    # Build profiles and test matching
    profiles = {name: build_voice_profile(texts) for name, texts in authors.items()}
    correct = 0
    total = 0

    for true_author, texts in authors.items():
        if len(texts) < 2:
            continue
        # Leave-one-out
        test_text = texts[-1]
        test_profile = build_voice_profile([test_text])
        best_match = max(profiles.keys(),
                        key=lambda n: compare_voice_profiles(test_profile, profiles[n]))
        if best_match == true_author:
            correct += 1
        total += 1

    return {"profile_accuracy": correct / max(total, 1), "num_authors": len(authors)}


def evaluate_tone_analysis(data_dir: Path) -> dict:
    """Evaluate tone classification accuracy."""
    from autoresearch.domains.tone_analysis import analyze_tone

    correct = 0
    total = 0
    consistency_scores = []

    for register_dir in data_dir.iterdir():
        if not register_dir.is_dir():
            continue
        for fpath in register_dir.glob("*.txt"):
            text = fpath.read_text(errors="replace")
            if len(text.split()) < 100:
                continue

            result = analyze_tone(text)
            consistency_scores.append(result.consistency_score)
            total += 1

    return {
        "tone_accuracy": correct / max(total, 1),
        "avg_consistency": sum(consistency_scores) / max(len(consistency_scores), 1),
        "total_samples": total,
    }


# ─── Comprehensive Evaluation ────────────────────────────────────────────────

def run_full_evaluation(data_dir: Optional[Path] = None) -> dict:
    """Run evaluation across all domains and return comprehensive results."""
    if data_dir is None:
        data_dir = TEST_SUITE_DIR

    results = {}

    print("Running comprehensive evaluation...")
    print("-" * 60)

    # AI Detection
    print("  Evaluating AI detection...")
    try:
        results["ai_detection"] = evaluate_ai_detection(data_dir)
        print(f"    F1: {results['ai_detection']['f1_score']:.4f}")
    except Exception as e:
        results["ai_detection"] = {"error": str(e)}
        print(f"    Error: {e}")

    # Voice Matching
    print("  Evaluating voice matching...")
    try:
        results["human_voice"] = evaluate_voice_matching(data_dir)
        print(f"    Accuracy: {results['human_voice']['profile_accuracy']:.4f}")
    except Exception as e:
        results["human_voice"] = {"error": str(e)}
        print(f"    Error: {e}")

    # Tone Analysis
    print("  Evaluating tone analysis...")
    try:
        results["tone_analysis"] = evaluate_tone_analysis(data_dir)
        print(f"    Consistency: {results['tone_analysis']['avg_consistency']:.4f}")
    except Exception as e:
        results["tone_analysis"] = {"error": str(e)}
        print(f"    Error: {e}")

    print("-" * 60)
    return results


# ─── Helpers ─────────────────────────────────────────────────────────────────

def _is_ai_text(fpath: Path) -> Optional[bool]:
    """Determine if a file contains AI-generated text."""
    path_str = str(fpath).lower()
    if "/ai/" in path_str or "_ai_" in path_str:
        return True
    if "/human/" in path_str or "_human_" in path_str:
        return False
    meta_path = fpath.with_suffix(".meta.json")
    if meta_path.exists():
        try:
            meta = json.loads(meta_path.read_text())
            return meta.get("label") == "ai"
        except Exception:
            pass
    return None


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Run evaluation harness")
    parser.add_argument("--data-dir", type=str, default=None)
    parser.add_argument("--domain", type=str, default="all")
    parser.add_argument("--output", type=str, default=None,
                       help="Save results to JSON file")
    args = parser.parse_args()

    data_dir = Path(args.data_dir) if args.data_dir else None
    results = run_full_evaluation(data_dir)

    if args.output:
        Path(args.output).write_text(json.dumps(results, indent=2))
        print(f"\nResults saved to {args.output}")
