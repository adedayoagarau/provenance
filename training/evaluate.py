#!/usr/bin/env python3
"""Evaluate authorship attribution models with detailed metrics and visualizations.

Usage:
    python evaluate.py --input features.csv --model models/svm_model.onnx
    python evaluate.py --input features.csv --cv-only --cv-folds 10

Produces:
- Classification report (precision, recall, F1 per author)
- Confusion matrix heatmap
- ROC curves (one-vs-rest)
"""

import argparse
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


def main():
    parser = argparse.ArgumentParser(
        description="Evaluate authorship attribution models."
    )
    parser.add_argument("--input", required=True, help="Input features CSV")
    parser.add_argument("--model", default=None, help="ONNX model path to evaluate")
    parser.add_argument("--cv-only", action="store_true", help="Only run cross-validation")
    parser.add_argument("--cv-folds", type=int, default=5)
    parser.add_argument("--output-dir", default="results", help="Directory for output plots")
    args = parser.parse_args()

    X, y, feature_names = load_features(args.input)
    print(f"Loaded {X.shape[0]} samples, {X.shape[1]} features, {len(set(y))} authors")

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

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
