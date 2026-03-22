#!/usr/bin/env python3
"""Train a gradient boosting ensemble for authorship attribution and export to ONNX.

Usage:
    python train_ensemble.py --input features.csv --output models/ensemble_model.onnx

Gradient boosting often outperforms SVM on larger datasets and provides
feature importance rankings useful for interpretability.
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd
from sklearn.ensemble import GradientBoostingClassifier, RandomForestClassifier
from sklearn.model_selection import GridSearchCV, StratifiedKFold
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import classification_report, accuracy_score


def load_features(csv_path):
    """Load features from CSV."""
    df = pd.read_csv(csv_path)
    y = df["label"].values
    meta_cols = {"label", "source", "word_count"}
    feature_cols = [c for c in df.columns if c not in meta_cols]
    X = df[feature_cols].values.astype(np.float64)
    X = np.nan_to_num(X, nan=0.0, posinf=0.0, neginf=0.0)
    return X, y, feature_cols


def train_gradient_boosting(X, y, cv_folds=5):
    """Train Gradient Boosting with grid search."""
    pipe = Pipeline([
        ("scaler", StandardScaler()),
        ("gb", GradientBoostingClassifier(random_state=42)),
    ])

    param_grid = {
        "gb__n_estimators": [100, 200],
        "gb__max_depth": [3, 5, 7],
        "gb__learning_rate": [0.01, 0.1, 0.2],
        "gb__subsample": [0.8, 1.0],
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


def train_random_forest(X, y, cv_folds=5):
    """Train Random Forest with grid search."""
    pipe = Pipeline([
        ("scaler", StandardScaler()),
        ("rf", RandomForestClassifier(random_state=42)),
    ])

    param_grid = {
        "rf__n_estimators": [100, 200, 500],
        "rf__max_depth": [None, 10, 20],
        "rf__min_samples_split": [2, 5],
        "rf__max_features": ["sqrt", "log2"],
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


def print_feature_importance(model, feature_names, top_n=20):
    """Print top feature importances from the ensemble model."""
    clf = None
    for name, step in model.steps:
        if hasattr(step, "feature_importances_"):
            clf = step
            break

    if clf is None:
        print("No feature importances available.")
        return

    importances = clf.feature_importances_
    indices = np.argsort(importances)[::-1][:top_n]

    print(f"\nTop {top_n} features by importance:")
    for rank, idx in enumerate(indices, 1):
        print(f"  {rank:2d}. {feature_names[idx]:30s} {importances[idx]:.4f}")


def export_onnx(model, feature_names, output_path):
    """Export trained sklearn pipeline to ONNX format."""
    try:
        from skl2onnx import convert_sklearn
        from skl2onnx.common.data_types import FloatTensorType

        initial_type = [("input", FloatTensorType([None, len(feature_names)]))]
        onnx_model = convert_sklearn(model, initial_types=initial_type, target_opset=15)

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
        description="Train gradient boosting / random forest for authorship attribution."
    )
    parser.add_argument("--input", required=True, help="Input features CSV")
    parser.add_argument("--output", required=True, help="Output ONNX model path")
    parser.add_argument(
        "--model", default="gradient_boosting",
        choices=["gradient_boosting", "random_forest"],
        help="Ensemble model type",
    )
    parser.add_argument("--cv-folds", type=int, default=5, help="Cross-validation folds")
    parser.add_argument("--manifest", default=None, help="Path to model manifest.json")
    args = parser.parse_args()

    print("Loading features...")
    X, y, feature_names = load_features(args.input)
    n_authors = len(set(y))
    print(f"Loaded {X.shape[0]} samples, {X.shape[1]} features, {n_authors} authors")

    if n_authors < 2:
        print("ERROR: Need at least 2 authors.", file=sys.stderr)
        sys.exit(1)

    if args.model == "gradient_boosting":
        model, best_params, best_score = train_gradient_boosting(X, y, args.cv_folds)
    else:
        model, best_params, best_score = train_random_forest(X, y, args.cv_folds)

    y_pred = model.predict(X)
    print(f"\nTraining report:")
    print(classification_report(y, y_pred))
    print_feature_importance(model, feature_names)

    export_onnx(model, feature_names, args.output)

    if args.manifest:
        manifest_path = Path(args.manifest)
        manifest = []
        if manifest_path.exists():
            with open(manifest_path) as f:
                manifest = json.load(f)

        model_info = {
            "name": Path(args.output).stem,
            "version": "1.0.0",
            "model_type": args.model,
            "feature_set": "standard",
            "n_features": len(feature_names),
            "class_labels": sorted(set(y)),
            "training_accuracy": float(best_score),
            "path": str(args.output),
        }
        manifest.append(model_info)

        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        with open(manifest_path, "w") as f:
            json.dump(manifest, f, indent=2)
        print(f"Manifest updated: {manifest_path}")


if __name__ == "__main__":
    main()
