"""Centralized configuration for the ML training pipeline.

All API keys are loaded from environment variables. Distribution targets,
model lists, and prompt templates are defined here for consistency across
pipeline phases.
"""

import os
from pathlib import Path

# ─── Paths ────────────────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
TRAINING_DIR = PROJECT_ROOT / "training"
DATA_DIR = TRAINING_DIR / "data"
RAW_DIR = DATA_DIR / "raw"
PROCESSED_DIR = DATA_DIR / "processed"
FEATURES_DIR = TRAINING_DIR / "features"
FEATURES_CACHE_DIR = FEATURES_DIR / "cache"
MODELS_DIR = PROJECT_ROOT / "models"
RESULTS_DIR = TRAINING_DIR / "results"

# Ensure key directories exist
for d in [RAW_DIR, PROCESSED_DIR, FEATURES_CACHE_DIR, MODELS_DIR, RESULTS_DIR]:
    d.mkdir(parents=True, exist_ok=True)

# ─── API Keys (from environment) ─────────────────────────────────────────────

OPENROUTER_API_KEY = os.environ.get("OPENROUTER_API_KEY", "")
REDDIT_CLIENT_ID = os.environ.get("REDDIT_CLIENT_ID", "")
REDDIT_CLIENT_SECRET = os.environ.get("REDDIT_CLIENT_SECRET", "")
REDDIT_USER_AGENT = os.environ.get("REDDIT_USER_AGENT", "provenance-collector/1.0")
GITHUB_TOKEN = os.environ.get("GITHUB_TOKEN", "")
QUILLBOT_API_KEY = os.environ.get("QUILLBOT_API_KEY", "")
UNDETECTABLE_API_KEY = os.environ.get("UNDETECTABLE_API_KEY", "")

# ─── Data Collection Targets ─────────────────────────────────────────────────

HUMAN_TARGET = 50_000
AI_TARGET = 75_000
HUMANIZED_TARGET = 25_000  # Subset of AI samples run through humanizers

# Human source distribution (fractions of HUMAN_TARGET)
HUMAN_SOURCES = {
    "arxiv": 0.25,         # ArXiv API — academic papers
    "news": 0.20,          # Reuters/AP RSS feeds — journalism
    "gutenberg": 0.15,     # Project Gutenberg — literary
    "edgar": 0.15,         # SEC EDGAR API — financial/legal filings
    "reddit": 0.10,        # Reddit API — verified human posts
    "github": 0.10,        # GitHub READMEs — technical documentation
    "legal": 0.05,         # Legal filings — court documents
}

# AI model distribution (equal split across models)
AI_MODELS = [
    "openai/gpt-4",
    "anthropic/claude-3-sonnet",
    "google/gemini-pro",
    "meta-llama/llama-3-70b-instruct",
    "mistralai/mistral-large",
    "openai/gpt-3.5-turbo",
]

# Prompting strategy distribution (fractions of AI_TARGET)
PROMPT_TYPES = {
    "zero_shot": 0.30,
    "few_shot": 0.25,
    "persona": 0.20,
    "anti_detection": 0.15,
    "chain_of_thought": 0.10,
}

# Humanizer tools for adversarial samples
HUMANIZER_TOOLS = ["quillbot", "undetectable_ai"]

# ─── Register Labels ─────────────────────────────────────────────────────────

REGISTERS = [
    "academic", "business", "journalism", "legal", "literary",
    "medical", "personal", "political", "technical", "mixed",
]

# ─── JSONL Schema ─────────────────────────────────────────────────────────────

JSONL_FIELDS = [
    "text", "label", "source", "register", "model",
    "prompt_type", "timestamp", "word_count",
]

# label: "human" | "ai" | "humanized"

# ─── Feature Engineering ─────────────────────────────────────────────────────

# Path to the compiled provenance binary
PROVENANCE_BIN = os.environ.get(
    "PROVENANCE_BIN",
    str(PROJECT_ROOT / "target" / "release" / "provenance"),
)

# Feature selection parameters
CORRELATION_THRESHOLD = 0.95       # Remove one from pairs with r > this
MI_TOP_K = 200                     # Keep top-K by mutual information
LASSO_CV_FOLDS = 5                 # Cross-validation folds for LASSO alpha
RFE_STEP = 0.10                    # Eliminate 10% of features per RFE iteration
STABILITY_BOOTSTRAP_N = 100        # Number of bootstrap samples
STABILITY_THRESHOLD = 0.80         # Keep features selected in >80% of runs
TARGET_FEATURE_COUNT = (50, 75)    # Target range for final feature set

# ─── Model Training ──────────────────────────────────────────────────────────

# XGBoost hyperparameters (exact values from ML_PIPELINE_IMPLEMENTATION_GUIDE)
XGBOOST_PARAMS = {
    "max_depth": 6,
    "learning_rate": 0.1,
    "n_estimators": 500,
    "subsample": 0.8,
    "colsample_bytree": 0.8,
    "reg_alpha": 0.1,
    "reg_lambda": 1.0,
    "scale_pos_weight": 1.25,
    "objective": "binary:logistic",
    "eval_metric": "auc",
    "random_state": 42,
}

# Neural network architecture
NN_PARAMS = {
    "input_dim": 75,               # Updated after feature selection
    "hidden_layers": [128, 64, 32],
    "dropout": 0.3,
    "batch_norm": True,
    "activation": "relu",
    "output_activation": "sigmoid",
    "optimizer": "adam",
    "learning_rate": 0.001,
    "loss": "bce",
    "batch_size": 32,
    "epochs": 100,
    "early_stopping_patience": 15,
}

# SVM hyperparameters
SVM_PARAMS = {
    "kernel": "rbf",
    "C_candidates": [0.1, 1.0, 10.0],
    "probability": True,
    "random_state": 42,
}

# Ensemble weights
ENSEMBLE_WEIGHTS = {
    "xgboost": 0.7,
    "neural_net": 0.2,
    "svm": 0.1,
}

# Cross-validation
CV_FOLDS = 5

# ─── Calibration & Threshold ─────────────────────────────────────────────────

CALIBRATION_TEMP_FRACTION = 0.70   # 70% of val set for temperature scaling
CALIBRATION_ISO_FRACTION = 0.30    # 30% for isotonic regression
TARGET_FPR = 0.02                  # 2% false positive rate
TARGET_TPR_MIN = 0.95              # Must achieve ≥95% TPR at target FPR
TARGET_ECE_MAX = 0.05              # Expected calibration error < 0.05

# ─── Performance Budgets ─────────────────────────────────────────────────────

MAX_INFERENCE_SECONDS = 2.0        # Total pipeline for 5000-word document
MAX_FEATURE_EXTRACTION_SECONDS = 1.5
MAX_MODEL_INFERENCE_MS = 50
TARGET_THROUGHPUT_PER_CORE = 100   # Documents per minute per core

# ─── Monitoring Thresholds ────────────────────────────────────────────────────

FPR_ALERT_THRESHOLD = 0.025       # Alert if FPR > 2.5%
TPR_ALERT_THRESHOLD = 0.92        # Alert if TPR < 92%
PSI_DRIFT_THRESHOLD = 0.10        # Alert if PSI > 0.1
ECE_WEEKLY_THRESHOLD = 0.05       # Alert if weekly ECE > 0.05

# ─── Prompt Templates ────────────────────────────────────────────────────────

# Base topics for AI text generation (cover all registers)
GENERATION_TOPICS = {
    "academic": [
        "Write a research paper introduction about {topic}",
        "Compose an academic literature review on {topic}",
        "Draft a methodology section for a study on {topic}",
    ],
    "business": [
        "Write a business proposal for {topic}",
        "Compose a quarterly earnings report analysis for {topic}",
        "Draft a corporate strategy memo about {topic}",
    ],
    "journalism": [
        "Write a news article about {topic}",
        "Compose an investigative report on {topic}",
        "Draft an editorial opinion piece about {topic}",
    ],
    "legal": [
        "Write a legal brief arguing {topic}",
        "Compose a contract clause for {topic}",
        "Draft a legal memorandum analyzing {topic}",
    ],
    "literary": [
        "Write a short story about {topic}",
        "Compose a personal essay reflecting on {topic}",
        "Draft a book review of a work about {topic}",
    ],
    "technical": [
        "Write technical documentation for {topic}",
        "Compose a README for a project about {topic}",
        "Draft an API reference guide for {topic}",
    ],
    "medical": [
        "Write a clinical case report about {topic}",
        "Compose patient education material about {topic}",
        "Draft a medical research abstract on {topic}",
    ],
    "personal": [
        "Write a personal blog post about {topic}",
        "Compose a reflective journal entry about {topic}",
        "Draft a personal letter discussing {topic}",
    ],
}

# Persona prompts for the "persona" prompt type
PERSONA_PREFIXES = [
    "You are a university professor writing for an academic journal.",
    "You are a journalist at a major newspaper.",
    "You are a corporate executive drafting an internal memo.",
    "You are a novelist working on your latest book.",
    "You are a lawyer preparing a legal document.",
    "You are a software engineer writing documentation.",
    "You are a medical researcher writing a paper.",
    "You are a high school student writing an essay.",
]

# Anti-detection prompts
ANTI_DETECTION_SUFFIXES = [
    "Write in a natural, human-like style with varied sentence lengths.",
    "Include personal anecdotes and subjective opinions.",
    "Use informal language and colloquialisms where appropriate.",
    "Vary your paragraph lengths and avoid overly structured formatting.",
    "Write as if you were a real person sharing their genuine thoughts.",
]
