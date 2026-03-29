#!/usr/bin/env python3
"""Import Kaggle AI detection datasets into Provenance JSONL format.

Supports:
- DAIGT V2 (train_v2_drcat_02.csv) — columns: text, label (0=human, 1=ai), source, prompt_name, etc.
- AI vs Human datasets — columns vary, auto-detected

Usage:
    python import_kaggle.py --input /path/to/train_v2_drcat_02.csv
    python import_kaggle.py --input /path/to/dataset.csv --output data/raw/kaggle.jsonl
    python import_kaggle.py --input /path/to/dataset.csv --preview  # Show first 5 rows

The output JSONL follows the Provenance schema:
  text, label, source, register, model, prompt_type, timestamp, word_count
"""

import argparse
import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd

import config

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger(__name__)


def detect_dataset_format(df: pd.DataFrame) -> str:
    """Auto-detect which Kaggle dataset format this is."""
    cols = set(df.columns.str.lower())

    if "text" in cols and "label" in cols and "source" in cols:
        return "daigt_v2"
    elif "text" in cols and "label" in cols:
        return "generic"
    elif "text" in cols and "generated" in cols:
        return "generated_flag"
    elif "human_text" in cols or "ai_text" in cols:
        return "paired"
    else:
        return "unknown"


def convert_daigt_v2(df: pd.DataFrame) -> list[dict]:
    """Convert DAIGT V2 format (train_v2_drcat_02.csv)."""
    records = []
    for _, row in df.iterrows():
        text = str(row.get("text", "")).strip()
        if not text or len(text.split()) < 50:
            continue

        label_val = row.get("label", 0)
        label = "ai" if int(label_val) == 1 else "human"

        source = str(row.get("source", "kaggle_daigt"))
        model = str(row.get("model", "")) if "model" in df.columns else ""
        prompt_name = str(row.get("prompt_name", "")) if "prompt_name" in df.columns else ""

        records.append({
            "text": text,
            "label": label,
            "source": f"kaggle/{source}",
            "register": "academic",  # DAIGT is mostly essays
            "model": model,
            "prompt_type": prompt_name,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "word_count": len(text.split()),
        })

    return records


def convert_generic(df: pd.DataFrame) -> list[dict]:
    """Convert generic format with text + label columns."""
    records = []

    # Find the text column
    text_col = None
    for col in df.columns:
        if col.lower() in ("text", "content", "essay", "document", "article"):
            text_col = col
            break
    if not text_col:
        text_col = df.columns[0]

    # Find the label column
    label_col = None
    for col in df.columns:
        if col.lower() in ("label", "class", "target", "is_ai", "generated", "ai"):
            label_col = col
            break
    if not label_col:
        label_col = df.columns[1] if len(df.columns) > 1 else None

    for _, row in df.iterrows():
        text = str(row[text_col]).strip()
        if not text or len(text.split()) < 50:
            continue

        if label_col:
            raw_label = row[label_col]
            if isinstance(raw_label, (int, float)):
                label = "ai" if int(raw_label) == 1 else "human"
            elif isinstance(raw_label, str):
                label = "ai" if raw_label.lower() in ("ai", "1", "generated", "machine", "fake") else "human"
            else:
                label = "human"
        else:
            label = "human"

        records.append({
            "text": text,
            "label": label,
            "source": "kaggle",
            "register": "mixed",
            "model": "",
            "prompt_type": "",
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "word_count": len(text.split()),
        })

    return records


def convert_paired(df: pd.DataFrame) -> list[dict]:
    """Convert paired format with separate human_text and ai_text columns."""
    records = []

    for col in df.columns:
        if "human" in col.lower() and "text" in col.lower():
            for _, row in df.iterrows():
                text = str(row[col]).strip()
                if text and len(text.split()) >= 50:
                    records.append({
                        "text": text, "label": "human", "source": "kaggle",
                        "register": "mixed", "model": "", "prompt_type": "",
                        "timestamp": datetime.now(timezone.utc).isoformat(),
                        "word_count": len(text.split()),
                    })

        if "ai" in col.lower() and "text" in col.lower():
            for _, row in df.iterrows():
                text = str(row[col]).strip()
                if text and len(text.split()) >= 50:
                    records.append({
                        "text": text, "label": "ai", "source": "kaggle",
                        "register": "mixed", "model": "", "prompt_type": "",
                        "timestamp": datetime.now(timezone.utc).isoformat(),
                        "word_count": len(text.split()),
                    })

    return records


def main():
    parser = argparse.ArgumentParser(description="Import Kaggle datasets into Provenance JSONL format.")
    parser.add_argument("--input", required=True, help="Path to Kaggle CSV file")
    parser.add_argument("--output", default=None, help="Output JSONL path (default: data/raw/kaggle.jsonl)")
    parser.add_argument("--preview", action="store_true", help="Preview first 5 rows and exit")
    parser.add_argument("--append", action="store_true", help="Append to existing JSONL instead of overwriting")
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        logger.error("File not found: %s", input_path)
        sys.exit(1)

    output_path = Path(args.output) if args.output else config.RAW_DIR / "kaggle.jsonl"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Load CSV
    logger.info("Loading %s...", input_path)
    try:
        df = pd.read_csv(input_path)
    except Exception as e:
        logger.error("Failed to read CSV: %s", e)
        sys.exit(1)

    logger.info("Loaded %d rows, %d columns: %s", len(df), len(df.columns), list(df.columns))

    if args.preview:
        print("\n--- Preview ---")
        print(df.head())
        print(f"\nColumns: {list(df.columns)}")
        print(f"Shape: {df.shape}")
        if "label" in df.columns:
            print(f"Label distribution:\n{df['label'].value_counts()}")
        return

    # Detect format and convert
    fmt = detect_dataset_format(df)
    logger.info("Detected format: %s", fmt)

    if fmt == "daigt_v2":
        records = convert_daigt_v2(df)
    elif fmt == "generic" or fmt == "generated_flag":
        records = convert_generic(df)
    elif fmt == "paired":
        records = convert_paired(df)
    else:
        logger.warning("Unknown format — trying generic conversion")
        records = convert_generic(df)

    if not records:
        logger.error("No valid records extracted!")
        sys.exit(1)

    # Count by label
    human_count = sum(1 for r in records if r["label"] == "human")
    ai_count = sum(1 for r in records if r["label"] == "ai")

    logger.info("Extracted %d records: %d human, %d AI", len(records), human_count, ai_count)

    # Write JSONL
    mode = "a" if args.append else "w"
    with open(output_path, mode, encoding="utf-8") as f:
        for record in records:
            f.write(json.dumps(record, ensure_ascii=False) + "\n")

    logger.info("Saved to %s", output_path)

    # Summary
    word_counts = [r["word_count"] for r in records]
    logger.info("Word count stats: min=%d, max=%d, mean=%d, median=%d",
                min(word_counts), max(word_counts),
                sum(word_counts) // len(word_counts),
                sorted(word_counts)[len(word_counts) // 2])


if __name__ == "__main__":
    main()
