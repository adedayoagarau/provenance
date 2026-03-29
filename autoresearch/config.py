"""Configuration for the Provenance Research Council.

Defines paths, research domains, analysis parameters, and data handling
configuration for 1M+ labeled samples.
"""

import os
from pathlib import Path

# -- Paths -----------------------------------------------------------------

PROJECT_ROOT = Path(__file__).resolve().parent.parent
AUTORESEARCH_DIR = PROJECT_ROOT / "autoresearch"
RESEARCHERS_DIR = AUTORESEARCH_DIR / "researchers"
RESEARCH_OUTPUT_DIR = AUTORESEARCH_DIR / "research_output"

# Primary dataset: 1M+ labeled JSONL samples
DATA_DIR = PROJECT_ROOT / "training" / "data" / "raw"
DATASET_PATH = DATA_DIR / "combined.jsonl"

# Ensure output directories exist
for _d in [RESEARCHERS_DIR, RESEARCH_OUTPUT_DIR]:
    _d.mkdir(parents=True, exist_ok=True)

# -- Dataset Schema --------------------------------------------------------
# Each JSONL record has:
#   text: str           -- the document text
#   label: str          -- "human", "ai", or "humanized"
#   source: str         -- provenance of the sample
#   register: str       -- genre/register (academic, news, creative, etc.)
#   model: str          -- generating model (llama3, mistral, gemma2, etc.) or null
#   prompt_type: str    -- prompt category used for generation
#   timestamp: str      -- ISO timestamp of collection
#   word_count: int     -- pre-computed word count

LABEL_HUMAN = "human"
LABEL_AI = "ai"
LABEL_HUMANIZED = "humanized"
ALL_LABELS = [LABEL_HUMAN, LABEL_AI, LABEL_HUMANIZED]

# -- Sampling & Performance ------------------------------------------------

# When exploring data, sample this many rows for fast iteration.
# Set to 0 to use full dataset (slower but complete).
QUICK_SAMPLE_SIZE = int(os.environ.get("COUNCIL_QUICK_SAMPLE", "50000"))

# Full analysis sample -- use when generating final memos
FULL_SAMPLE_SIZE = int(os.environ.get("COUNCIL_FULL_SAMPLE", "200000"))

# Random seed for reproducibility
RANDOM_SEED = 42

# Maximum memory budget (approximate, in MB)
MAX_MEMORY_MB = int(os.environ.get("COUNCIL_MAX_MEMORY", "8192"))

# -- Research Domains ------------------------------------------------------

RESEARCH_DOMAINS = {
    "ai_vs_human": {
        "description": "Core statistical comparison of AI vs human text",
        "researcher": "statistical_features",
        "priority": 1,
    },
    "model_signatures": {
        "description": "Per-model fingerprinting (Llama3, Mistral, Gemma2, etc.)",
        "researcher": "model_signatures",
        "priority": 2,
    },
    "evasion_resistance": {
        "description": "How AI text survives humanization and paraphrasing",
        "researcher": "evasion_research",
        "priority": 3,
    },
    "voice_patterns": {
        "description": "Human voice psychology and cognitive signatures",
        "researcher": "voice_psychology",
        "priority": 4,
    },
    "fairness": {
        "description": "NNES fairness, cross-cultural calibration, bias reduction",
        "researcher": "fairness_equity",
        "priority": 5,
    },
    "temporal_drift": {
        "description": "How AI-generated text evolves over model generations",
        "researcher": "temporal_analysis",
        "priority": 6,
    },
    "frontier": {
        "description": "Novel research ideas the field is missing",
        "researcher": "frontier_ideas",
        "priority": 7,
    },
}

# -- Statistical Thresholds ------------------------------------------------

# Minimum effect size (Cohen's d) to report as "notable"
MIN_EFFECT_SIZE = 0.2  # small effect per Cohen's conventions

# Significance level
ALPHA = 0.05

# Minimum sample size per group for reliable statistics
MIN_GROUP_SIZE = 100

# -- Feature Definitions ---------------------------------------------------
# Features computed on each text sample for analysis

TEXT_FEATURES = [
    "word_count",
    "sentence_count",
    "avg_sentence_length",
    "sentence_length_std",
    "avg_word_length",
    "type_token_ratio",
    "hapax_ratio",
    "function_word_ratio",
    "punctuation_ratio",
    "comma_rate",
    "semicolon_rate",
    "question_rate",
    "exclamation_rate",
    "paragraph_count",
    "avg_paragraph_length",
    "word_entropy",
    "char_entropy",
    "bigram_entropy",
    "yule_k",
    "brunet_w",
    "honore_r",
    "flesch_kincaid_grade",
    "automated_readability_index",
    "conjunction_rate",
    "adverb_rate",
    "passive_voice_ratio",
    "first_person_ratio",
    "hedge_word_ratio",
    "discourse_marker_ratio",
]

# -- Logging ---------------------------------------------------------------

LOG_LEVEL = os.environ.get("COUNCIL_LOG_LEVEL", "INFO")
