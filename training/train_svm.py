#!/usr/bin/env python3
"""Train an SVM model for authorship attribution and export to ONNX.

Usage:
    python train_svm.py --input features.csv --output models/svm_model.onnx
    python train_svm.py --input features.csv --output models/svm_model.onnx --kernel rbf --C 10

This implements the PAN competition winning approach: SVM with RBF kernel
on stylometric features with grid search for hyperparameter optimization.
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd
from sklearn.model_selection import GridSearchCV, StratifiedKFold
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.svm import SVC
from sklearn.metrics import classification_report, accuracy_score


def load_features(csv_path):
    """Load features from CSV, returning X (features), y (labels), and feature names."""
    df = pd.read_csv(csv_path)

    if "label" not in df.columns:
        print("ERROR: CSV must have a 'label' column.", file=sys.stderr)
        sys.exit(1)

    y = df["label"].values
    meta_cols = {"label", "source", "word_count"}
    feature_cols = [c for c in df.columns if c not in meta_cols]
    X = df[feature_cols].values.astype(np.float64)
    X = np.nan_to_num(X, nan=0.0, posinf=0.0, neginf=0.0)

    return X, y, feature_cols


def train_svm(X, y, kernel="rbf", C=None, gamma=None, cv_folds=5):
    """Train SVM with optional grid search."""
    pipe = Pipeline([
        ("scaler", StandardScaler()),
        ("svm", SVC(probability=True, random_state=42)),
    ])

    if C is None or gamma is None:
        param_grid = {
            "svm__C": [0.1, 1.0, 10.0, 100.0],
            "svm__gamma": ["scale", "auto", 0.01, 0.1],
            "svm__kernel": [kernel],
        }
        print(f"Running grid search with {cv_folds}-fold CV...")
        cv = StratifiedKFold(n_splits=cv_folds, shuffle=True, random_state=42)
        search = GridSearchCV(
            pipe, param_grid, cv=cv, scoring="accuracy",
            n_jobs=-1, verbose=1, refit=True,
        )
        search.fit(X, y)

        print(f"\nBest parameters: {search.best_params_}")
        print(f"Best CV accuracy: {search.best_score_:.4f}")
        return search.best_estimator_, search.best_params_, search.best_score_
    else:
        pipe.set_params(svm__C=C, svm__gamma=gamma, svm__kernel=kernel)
        pipe.fit(X, y)
        train_acc = accuracy_score(y, pipe.predict(X))
        print(f"Training accuracy: {train_acc:.4f}")
        return pipe, {"C": C, "gamma": gamma, "kernel": kernel}, train_acc


def export_onnx(model, feature_names, output_path):
    """Export trained sklearn pipeline to ONNX format."""
    try:
        from skl2onnx import convert_sklearn
        from skl2onnx.common.data_types import FloatTensorType

        initial_type = [("input", FloatTensorType([None, len(feature_names)]))]
        onnx_model = convert_sklearn(
            model, initial_types=initial_type, target_opset=15,
        )

        Path(output_path).parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "wb") as f:
            f.write(onnx_model.SerializeToString())

        print(f"ONNX model saved to {output_path}")
        return True
    except ImportError:
        print("WARNING: skl2onnx not installed, skipping ONNX export.", file=sys.stderr)
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Train SVM for authorship attribution."
    )
    parser.add_argument("--input", required=True, help="Input features CSV")
    parser.add_argument("--output", required=True, help="Output ONNX model path")
    parser.add_argument("--kernel", default="rbf", choices=["rbf", "linear", "poly"])
    parser.add_argument("--C", type=float, default=None, help="Regularization (None=grid search)")
    parser.add_argument("--gamma", type=float, default=None, help="Kernel coefficient (None=grid search)")
    parser.add_argument("--cv-folds", type=int, default=5, help="Cross-validation folds")
    parser.add_argument("--manifest", default=None, help="Path to model manifest.json")
    args = parser.parse_args()

    print("Loading features...")
    X, y, feature_names = load_features(args.input)
    n_authors = len(set(y))
    print(f"Loaded {X.shape[0]} samples, {X.shape[1]} features, {n_authors} authors")

    if n_authors < 2:
        print("ERROR: Need at least 2 authors for classification.", file=sys.stderr)
        sys.exit(1)

    model, best_params, best_score = train_svm(
        X, y, kernel=args.kernel, C=args.C, gamma=args.gamma, cv_folds=args.cv_folds,
    )

    y_pred = model.predict(X)
    print(f"\nFinal training report:")
    print(classification_report(y, y_pred))

    export_onnx(model, feature_names, args.output)

    if args.manifest:
        manifest_path = Path(args.manifest)
        if manifest_path.exists():
            with open(manifest_path) as f:
                manifest = json.load(f)
        else:
            manifest = []

        model_info = {
            "name": Path(args.output).stem,
            "version": "1.0.0",
            "model_type": "svm",
            "feature_set": "standard",
            "n_features": len(feature_names),
            "class_labels": sorted(set(y)),
            "training_accuracy": float(best_score),
            "path": str(args.output),
            "params": {k: str(v) for k, v in best_params.items()},
        }
        manifest.append(model_info)

        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        with open(manifest_path, "w") as f:
            json.dump(manifest, f, indent=2)
        print(f"Manifest updated: {manifest_path}")


if __name__ == "__main__":
    main()
