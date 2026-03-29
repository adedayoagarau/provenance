#!/usr/bin/env python3
"""3-model ensemble training pipeline for Provenance AI detection.

Trains XGBoost (0.7), Neural Network (0.2), and SVM (0.1) on the
feature matrix from Phase 2. Uses 5-fold stratified cross-validation
and Optuna hyperparameter optimization.

Usage:
    python train.py --features features/feature_matrix.npy --labels features/labels.npy
    python train.py --features features/feature_matrix.npy --labels features/labels.npy --skip-optuna
    python train.py --features features/feature_matrix.npy --labels features/labels.npy --quick

Primary metric: TPR@2%FPR >= 95% (GO/NO-GO gate).

See ML_PIPELINE_IMPLEMENTATION_GUIDE.md Part 3 for model architecture.
"""

import argparse
import json
import logging
import pickle
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from sklearn.ensemble import GradientBoostingClassifier
from sklearn.model_selection import StratifiedKFold
from sklearn.metrics import roc_curve, auc
from sklearn.preprocessing import StandardScaler
from sklearn.svm import SVC
from torch.utils.data import DataLoader, TensorDataset

import config

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)


# ─── Neural Network Model ────────────────────────────────────────────────────


class ProvenanceNN(nn.Module):
    """3-layer neural network for AI detection scoring."""

    def __init__(
        self,
        input_dim: int = 75,
        hidden_dims: list[int] | None = None,
        dropout: float = 0.3,
    ):
        super().__init__()
        if hidden_dims is None:
            hidden_dims = config.NN_PARAMS["hidden_layers"]

        layers = []
        prev_dim = input_dim
        for hidden_dim in hidden_dims:
            layers.extend([
                nn.Linear(prev_dim, hidden_dim),
                nn.BatchNorm1d(hidden_dim),
                nn.ReLU(),
                nn.Dropout(dropout),
            ])
            prev_dim = hidden_dim

        layers.append(nn.Linear(prev_dim, 1))
        layers.append(nn.Sigmoid())

        self.network = nn.Sequential(*layers)

    def forward(self, x):
        return self.network(x)


# ─── TPR@FPR Metric ──────────────────────────────────────────────────────────


def tpr_at_fpr(y_true: np.ndarray, y_scores: np.ndarray, target_fpr: float = 0.02) -> float:
    """Compute TPR at a specific FPR threshold."""
    fpr, tpr, _ = roc_curve(y_true, y_scores)
    # Find closest FPR to target
    idx = np.argmin(np.abs(fpr - target_fpr))
    return float(tpr[idx])


# ─── XGBoost Training ────────────────────────────────────────────────────────


def train_xgboost(
    X_train: np.ndarray,
    y_train: np.ndarray,
    X_val: np.ndarray,
    y_val: np.ndarray,
    use_optuna: bool = True,
) -> tuple:
    """Train gradient boosting classifier.

    Uses sklearn GradientBoostingClassifier instead of XGBoost native API
    to avoid segfaults on Python 3.14 + Apple Silicon.
    """
    logger.info("Training Gradient Boosting...")

    scaler = StandardScaler()
    X_train_s = scaler.fit_transform(X_train)
    X_val_s = scaler.transform(X_val)

    if use_optuna:
        import optuna

        optuna.logging.set_verbosity(optuna.logging.WARNING)

        def objective(trial):
            model = GradientBoostingClassifier(
                max_depth=trial.suggest_int("max_depth", 4, 8),
                learning_rate=trial.suggest_float("learning_rate", 0.01, 0.3, log=True),
                n_estimators=200,
                subsample=trial.suggest_float("subsample", 0.6, 1.0),
                random_state=config.RANDOM_SEED,
            )
            model.fit(X_train_s, y_train)
            preds = model.predict_proba(X_val_s)[:, 1]
            return tpr_at_fpr(y_val, preds, target_fpr=0.02)

        study = optuna.create_study(direction="maximize")
        study.optimize(objective, n_trials=30, timeout=600)

        logger.info("Optuna best TPR@2%%FPR: %.4f", study.best_value)
        logger.info("Optuna best params: %s", study.best_params)

        model = GradientBoostingClassifier(
            **study.best_params,
            n_estimators=config.XGBOOST_PARAMS["n_estimators"],
            random_state=config.RANDOM_SEED,
        )
    else:
        model = GradientBoostingClassifier(
            max_depth=config.XGBOOST_PARAMS["max_depth"],
            learning_rate=config.XGBOOST_PARAMS["learning_rate"],
            n_estimators=config.XGBOOST_PARAMS["n_estimators"],
            subsample=config.XGBOOST_PARAMS["subsample"],
            random_state=config.RANDOM_SEED,
        )

    model.fit(X_train_s, y_train)

    preds = model.predict_proba(X_val_s)[:, 1]
    val_tpr = tpr_at_fpr(y_val, preds, target_fpr=0.02)
    val_auc = auc(*roc_curve(y_val, preds)[:2])

    logger.info("Gradient Boosting — AUC: %.4f, TPR@2%%FPR: %.4f", val_auc, val_tpr)

    return model, scaler


# ─── Neural Network Training ─────────────────────────────────────────────────


def train_neural_net(
    X_train: np.ndarray,
    y_train: np.ndarray,
    X_val: np.ndarray,
    y_val: np.ndarray,
) -> ProvenanceNN:
    """Train the 3-layer neural network with early stopping."""
    logger.info("Training Neural Network...")

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    input_dim = X_train.shape[1]

    model = ProvenanceNN(input_dim=input_dim).to(device)
    criterion = nn.BCELoss()
    optimizer = optim.Adam(model.parameters(), lr=config.NN_PARAMS["learning_rate"])
    scheduler = optim.lr_scheduler.ReduceLROnPlateau(optimizer, mode="min", patience=5)

    train_ds = TensorDataset(
        torch.FloatTensor(X_train),
        torch.FloatTensor(y_train).unsqueeze(1),
    )
    val_ds = TensorDataset(
        torch.FloatTensor(X_val),
        torch.FloatTensor(y_val).unsqueeze(1),
    )
    train_loader = DataLoader(train_ds, batch_size=config.NN_PARAMS["batch_size"], shuffle=True)
    val_loader = DataLoader(val_ds, batch_size=config.NN_PARAMS["batch_size"], shuffle=False)

    best_val_loss = float("inf")
    patience_counter = 0
    best_state = None

    for epoch in range(config.NN_PARAMS["epochs"]):
        # Training
        model.train()
        train_loss = 0.0
        for X_batch, y_batch in train_loader:
            X_batch, y_batch = X_batch.to(device), y_batch.to(device)
            optimizer.zero_grad()
            outputs = model(X_batch)
            loss = criterion(outputs, y_batch)
            loss.backward()
            optimizer.step()
            train_loss += loss.item()

        # Validation
        model.eval()
        val_loss = 0.0
        with torch.no_grad():
            for X_batch, y_batch in val_loader:
                X_batch, y_batch = X_batch.to(device), y_batch.to(device)
                outputs = model(X_batch)
                loss = criterion(outputs, y_batch)
                val_loss += loss.item()

        val_loss /= max(len(val_loader), 1)
        scheduler.step(val_loss)

        if (epoch + 1) % 10 == 0:
            logger.info(
                "  Epoch %d/%d — Train: %.4f, Val: %.4f",
                epoch + 1, config.NN_PARAMS["epochs"],
                train_loss / max(len(train_loader), 1), val_loss,
            )

        # Early stopping
        if val_loss < best_val_loss:
            best_val_loss = val_loss
            patience_counter = 0
            best_state = model.state_dict().copy()
        else:
            patience_counter += 1
            if patience_counter >= config.NN_PARAMS["early_stopping_patience"]:
                logger.info("  Early stopping at epoch %d", epoch + 1)
                break

    if best_state:
        model.load_state_dict(best_state)

    # Evaluate
    model.eval()
    with torch.no_grad():
        preds = model(torch.FloatTensor(X_val).to(device)).cpu().numpy().flatten()

    val_tpr = tpr_at_fpr(y_val, preds, target_fpr=0.02)
    val_auc = auc(*roc_curve(y_val, preds)[:2])
    logger.info("Neural Net — AUC: %.4f, TPR@2%%FPR: %.4f", val_auc, val_tpr)

    return model


# ─── SVM Training ─────────────────────────────────────────────────────────────


def train_svm(
    X_train: np.ndarray,
    y_train: np.ndarray,
    X_val: np.ndarray,
    y_val: np.ndarray,
) -> SVC:
    """Train SVM with RBF kernel, tuning C via cross-validation."""
    logger.info("Training SVM...")

    best_svm = None
    best_score = -1.0

    for C in config.SVM_PARAMS["C_candidates"]:
        svm = SVC(
            kernel=config.SVM_PARAMS["kernel"],
            C=C,
            probability=True,
            random_state=config.SVM_PARAMS["random_state"],
        )
        svm.fit(X_train, y_train)
        preds = svm.predict_proba(X_val)[:, 1]
        score = tpr_at_fpr(y_val, preds, target_fpr=0.02)
        logger.info("  SVM C=%.1f — TPR@2%%FPR: %.4f", C, score)

        if score > best_score:
            best_score = score
            best_svm = svm

    preds = best_svm.predict_proba(X_val)[:, 1]
    val_auc = auc(*roc_curve(y_val, preds)[:2])
    logger.info("SVM (best) — AUC: %.4f, TPR@2%%FPR: %.4f", val_auc, best_score)

    return best_svm


# ─── Ensemble Evaluation ─────────────────────────────────────────────────────


def ensemble_predict(
    xgb_model,
    nn_model: ProvenanceNN,
    svm_model: SVC,
    X: np.ndarray,
    scaler: StandardScaler,
) -> np.ndarray:
    """Get ensemble predictions from all three models."""
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    X_scaled = scaler.transform(X)

    # XGBoost predictions (sklearn API)
    xgb_preds = xgb_model.predict_proba(X_scaled)[:, 1]

    # Neural net predictions
    nn_model.eval()
    with torch.no_grad():
        nn_preds = nn_model(torch.FloatTensor(X).to(device)).cpu().numpy().flatten()

    # SVM predictions
    svm_preds = svm_model.predict_proba(X_scaled)[:, 1]

    # Weighted ensemble
    w = config.ENSEMBLE_WEIGHTS
    ensemble = (
        w["xgboost"] * xgb_preds
        + w["neural_net"] * nn_preds
        + w["svm"] * svm_preds
    )

    return ensemble


# ─── Cross-Validation ─────────────────────────────────────────────────────────


def run_cross_validation(
    X: np.ndarray,
    y: np.ndarray,
    n_folds: int = 5,
    use_optuna: bool = True,
) -> dict:
    """Run stratified k-fold cross-validation."""
    logger.info("=" * 60)
    logger.info("Running %d-fold stratified cross-validation...", n_folds)
    logger.info("=" * 60)

    skf = StratifiedKFold(n_splits=n_folds, shuffle=True, random_state=42)
    fold_results = []

    for fold, (train_idx, val_idx) in enumerate(skf.split(X, y)):
        logger.info("\n--- Fold %d/%d ---", fold + 1, n_folds)

        X_train, X_val = X[train_idx], X[val_idx]
        y_train, y_val = y[train_idx], y[val_idx]

        # Train all three models
        xgb_model, scaler = train_xgboost(X_train, y_train, X_val, y_val, use_optuna=use_optuna)
        nn_model = train_neural_net(X_train, y_train, X_val, y_val)
        svm_model = train_svm(scaler.transform(X_train), y_train, scaler.transform(X_val), y_val)

        # Ensemble predictions
        ensemble_preds = ensemble_predict(xgb_model, nn_model, svm_model, X_val, scaler)

        fpr_arr, tpr_arr, _ = roc_curve(y_val, ensemble_preds)
        fold_auc = auc(fpr_arr, tpr_arr)
        fold_tpr = tpr_at_fpr(y_val, ensemble_preds, target_fpr=0.02)

        fold_results.append({
            "fold": fold + 1,
            "auc": float(fold_auc),
            "tpr_at_2pct_fpr": float(fold_tpr),
        })

        logger.info("Fold %d — Ensemble AUC: %.4f, TPR@2%%FPR: %.4f", fold + 1, fold_auc, fold_tpr)

    # Summary
    aucs = [r["auc"] for r in fold_results]
    tprs = [r["tpr_at_2pct_fpr"] for r in fold_results]

    summary = {
        "cv_folds": n_folds,
        "fold_results": fold_results,
        "mean_auc": float(np.mean(aucs)),
        "std_auc": float(np.std(aucs)),
        "mean_tpr_at_2pct_fpr": float(np.mean(tprs)),
        "std_tpr_at_2pct_fpr": float(np.std(tprs)),
    }

    logger.info("\n" + "=" * 60)
    logger.info("CROSS-VALIDATION RESULTS")
    logger.info("=" * 60)
    logger.info("  Mean AUC:          %.4f +/- %.4f", summary["mean_auc"], summary["std_auc"])
    logger.info("  Mean TPR@2%%FPR:   %.4f +/- %.4f", summary["mean_tpr_at_2pct_fpr"], summary["std_tpr_at_2pct_fpr"])

    if summary["mean_tpr_at_2pct_fpr"] >= config.TARGET_TPR_MIN:
        logger.info("  GO/NO-GO Gate 1:   PASS (TPR@2%%FPR >= %.0f%%)", config.TARGET_TPR_MIN * 100)
    else:
        logger.warning("  GO/NO-GO Gate 1:   FAIL (TPR@2%%FPR < %.0f%%)", config.TARGET_TPR_MIN * 100)

    return summary


# ─── Final Training ──────────────────────────────────────────────────────────


def train_final_models(
    X: np.ndarray,
    y: np.ndarray,
    output_dir: Path,
    use_optuna: bool = True,
):
    """Train final models on full dataset, save to output directory."""
    logger.info("=" * 60)
    logger.info("Training final models on full dataset...")
    logger.info("=" * 60)

    output_dir.mkdir(parents=True, exist_ok=True)

    # 80/20 split for final training
    n = len(y)
    indices = np.random.RandomState(42).permutation(n)
    split = int(n * 0.8)
    train_idx, val_idx = indices[:split], indices[split:]
    X_train, X_val = X[train_idx], X[val_idx]
    y_train, y_val = y[train_idx], y[val_idx]

    # Train models
    xgb_model, scaler = train_xgboost(X_train, y_train, X_val, y_val, use_optuna=use_optuna)
    nn_model = train_neural_net(X_train, y_train, X_val, y_val)
    svm_model = train_svm(scaler.transform(X_train), y_train, scaler.transform(X_val), y_val)

    # Ensemble evaluation
    ensemble_preds = ensemble_predict(xgb_model, nn_model, svm_model, X_val, scaler)
    final_tpr = tpr_at_fpr(y_val, ensemble_preds, target_fpr=0.02)
    fpr_arr, tpr_arr, _ = roc_curve(y_val, ensemble_preds)
    final_auc = auc(fpr_arr, tpr_arr)

    logger.info("Final Ensemble — AUC: %.4f, TPR@2%%FPR: %.4f", final_auc, final_tpr)

    # Save models
    with open(output_dir / "xgboost_model.pkl", "wb") as f:
        pickle.dump(xgb_model, f)
    torch.save(nn_model.state_dict(), str(output_dir / "neural_net.pt"))
    with open(output_dir / "svm_model.pkl", "wb") as f:
        pickle.dump(svm_model, f)
    with open(output_dir / "scaler.pkl", "wb") as f:
        pickle.dump(scaler, f)

    # Save NN architecture info for ONNX export
    nn_info = {
        "input_dim": X_train.shape[1],
        "hidden_layers": config.NN_PARAMS["hidden_layers"],
        "dropout": config.NN_PARAMS["dropout"],
    }
    with open(output_dir / "nn_architecture.json", "w") as f:
        json.dump(nn_info, f, indent=2)

    # Save training summary
    summary = {
        "final_auc": float(final_auc),
        "final_tpr_at_2pct_fpr": float(final_tpr),
        "n_train": len(y_train),
        "n_val": len(y_val),
        "n_features": X_train.shape[1],
        "ensemble_weights": config.ENSEMBLE_WEIGHTS,
        "gate1_pass": final_tpr >= config.TARGET_TPR_MIN,
    }
    with open(output_dir / "training_summary.json", "w") as f:
        json.dump(summary, f, indent=2)

    # Save validation predictions and labels for calibration/evaluation
    np.save(output_dir / "val_predictions.npy", ensemble_preds)
    np.save(output_dir / "val_labels.npy", y_val)

    logger.info("Models saved to %s", output_dir)
    return xgb_model, nn_model, svm_model, scaler, ensemble_preds, y_val


def main():
    parser = argparse.ArgumentParser(description="Train 3-model ensemble for AI detection.")
    parser.add_argument("--features", required=True, help="Path to feature_matrix.npy")
    parser.add_argument("--labels", required=True, help="Path to labels.npy")
    parser.add_argument("--output-dir", default=str(config.MODELS_DIR), help="Model output directory")
    parser.add_argument("--skip-optuna", action="store_true", help="Skip Optuna hyperparameter search")
    parser.add_argument("--cv-only", action="store_true", help="Only run cross-validation, don't save final models")
    parser.add_argument("--quick", action="store_true", help="Quick mode: no Optuna, 3-fold CV")
    args = parser.parse_args()

    X = np.load(args.features)
    y = np.load(args.labels)

    logger.info("Loaded: X=%s, y=%s (%.1f%% positive)", X.shape, y.shape, y.mean() * 100)

    use_optuna = not (args.skip_optuna or args.quick)
    n_folds = 3 if args.quick else config.CV_FOLDS

    # Cross-validation
    cv_summary = run_cross_validation(X, y, n_folds=n_folds, use_optuna=use_optuna)

    # Save CV results
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    with open(output_dir / "cv_results.json", "w") as f:
        json.dump(cv_summary, f, indent=2)

    if not args.cv_only:
        train_final_models(X, y, output_dir, use_optuna=use_optuna)


if __name__ == "__main__":
    main()
