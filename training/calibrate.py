#!/usr/bin/env python3
"""Two-stage calibration and threshold optimization for the Provenance ensemble.

Stage 1: Temperature scaling on 70% of validation set.
Stage 2: Isotonic regression on remaining 30%.
Then: find threshold where FPR = exactly 2% using scipy.optimize.brentq.

Usage:
    python calibrate.py --predictions models/val_predictions.npy --labels models/val_labels.npy
    python calibrate.py --predictions models/val_predictions.npy --labels models/val_labels.npy --output-dir models/

GO/NO-GO Gate 2: ECE < 0.05.

See ML_PIPELINE_IMPLEMENTATION_GUIDE.md Parts 4-5 for specification.
"""

import argparse
import json
import logging
from pathlib import Path

import numpy as np
from scipy.optimize import brentq, minimize_scalar
from sklearn.isotonic import IsotonicRegression
from sklearn.metrics import roc_curve

import config

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)


# ─── Calibration Metrics ─────────────────────────────────────────────────────


def expected_calibration_error(
    y_true: np.ndarray, y_prob: np.ndarray, n_bins: int = 10
) -> float:
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


def brier_score(y_true: np.ndarray, y_prob: np.ndarray) -> float:
    """Compute Brier score (MSE of probability estimates)."""
    return float(np.mean((y_prob - y_true) ** 2))


# ─── Two-Stage Calibrator ────────────────────────────────────────────────────


class TwoStageCalibrator:
    """Temperature scaling followed by isotonic regression.

    Stage 1: Learn a single temperature parameter T that minimizes NLL
             on the first portion of the validation set.
    Stage 2: Fit isotonic regression on the remaining portion using
             the temperature-scaled logits.
    """

    def __init__(self):
        self.temperature: float = 1.0
        self.isotonic: IsotonicRegression | None = None

    def fit(
        self,
        logits: np.ndarray,
        labels: np.ndarray,
        temp_fraction: float = 0.70,
    ):
        """Fit the two-stage calibrator.

        Args:
            logits: Raw model output scores (ensemble predictions).
            labels: Binary labels (0=human, 1=AI).
            temp_fraction: Fraction of data for temperature scaling (default 70%).
        """
        n = len(logits)
        indices = np.random.RandomState(42).permutation(n)
        split = int(n * temp_fraction)

        temp_idx = indices[:split]
        iso_idx = indices[split:]

        # Stage 1: Temperature scaling
        logger.info("Stage 1: Temperature scaling on %d samples...", len(temp_idx))

        def nll(t):
            """Negative log-likelihood with temperature."""
            t = max(t, 1e-6)
            scaled = logits[temp_idx] / t
            probs = 1.0 / (1.0 + np.exp(-scaled))
            probs = np.clip(probs, 1e-10, 1 - 1e-10)
            return -np.mean(
                labels[temp_idx] * np.log(probs)
                + (1 - labels[temp_idx]) * np.log(1 - probs)
            )

        result = minimize_scalar(nll, bounds=(0.1, 10.0), method="bounded")
        self.temperature = float(result.x)
        logger.info("  Optimal temperature: %.4f", self.temperature)

        # Stage 2: Isotonic regression on temperature-scaled logits
        logger.info("Stage 2: Isotonic regression on %d samples...", len(iso_idx))
        scaled_logits = logits[iso_idx] / self.temperature

        self.isotonic = IsotonicRegression(out_of_bounds="clip")
        self.isotonic.fit(scaled_logits, labels[iso_idx])

        logger.info("  Calibration fitting complete.")

    def calibrate(self, logit: float) -> float:
        """Apply both stages to a single logit."""
        scaled = logit / self.temperature
        return float(self.isotonic.predict([scaled])[0])

    def calibrate_batch(self, logits: np.ndarray) -> np.ndarray:
        """Apply both stages to a batch of logits."""
        scaled = logits / self.temperature
        return self.isotonic.predict(scaled)

    def to_dict(self) -> dict:
        """Serialize calibration parameters for Rust inference."""
        iso_x = self.isotonic.X_thresholds_.tolist() if hasattr(self.isotonic, "X_thresholds_") else []
        iso_y = self.isotonic.y_thresholds_.tolist() if hasattr(self.isotonic, "y_thresholds_") else []

        return {
            "temperature": self.temperature,
            "isotonic_x": iso_x,
            "isotonic_y": iso_y,
        }


# ─── Threshold Optimization ──────────────────────────────────────────────────


def find_threshold_at_fpr(
    y_true: np.ndarray,
    y_scores: np.ndarray,
    target_fpr: float = 0.02,
) -> tuple[float, float, float]:
    """Find threshold where FPR = exactly target_fpr using brentq.

    Returns: (threshold, actual_fpr, actual_tpr)
    """
    fpr_arr, tpr_arr, thresholds = roc_curve(y_true, y_scores)

    # Try brentq for precise threshold
    try:
        # Define function that returns FPR(threshold) - target_fpr
        def fpr_minus_target(thresh):
            preds = (y_scores >= thresh).astype(int)
            n_neg = (y_true == 0).sum()
            fp = ((preds == 1) & (y_true == 0)).sum()
            return fp / n_neg - target_fpr

        # Search between min and max score
        threshold = brentq(fpr_minus_target, float(y_scores.min()), float(y_scores.max()))
    except ValueError:
        # Fallback: find closest point on ROC curve
        idx = np.argmin(np.abs(fpr_arr - target_fpr))
        threshold = float(thresholds[idx])

    # Compute actual metrics at found threshold
    preds = (y_scores >= threshold).astype(int)
    n_neg = (y_true == 0).sum()
    n_pos = (y_true == 1).sum()
    fp = ((preds == 1) & (y_true == 0)).sum()
    tp = ((preds == 1) & (y_true == 1)).sum()
    actual_fpr = fp / n_neg if n_neg > 0 else 0.0
    actual_tpr = tp / n_pos if n_pos > 0 else 0.0

    return float(threshold), float(actual_fpr), float(actual_tpr)


# ─── Main Pipeline ────────────────────────────────────────────────────────────


def run_calibration(
    predictions: np.ndarray,
    labels: np.ndarray,
    output_dir: Path,
):
    """Run the full calibration and threshold optimization pipeline."""
    logger.info("=" * 60)
    logger.info("CALIBRATION PIPELINE")
    logger.info("=" * 60)
    logger.info("Predictions shape: %s", predictions.shape)
    logger.info("Label distribution: %.1f%% positive", labels.mean() * 100)

    # Fit calibrator
    calibrator = TwoStageCalibrator()
    calibrator.fit(
        predictions, labels,
        temp_fraction=config.CALIBRATION_TEMP_FRACTION,
    )

    # Calibrate predictions
    calibrated = calibrator.calibrate_batch(predictions)

    # Compute calibration metrics
    ece = expected_calibration_error(labels, calibrated)
    brier = brier_score(labels, calibrated)

    logger.info("\nCalibration Metrics:")
    logger.info("  ECE:    %.4f (target: < %.2f) %s",
                ece, config.TARGET_ECE_MAX,
                "PASS" if ece < config.TARGET_ECE_MAX else "FAIL")
    logger.info("  Brier:  %.4f", brier)

    # Find optimal threshold
    threshold, actual_fpr, actual_tpr = find_threshold_at_fpr(
        labels, calibrated, target_fpr=config.TARGET_FPR
    )

    logger.info("\nThreshold Optimization:")
    logger.info("  Target FPR:  %.2f%%", config.TARGET_FPR * 100)
    logger.info("  Threshold:   %.6f", threshold)
    logger.info("  Actual FPR:  %.4f%%", actual_fpr * 100)
    logger.info("  Actual TPR:  %.4f%% %s",
                actual_tpr * 100,
                "PASS" if actual_tpr >= config.TARGET_TPR_MIN else "FAIL")

    # GO/NO-GO gates
    logger.info("\n" + "=" * 60)
    logger.info("GO/NO-GO GATES")
    logger.info("=" * 60)
    gate1 = actual_tpr >= config.TARGET_TPR_MIN
    gate2 = ece < config.TARGET_ECE_MAX
    logger.info("  Gate 1 (TPR@2%%FPR >= %.0f%%): %s", config.TARGET_TPR_MIN * 100, "PASS" if gate1 else "FAIL")
    logger.info("  Gate 2 (ECE < %.2f):         %s", config.TARGET_ECE_MAX, "PASS" if gate2 else "FAIL")

    if gate1 and gate2:
        logger.info("  RESULT: GO — Proceed to Phase 3")
    else:
        logger.warning("  RESULT: NO-GO — Do NOT proceed to Phase 3")

    # Save outputs
    output_dir.mkdir(parents=True, exist_ok=True)

    # Calibration parameters (for Rust inference)
    calibration_params = calibrator.to_dict()
    with open(output_dir / "calibration.json", "w") as f:
        json.dump(calibration_params, f, indent=2)

    # Threshold and metrics
    threshold_config = {
        "threshold": threshold,
        "target_fpr": config.TARGET_FPR,
        "actual_fpr": actual_fpr,
        "actual_tpr": actual_tpr,
        "ece": ece,
        "brier_score": brier,
        "gate1_pass": gate1,
        "gate2_pass": gate2,
    }
    with open(output_dir / "threshold.json", "w") as f:
        json.dump(threshold_config, f, indent=2)

    logger.info("\nSaved to %s:", output_dir)
    logger.info("  calibration.json — calibration parameters (for Rust)")
    logger.info("  threshold.json   — threshold and metrics")

    return calibrator, threshold, ece


def main():
    parser = argparse.ArgumentParser(
        description="Two-stage calibration and threshold optimization."
    )
    parser.add_argument("--predictions", required=True, help="Path to ensemble predictions .npy")
    parser.add_argument("--labels", required=True, help="Path to validation labels .npy")
    parser.add_argument("--output-dir", default=str(config.MODELS_DIR), help="Output directory")
    args = parser.parse_args()

    predictions = np.load(args.predictions)
    labels = np.load(args.labels)

    run_calibration(predictions, labels, Path(args.output_dir))


if __name__ == "__main__":
    main()
