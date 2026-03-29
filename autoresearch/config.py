"""Configuration for the Provenance AutoResearch agent.

Defines paths, research domains, evaluation parameters, and experiment
constraints. Mirrors the autoresearch pattern: humans edit this file,
the agent edits domain files.
"""

import os
import sys
from pathlib import Path

# ─── Paths ────────────────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
AUTORESEARCH_DIR = PROJECT_ROOT / "autoresearch"
DOMAINS_DIR = AUTORESEARCH_DIR / "domains"
RESULTS_DIR = AUTORESEARCH_DIR / "results"
RESULTS_TSV = RESULTS_DIR / "results.tsv"

# Provenance paths
TRAINING_DIR = PROJECT_ROOT / "training"
DATA_DIR = TRAINING_DIR / "data"
PROCESSED_DIR = DATA_DIR / "processed"
MODELS_DIR = PROJECT_ROOT / "models"
TEST_SUITE_DIR = PROJECT_ROOT / "test_suite"

# Provenance binary (Rust CLI)
PROVENANCE_BIN = os.environ.get(
    "PROVENANCE_BIN",
    str(PROJECT_ROOT / "target" / "release" / "provenance"),
)

# Ensure directories exist
for d in [DOMAINS_DIR, RESULTS_DIR]:
    d.mkdir(parents=True, exist_ok=True)

# ─── Experiment Constraints ──────────────────────────────────────────────────

# Default time budget per experiment (seconds)
TIME_BUDGET = int(os.environ.get("AUTORESEARCH_TIME_BUDGET", "300"))  # 5 minutes

# Maximum memory usage (MB) before an experiment is killed
MAX_MEMORY_MB = int(os.environ.get("AUTORESEARCH_MAX_MEMORY", "4096"))

# Random seed for reproducibility
RANDOM_SEED = 42

# ─── Research Domains ────────────────────────────────────────────────────────

DOMAINS = {
    "ai_detection": {
        "file": "domains/ai_detection.py",
        "description": "Improve AI-generated text detection accuracy and robustness",
        "primary_metric": "f1_score",
        "secondary_metrics": ["fpr", "tpr", "auroc", "adversarial_robustness"],
        "targets": {
            "f1_score": 0.97,       # F1 ≥ 0.97
            "fpr": 0.02,            # False positive rate ≤ 2%
            "tpr": 0.95,            # True positive rate ≥ 95%
            "auroc": 0.99,          # AUC-ROC ≥ 0.99
        },
        "optimization": "maximize",  # maximize f1
    },
    "human_voice": {
        "file": "domains/human_voice.py",
        "description": "Model unique stylistic fingerprints of individual writers",
        "primary_metric": "profile_accuracy",
        "secondary_metrics": ["voice_embedding_quality", "temporal_stability"],
        "targets": {
            "profile_accuracy": 0.95,
            "voice_embedding_quality": 0.90,
            "temporal_stability": 0.85,
        },
        "optimization": "maximize",
    },
    "logic_psychology": {
        "file": "domains/logic_psychology.py",
        "description": "Detect reasoning patterns, persuasion strategies, cognitive signatures",
        "primary_metric": "argument_f1",
        "secondary_metrics": ["fallacy_detection", "persuasion_accuracy", "bias_detection"],
        "targets": {
            "argument_f1": 0.90,
            "fallacy_detection": 0.85,
            "persuasion_accuracy": 0.88,
        },
        "optimization": "maximize",
    },
    "tone_analysis": {
        "file": "domains/tone_analysis.py",
        "description": "Classify emotional register, formality, irony, tonal shifts",
        "primary_metric": "tone_accuracy",
        "secondary_metrics": ["shift_detection_f1", "sarcasm_f1", "formality_correlation"],
        "targets": {
            "tone_accuracy": 0.92,
            "shift_detection_f1": 0.85,
            "sarcasm_f1": 0.80,
        },
        "optimization": "maximize",
    },
    "writer_identity": {
        "file": "domains/writer_identity.py",
        "description": "Build rich identity models capturing how a person thinks",
        "primary_metric": "eer",
        "secondary_metrics": ["verification_accuracy", "ranking_mrr", "adversarial_eer"],
        "targets": {
            "eer": 0.05,                # Equal Error Rate ≤ 5%
            "verification_accuracy": 0.95,
            "ranking_mrr": 0.90,
        },
        "optimization": "minimize",  # minimize EER
    },
    "adversarial_robustness": {
        "file": "domains/adversarial_robustness.py",
        "description": "Resist humanizers, paraphrasers, style transfer attacks",
        "primary_metric": "detection_accuracy",
        "secondary_metrics": ["humanizer_detection", "paraphrase_detection"],
        "targets": {
            "detection_accuracy": 0.90,
            "humanizer_detection": 0.85,
        },
        "optimization": "maximize",
    },
    "cross_linguistic": {
        "file": "domains/cross_linguistic.py",
        "description": "Handle NNES, L1 interference, code-switching, cultural patterns",
        "primary_metric": "l1_accuracy",
        "secondary_metrics": ["nnes_fpr_reduction", "code_switch_detection"],
        "targets": {
            "l1_accuracy": 0.80,
            "nnes_fpr_reduction": 0.50,
        },
        "optimization": "maximize",
    },
    "temporal_evolution": {
        "file": "domains/temporal_evolution.py",
        "description": "Model style drift, detect AI-assistance onset over time",
        "primary_metric": "drift_detection_accuracy",
        "secondary_metrics": ["change_point_f1", "model_version_accuracy"],
        "targets": {
            "drift_detection_accuracy": 0.85,
            "change_point_f1": 0.80,
        },
        "optimization": "maximize",
    },
    "document_forensics": {
        "file": "domains/document_forensics.py",
        "description": "Multi-author detection, copy-paste boundaries, assembly forensics",
        "primary_metric": "multi_author_f1",
        "secondary_metrics": ["boundary_detection_f1", "ghost_writing_accuracy"],
        "targets": {
            "multi_author_f1": 0.85,
            "boundary_detection_f1": 0.80,
        },
        "optimization": "maximize",
    },
}

# Domain rotation order (cycles through these)
DOMAIN_ORDER = [
    "ai_detection",
    "human_voice",
    "tone_analysis",
    "logic_psychology",
    "writer_identity",
    "adversarial_robustness",
    "cross_linguistic",
    "temporal_evolution",
    "document_forensics",
]

# ─── Evaluation Parameters ───────────────────────────────────────────────────

# Train/validation/test split ratios
SPLIT_RATIOS = {
    "train": 0.70,
    "validation": 0.15,
    "test": 0.15,
}

# Cross-validation folds
CV_FOLDS = 5

# Minimum improvement threshold to count as a real gain (not noise)
MIN_IMPROVEMENT_THRESHOLD = 0.001

# ─── Feature Categories ─────────────────────────────────────────────────────
# Maps research domains to Provenance's existing analysis modules

PROVENANCE_MODULES = {
    "ai_detection": [
        "detection.burstiness",
        "detection.zipf",
        "detection.hedge_ratio",
        "detection.autocorrelation",
        "detection.pos_entropy",
        "detection.interaction",
        "detection.tier2_features",
        "detection.advanced_features",
        "detection.countermeasures",
    ],
    "human_voice": [
        "analysis.lexical",
        "analysis.stylometric",
        "analysis.function_words",
        "analysis.ngrams",
        "identity.profile",
    ],
    "logic_psychology": [
        "analysis.syntactic",
        "analysis.semantic",
        "analysis.register",
    ],
    "tone_analysis": [
        "analysis.lexical",
        "analysis.semantic",
        "analysis.stylometric",
        "analysis.register",
    ],
    "writer_identity": [
        "identity.profile",
        "identity.comparison",
        "identity.confidence",
        "identity.anomaly",
        "identity.ranking",
        "identity.features",
        "identity.distances",
    ],
    "adversarial_robustness": [
        "detection.countermeasures",
        "detection.nnes",
        "detection.editing_tools",
        "adversarial",
    ],
    "cross_linguistic": [
        "detection.nnes",
        "analysis.lexical",
        "analysis.syntactic",
        "analysis.function_words",
    ],
    "temporal_evolution": [
        "identity.profile",
        "identity.comparison",
        "analysis.lexical",
        "analysis.stylometric",
    ],
    "document_forensics": [
        "forensics.metadata",
        "forensics.integrity",
        "forensics.docx",
        "analysis.stylometric",
        "analysis.semantic",
    ],
}

# ─── Logging ─────────────────────────────────────────────────────────────────

LOG_LEVEL = os.environ.get("AUTORESEARCH_LOG_LEVEL", "INFO")

# TSV header for results file
RESULTS_HEADER = (
    "timestamp\tcommit\tdomain\thypothesis\t"
    "metric_before\tmetric_after\tdelta\t"
    "memory_mb\tduration_s\tstatus\tnotes"
)
