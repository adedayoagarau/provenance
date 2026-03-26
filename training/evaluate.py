#!/usr/bin/env python3
"""Evaluate AI detection models with detailed metrics and GO/NO-GO validation.

Supports both authorship attribution (multi-class) and AI detection (binary)
evaluation. Includes TPR@FPR metrics, ECE, Brier score, and per-register
evaluation for the ML ensemble pipeline.

Usage:
    python evaluate.py --input features.csv --model models/svm_model.onnx
    python evaluate.py --input features.csv --cv-only --cv-folds 10
    python evaluate.py --ensemble-eval --predictions preds.npy --labels labels.npy
    python evaluate.py --go-no-go --predictions preds.npy --labels labels.npy

Produces:
- Classification report (precision, recall, F1)
- Confusion matrix heatmap
- ROC curves
- TPR@2%FPR computation (primary metric)
- ECE and Brier score (calibration quality)
- GO/NO-GO gate validation
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd
from sklearn.metrics import (
    accuracy_score, classification_report, confusion_matrix,
    roc_curve, auc,
)
from sklearn.model_selection import StratifiedKFold, cross_val_predict
from sklearn.preprocessing import StandardScaler, label_binarize
from sklearn.pipeline import Pipeline
from sklearn.svm import SVC

import config


def load_features(csv_path):
    """Load features from CSV."""
    df = pd.read_csv(csv_path)
    y = df["label"].values
    meta_cols = {"label", "source", "word_count"}
    feature_cols = [c for c in df.columns if c not in meta_cols]
    X = df[feature_cols].values.astype(np.float64)
    X = np.nan_to_num(X, nan=0.0, posinf=0.0, neginf=0.0)
    return X, y, feature_cols


def evaluate_cv(X, y, cv_folds=5):
    """Run cross-validated evaluation with SVM."""
    pipe = Pipeline([
        ("scaler", StandardScaler()),
        ("svm", SVC(kernel="rbf", C=10, probability=True, random_state=42)),
    ])

    cv = StratifiedKFold(n_splits=cv_folds, shuffle=True, random_state=42)

    print(f"Running {cv_folds}-fold cross-validation...")
    y_pred = cross_val_predict(pipe, X, y, cv=cv)
    y_prob = cross_val_predict(pipe, X, y, cv=cv, method="predict_proba")

    print("\n" + "=" * 60)
    print("CLASSIFICATION REPORT")
    print("=" * 60)
    print(classification_report(y, y_pred, digits=4))

    accuracy = accuracy_score(y, y_pred)
    print(f"Overall accuracy: {accuracy:.4f}")

    return y_pred, y_prob


def plot_confusion_matrix(y_true, y_pred, output_path=None):
    """Plot and optionally save confusion matrix."""
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import seaborn as sns

        labels = sorted(set(y_true))
        cm = confusion_matrix(y_true, y_pred, labels=labels)

        fig, ax = plt.subplots(figsize=(max(8, len(labels)), max(6, len(labels) * 0.8)))
        sns.heatmap(
            cm, annot=True, fmt="d", cmap="Blues",
            xticklabels=labels, yticklabels=labels, ax=ax,
        )
        ax.set_xlabel("Predicted")
        ax.set_ylabel("Actual")
        ax.set_title("Confusion Matrix")

        if output_path:
            fig.savefig(output_path, dpi=150, bbox_inches="tight")
            print(f"Confusion matrix saved to {output_path}")
        plt.close(fig)
    except ImportError:
        print("WARNING: matplotlib/seaborn not available, skipping plot.", file=sys.stderr)


def plot_roc_curves(y_true, y_prob, output_path=None):
    """Plot ROC curves (one-vs-rest)."""
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt

        classes = sorted(set(y_true))
        if len(classes) < 2:
            return

        y_bin = label_binarize(y_true, classes=classes)

        fig, ax = plt.subplots(figsize=(10, 8))
        for i, cls in enumerate(classes):
            if y_bin.shape[1] > 1:
                fpr, tpr, _ = roc_curve(y_bin[:, i], y_prob[:, i])
            else:
                fpr, tpr, _ = roc_curve(y_bin.ravel(), y_prob[:, 1])

            roc_auc = auc(fpr, tpr)
            ax.plot(fpr, tpr, label=f"{cls} (AUC = {roc_auc:.3f})")

        ax.plot([0, 1], [0, 1], "k--", alpha=0.5)
        ax.set_xlabel("False Positive Rate")
        ax.set_ylabel("True Positive Rate")
        ax.set_title("ROC Curves")
        ax.legend(loc="lower right")

        if output_path:
            fig.savefig(output_path, dpi=150, bbox_inches="tight")
            print(f"ROC curves saved to {output_path}")
        plt.close(fig)
    except ImportError:
        print("WARNING: matplotlib not available, skipping ROC plot.", file=sys.stderr)


def evaluate_onnx_model(model_path, X, y):
    """Evaluate an ONNX model."""
    try:
        import onnxruntime as ort

        sess = ort.InferenceSession(str(model_path))
        input_name = sess.get_inputs()[0].name

        y_pred = sess.run(None, {input_name: X.astype(np.float32)})[0]

        print(f"\nONNX model evaluation ({model_path}):")
        print(classification_report(y, y_pred, digits=4))
        print(f"Accuracy: {accuracy_score(y, y_pred):.4f}")

        return y_pred
    except ImportError:
        print("ERROR: onnxruntime not installed.", file=sys.stderr)
        return None


def compute_eer(y_true, y_prob):
    """Compute Equal Error Rate."""
    classes = sorted(set(y_true))
    if len(classes) != 2:
        print("EER computation requires exactly 2 classes (binary).")
        return None

    y_bin = label_binarize(y_true, classes=classes).ravel()
    fpr, tpr, _ = roc_curve(y_bin, y_prob[:, 1])
    fnr = 1 - tpr

    eer_idx = np.nanargmin(np.abs(fpr - fnr))
    eer = (fpr[eer_idx] + fnr[eer_idx]) / 2

    print(f"\nEqual Error Rate (EER): {eer:.4f}")
    return eer


def tpr_at_fpr(y_true, y_scores, target_fpr=0.02):
    """Compute TPR at a specific FPR threshold."""
    fpr, tpr, _ = roc_curve(y_true, y_scores)
    idx = np.argmin(np.abs(fpr - target_fpr))
    return float(tpr[idx])


def expected_calibration_error(y_true, y_prob, n_bins=10):
    """Compute Expected Calibration Error (ECE)."""
    bin_boundaries = np.linspace(0, 1, n_bins + 1)
    ece = 0.0
    for lower, upper in zip(bin_boundaries[:-1], bin_boundaries[1:]):
        in_bin = (y_prob > lower) & (y_prob <= upper)
        prop = in_bin.mean()
        if prop > 0:
            accuracy = y_true[in_bin].mean()
            confidence = y_prob[in_bin].mean()
            ece += prop * abs(accuracy - confidence)
    return float(ece)


def brier_score(y_true, y_prob):
    """Compute Brier score (MSE of probability estimates)."""
    return float(np.mean((y_prob - y_true) ** 2))


def evaluate_ensemble(predictions, labels, output_dir=None):
    """Evaluate ensemble predictions with AI detection metrics."""
    print("\n" + "=" * 60)
    print("ENSEMBLE EVALUATION — AI Detection")
    print("=" * 60)

    fpr_arr, tpr_arr, _ = roc_curve(labels, predictions)
    roc_auc = auc(fpr_arr, tpr_arr)
    tpr_2pct = tpr_at_fpr(labels, predictions, target_fpr=0.02)
    ece = expected_calibration_error(labels, predictions)
    brier = brier_score(labels, predictions)

    print(f"\n  AUC:           {roc_auc:.4f}")
    print(f"  TPR@2%FPR:     {tpr_2pct:.4f} (target: >= {config.TARGET_TPR_MIN})")
    print(f"  ECE:           {ece:.4f} (target: < {config.TARGET_ECE_MAX})")
    print(f"  Brier Score:   {brier:.4f}")

    # Additional FPR operating points
    for target in [0.01, 0.02, 0.05, 0.10]:
        t = tpr_at_fpr(labels, predictions, target_fpr=target)
        print(f"  TPR@{target*100:.0f}%FPR:    {t:.4f}")

    results = {
        "auc": roc_auc,
        "tpr_at_2pct_fpr": tpr_2pct,
        "ece": ece,
        "brier_score": brier,
    }

    if output_dir:
        output_dir = Path(output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)
        with open(output_dir / "ensemble_metrics.json", "w") as f:
            json.dump(results, f, indent=2)

    return results


def evaluate_go_no_go(predictions, labels):
    """Run GO/NO-GO gate validation."""
    print("\n" + "=" * 60)
    print("GO/NO-GO GATE VALIDATION")
    print("=" * 60)

    tpr_2pct = tpr_at_fpr(labels, predictions, target_fpr=0.02)
    ece = expected_calibration_error(labels, predictions)

    gate1 = tpr_2pct >= config.TARGET_TPR_MIN
    gate2 = ece < config.TARGET_ECE_MAX

    print(f"\n  Gate 1: TPR@2%FPR >= {config.TARGET_TPR_MIN*100:.0f}%")
    print(f"          Actual: {tpr_2pct*100:.2f}% — {'PASS' if gate1 else 'FAIL'}")
    print(f"\n  Gate 2: ECE < {config.TARGET_ECE_MAX}")
    print(f"          Actual: {ece:.4f} — {'PASS' if gate2 else 'FAIL'}")

    if gate1 and gate2:
        print("\n  RESULT: GO — Proceed to Phase 3 (Production Engineering)")
    else:
        print("\n  RESULT: NO-GO — DO NOT proceed to Phase 3")
        if not gate1:
            print("  ACTION: Improve model discrimination (more data, features, or tuning)")
        if not gate2:
            print("  ACTION: Improve calibration (adjust temperature, retrain isotonic)")

    return gate1 and gate2


def main():
    parser = argparse.ArgumentParser(
        description="Evaluate authorship and AI detection models."
    )
    parser.add_argument("--input", default=None, help="Input features CSV (for authorship eval)")
    parser.add_argument("--model", default=None, help="ONNX model path to evaluate")
    parser.add_argument("--cv-only", action="store_true", help="Only run cross-validation")
    parser.add_argument("--cv-folds", type=int, default=5)
    parser.add_argument("--output-dir", default="results", help="Directory for output plots")
    parser.add_argument("--ensemble-eval", action="store_true", help="Evaluate ensemble predictions")
    parser.add_argument("--go-no-go", action="store_true", help="Run GO/NO-GO gate validation")
    parser.add_argument("--predictions", default=None, help="Path to predictions .npy (for ensemble/go-no-go)")
    parser.add_argument("--labels", default=None, help="Path to labels .npy (for ensemble/go-no-go)")
    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Ensemble evaluation mode
    if args.ensemble_eval or args.go_no_go:
        if not args.predictions or not args.labels:
            print("ERROR: --predictions and --labels required for ensemble/go-no-go evaluation")
            sys.exit(1)

        predictions = np.load(args.predictions)
        labels = np.load(args.labels)

        if args.ensemble_eval:
            evaluate_ensemble(predictions, labels, output_dir)

        if args.go_no_go:
            passed = evaluate_go_no_go(predictions, labels)
            sys.exit(0 if passed else 1)

        return

    # Original authorship evaluation mode
    if not args.input:
        print("ERROR: --input required for authorship evaluation")
        sys.exit(1)

    X, y, feature_names = load_features(args.input)
    print(f"Loaded {X.shape[0]} samples, {X.shape[1]} features, {len(set(y))} classes")

    if args.cv_only or args.model is None:
        y_pred, y_prob = evaluate_cv(X, y, args.cv_folds)
        plot_confusion_matrix(y, y_pred, output_dir / "confusion_matrix.png")
        plot_roc_curves(y, y_prob, output_dir / "roc_curves.png")
        if len(set(y)) == 2:
            compute_eer(y, y_prob)
    else:
        y_pred = evaluate_onnx_model(args.model, X, y)
        if y_pred is not None:
            plot_confusion_matrix(y, y_pred, output_dir / "confusion_matrix_onnx.png")


if __name__ == "__main__":
    main()
