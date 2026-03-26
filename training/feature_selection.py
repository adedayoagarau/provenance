#!/usr/bin/env python3
"""5-stage feature selection pipeline for the Provenance ML dataset.

Reduces the full feature set (700+ features) to 50-75 high-quality features
through a rigorous selection process:

  1. Correlation filtering (remove one from pairs with r > 0.95)
  2. Mutual information (keep top 200 by MI score)
  3. LASSO with cross-validated alpha
  4. RFE with SVM (eliminate 10% per iteration)
  5. Stability selection (bootstrap 100x, keep features in >80% of runs)

Usage:
    python feature_selection.py --input features/features.h5
    python feature_selection.py --input features/features.h5 --target-min 50 --target-max 75

See ML_PIPELINE_IMPLEMENTATION_GUIDE.md Part 2 for full specification.
"""

import argparse
import json
import logging
from pathlib import Path

import h5py
import numpy as np
from sklearn.feature_selection import (
    RFE,
    mutual_info_classif,
)
from sklearn.linear_model import LassoCV
from sklearn.preprocessing import StandardScaler
from sklearn.svm import SVC

import config

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)


def load_features(h5_path: Path) -> tuple[np.ndarray, np.ndarray, list[str]]:
    """Load feature matrix, labels, and names from HDF5."""
    with h5py.File(h5_path, "r") as hf:
        X = hf["features"][:]
        labels_raw = hf["labels"][:]
        feature_names = json.loads(hf.attrs["feature_names"])

    # Convert labels to binary: human=0, ai/humanized=1
    y = np.array([0 if l.decode().strip() == "human" else 1 for l in labels_raw])

    # Replace NaN with column medians
    col_medians = np.nanmedian(X, axis=0)
    nan_mask = np.isnan(X)
    for j in range(X.shape[1]):
        X[nan_mask[:, j], j] = col_medians[j] if not np.isnan(col_medians[j]) else 0.0

    logger.info("Loaded: %d samples, %d features", X.shape[0], X.shape[1])
    return X, y, feature_names


# ─── Stage 1: Correlation Filtering ──────────────────────────────────────────


def stage1_correlation_filter(
    X: np.ndarray,
    feature_names: list[str],
    threshold: float = 0.95,
) -> tuple[np.ndarray, list[str], dict]:
    """Remove one feature from each highly correlated pair."""
    logger.info("Stage 1: Correlation filtering (threshold=%.2f)...", threshold)

    corr = np.corrcoef(X, rowvar=False)
    n = corr.shape[0]
    to_remove: set[int] = set()
    pairs_removed = []

    for i in range(n):
        if i in to_remove:
            continue
        for j in range(i + 1, n):
            if j in to_remove:
                continue
            if abs(corr[i, j]) > threshold:
                # Remove the feature with lower variance
                if np.var(X[:, i]) < np.var(X[:, j]):
                    to_remove.add(i)
                    pairs_removed.append((feature_names[i], feature_names[j], float(corr[i, j])))
                else:
                    to_remove.add(j)
                    pairs_removed.append((feature_names[j], feature_names[i], float(corr[i, j])))

    keep_mask = [i not in to_remove for i in range(n)]
    X_filtered = X[:, keep_mask]
    names_filtered = [n for i, n in enumerate(feature_names) if keep_mask[i]]

    report = {
        "stage": "correlation_filter",
        "threshold": threshold,
        "input_features": len(feature_names),
        "removed": len(to_remove),
        "output_features": len(names_filtered),
        "pairs_removed": len(pairs_removed),
    }

    logger.info(
        "  Removed %d features (%d correlated pairs). %d → %d",
        len(to_remove), len(pairs_removed), len(feature_names), len(names_filtered),
    )
    return X_filtered, names_filtered, report


# ─── Stage 2: Mutual Information ──────────────────────────────────────────────


def stage2_mutual_information(
    X: np.ndarray,
    y: np.ndarray,
    feature_names: list[str],
    top_k: int = 200,
) -> tuple[np.ndarray, list[str], dict]:
    """Keep top-K features by mutual information score."""
    logger.info("Stage 2: Mutual information (top-%d)...", top_k)

    mi_scores = mutual_info_classif(X, y, random_state=42, n_neighbors=5)
    top_indices = np.argsort(mi_scores)[::-1][:top_k]
    top_indices = np.sort(top_indices)  # Preserve original order

    X_selected = X[:, top_indices]
    names_selected = [feature_names[i] for i in top_indices]
    mi_ranking = {feature_names[i]: float(mi_scores[i]) for i in top_indices[:20]}

    report = {
        "stage": "mutual_information",
        "top_k": top_k,
        "input_features": len(feature_names),
        "output_features": len(names_selected),
        "top_20_mi_scores": mi_ranking,
    }

    logger.info("  Selected top %d by MI. %d → %d", top_k, len(feature_names), len(names_selected))
    return X_selected, names_selected, report


# ─── Stage 3: LASSO ──────────────────────────────────────────────────────────


def stage3_lasso(
    X: np.ndarray,
    y: np.ndarray,
    feature_names: list[str],
    cv_folds: int = 5,
) -> tuple[np.ndarray, list[str], dict]:
    """Use LASSO with cross-validated alpha for feature selection."""
    logger.info("Stage 3: LASSO with %d-fold CV...", cv_folds)

    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)

    lasso = LassoCV(cv=cv_folds, random_state=42, max_iter=10000)
    lasso.fit(X_scaled, y)

    # Keep features with non-zero coefficients
    nonzero_mask = np.abs(lasso.coef_) > 1e-8
    X_selected = X[:, nonzero_mask]
    names_selected = [n for n, keep in zip(feature_names, nonzero_mask) if keep]

    report = {
        "stage": "lasso",
        "cv_folds": cv_folds,
        "optimal_alpha": float(lasso.alpha_),
        "input_features": len(feature_names),
        "output_features": len(names_selected),
        "nonzero_coefficients": int(nonzero_mask.sum()),
    }

    logger.info(
        "  LASSO alpha=%.6f. %d → %d features (non-zero coefficients)",
        lasso.alpha_, len(feature_names), len(names_selected),
    )
    return X_selected, names_selected, report


# ─── Stage 4: RFE with SVM ───────────────────────────────────────────────────


def stage4_rfe(
    X: np.ndarray,
    y: np.ndarray,
    feature_names: list[str],
    target_features: int = 75,
    step_fraction: float = 0.10,
) -> tuple[np.ndarray, list[str], dict]:
    """Recursive Feature Elimination with SVM, eliminating 10% per iteration."""
    logger.info("Stage 4: RFE with SVM (target=%d, step=%.0f%%)...", target_features, step_fraction * 100)

    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)

    step = max(1, int(len(feature_names) * step_fraction))
    svm = SVC(kernel="linear", C=1.0, random_state=42)
    rfe = RFE(
        estimator=svm,
        n_features_to_select=min(target_features, len(feature_names)),
        step=step,
    )
    rfe.fit(X_scaled, y)

    selected_mask = rfe.support_
    X_selected = X[:, selected_mask]
    names_selected = [n for n, keep in zip(feature_names, selected_mask) if keep]
    rankings = {n: int(r) for n, r in zip(feature_names, rfe.ranking_)}

    report = {
        "stage": "rfe_svm",
        "target_features": target_features,
        "step": step,
        "input_features": len(feature_names),
        "output_features": len(names_selected),
        "top_10_rankings": dict(sorted(rankings.items(), key=lambda x: x[1])[:10]),
    }

    logger.info("  RFE selected %d features. %d → %d", len(names_selected), len(feature_names), len(names_selected))
    return X_selected, names_selected, report


# ─── Stage 5: Stability Selection ────────────────────────────────────────────


def stage5_stability_selection(
    X: np.ndarray,
    y: np.ndarray,
    feature_names: list[str],
    n_bootstrap: int = 100,
    threshold: float = 0.80,
) -> tuple[np.ndarray, list[str], dict]:
    """Bootstrap stability selection — keep features selected in >80% of runs."""
    logger.info("Stage 5: Stability selection (%d bootstraps, threshold=%.0f%%)...", n_bootstrap, threshold * 100)

    n_samples = X.shape[0]
    n_features = X.shape[1]
    selection_counts = np.zeros(n_features, dtype=int)

    scaler = StandardScaler()

    for i in range(n_bootstrap):
        # Bootstrap sample (with replacement)
        indices = np.random.choice(n_samples, size=n_samples, replace=True)
        X_boot = X[indices]
        y_boot = y[indices]

        X_boot_scaled = scaler.fit_transform(X_boot)

        # Use LASSO for fast feature selection
        lasso = LassoCV(cv=3, random_state=i, max_iter=5000)
        lasso.fit(X_boot_scaled, y_boot)

        selected = np.abs(lasso.coef_) > 1e-8
        selection_counts += selected.astype(int)

        if (i + 1) % 20 == 0:
            logger.info("  Bootstrap %d/%d complete", i + 1, n_bootstrap)

    # Keep features selected in > threshold fraction of runs
    selection_rates = selection_counts / n_bootstrap
    stable_mask = selection_rates >= threshold

    X_selected = X[:, stable_mask]
    names_selected = [n for n, keep in zip(feature_names, stable_mask) if keep]
    stability_scores = {
        n: float(r) for n, r in zip(feature_names, selection_rates) if r >= threshold
    }

    report = {
        "stage": "stability_selection",
        "n_bootstrap": n_bootstrap,
        "threshold": threshold,
        "input_features": len(feature_names),
        "output_features": len(names_selected),
        "stability_scores": dict(sorted(stability_scores.items(), key=lambda x: -x[1])),
    }

    logger.info(
        "  %d features stable (>%.0f%% selection rate). %d → %d",
        len(names_selected), threshold * 100, len(feature_names), len(names_selected),
    )
    return X_selected, names_selected, report


# ─── Full Pipeline ────────────────────────────────────────────────────────────


def run_selection_pipeline(
    h5_path: Path,
    output_dir: Path,
    target_min: int = 50,
    target_max: int = 75,
):
    """Run the complete 5-stage feature selection pipeline.

    Outputs:
        - feature_matrix.npy: Final feature matrix (n_docs × n_features)
        - feature_names.json: Ordered list of selected feature names
        - labels.npy: Binary labels (0=human, 1=ai)
        - selection_report.json: Detailed report from each stage
        - normalization.json: Z-score parameters for each feature
    """
    X, y, feature_names = load_features(h5_path)
    reports = []

    # Stage 1: Correlation filtering
    X, feature_names, report = stage1_correlation_filter(
        X, feature_names, threshold=config.CORRELATION_THRESHOLD
    )
    reports.append(report)

    # Stage 2: Mutual information
    X, feature_names, report = stage2_mutual_information(
        X, y, feature_names, top_k=config.MI_TOP_K
    )
    reports.append(report)

    # Stage 3: LASSO
    X, feature_names, report = stage3_lasso(
        X, y, feature_names, cv_folds=config.LASSO_CV_FOLDS
    )
    reports.append(report)

    # Stage 4: RFE
    X, feature_names, report = stage4_rfe(
        X, y, feature_names, target_features=target_max, step_fraction=config.RFE_STEP
    )
    reports.append(report)

    # Stage 5: Stability selection
    X, feature_names, report = stage5_stability_selection(
        X, y, feature_names,
        n_bootstrap=config.STABILITY_BOOTSTRAP_N,
        threshold=config.STABILITY_THRESHOLD,
    )
    reports.append(report)

    # Check target range
    n_final = len(feature_names)
    logger.info("=" * 60)
    if target_min <= n_final <= target_max:
        logger.info("PASS: %d features in target range [%d, %d]", n_final, target_min, target_max)
    else:
        logger.warning(
            "WARNING: %d features outside target range [%d, %d]",
            n_final, target_min, target_max,
        )

    # Compute normalization parameters (z-score)
    means = np.nanmean(X, axis=0)
    stds = np.nanstd(X, axis=0)
    stds[stds < 1e-10] = 1.0  # Avoid division by zero

    normalization = {
        "features": {
            name: {"mean": float(means[i]), "std": float(stds[i])}
            for i, name in enumerate(feature_names)
        }
    }

    # Save outputs
    output_dir.mkdir(parents=True, exist_ok=True)

    np.save(output_dir / "feature_matrix.npy", X)
    np.save(output_dir / "labels.npy", y)

    with open(output_dir / "feature_names.json", "w") as f:
        json.dump(feature_names, f, indent=2)

    with open(output_dir / "selection_report.json", "w") as f:
        json.dump(
            {"stages": reports, "final_feature_count": n_final, "target_range": [target_min, target_max]},
            f, indent=2,
        )

    with open(output_dir / "normalization.json", "w") as f:
        json.dump(normalization, f, indent=2)

    logger.info("Saved to %s:", output_dir)
    logger.info("  feature_matrix.npy: %s", X.shape)
    logger.info("  feature_names.json: %d features", n_final)
    logger.info("  labels.npy: %d labels", len(y))
    logger.info("  normalization.json: z-score params")
    logger.info("  selection_report.json: full pipeline report")

    return X, y, feature_names


def main():
    parser = argparse.ArgumentParser(
        description="5-stage feature selection pipeline."
    )
    parser.add_argument("--input", required=True, help="Input HDF5 features file")
    parser.add_argument("--output-dir", default=str(config.FEATURES_DIR), help="Output directory")
    parser.add_argument("--target-min", type=int, default=50, help="Minimum target features")
    parser.add_argument("--target-max", type=int, default=75, help="Maximum target features")
    args = parser.parse_args()

    run_selection_pipeline(
        Path(args.input),
        Path(args.output_dir),
        target_min=args.target_min,
        target_max=args.target_max,
    )


if __name__ == "__main__":
    main()
