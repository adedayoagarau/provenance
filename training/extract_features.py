#!/usr/bin/env python3
"""Extract features from a corpus using the Provenance Rust binary.

Usage:
    python extract_features.py --corpus-dir data/corpus --output features.csv
    python extract_features.py --corpus-dir data/corpus --output features.csv --feature-set comprehensive

Expected corpus structure:
    corpus_dir/
        author_a/
            text1.txt
            text2.txt
        author_b/
            text1.txt
            text2.txt

The script calls the Provenance binary to extract stylometric features
from each text file, then assembles them into a single CSV for training.
"""

import argparse
import csv
import json
import os
import subprocess
import sys
from pathlib import Path


def find_provenance_binary():
    """Locate the compiled Provenance binary."""
    candidates = [
        Path("target/release/provenance"),
        Path("target/debug/provenance"),
        Path("provenance"),
    ]
    for path in candidates:
        if path.exists() and os.access(path, os.X_OK):
            return str(path)

    import shutil
    found = shutil.which("provenance")
    if found:
        return found

    print("ERROR: Could not find 'provenance' binary.", file=sys.stderr)
    print("Build it first: cargo build --release", file=sys.stderr)
    sys.exit(1)


def extract_features_for_file(binary, filepath, feature_set="standard"):
    """Run the provenance binary to extract features for a single file.

    Returns a dict of feature_name -> value, or None on failure.
    """
    try:
        result = subprocess.run(
            [binary, "analyze", str(filepath), "--format", "json"],
            capture_output=True,
            text=True,
            timeout=60,
        )
        if result.returncode != 0:
            print(f"  WARNING: Failed to process {filepath}: {result.stderr.strip()}", file=sys.stderr)
            return None

        data = json.loads(result.stdout)
        return data.get("features", {})
    except (subprocess.TimeoutExpired, json.JSONDecodeError, Exception) as e:
        print(f"  WARNING: Error processing {filepath}: {e}", file=sys.stderr)
        return None


def collect_corpus(corpus_dir):
    """Collect (author, filepath) pairs from the corpus directory."""
    corpus_path = Path(corpus_dir)
    if not corpus_path.is_dir():
        print(f"ERROR: Corpus directory not found: {corpus_dir}", file=sys.stderr)
        sys.exit(1)

    samples = []
    for author_dir in sorted(corpus_path.iterdir()):
        if not author_dir.is_dir():
            continue
        author_name = author_dir.name
        for text_file in sorted(author_dir.iterdir()):
            if text_file.is_file():
                samples.append((author_name, str(text_file)))

    return samples


def main():
    parser = argparse.ArgumentParser(
        description="Extract features from a corpus for ML training."
    )
    parser.add_argument(
        "--corpus-dir", required=True,
        help="Path to corpus directory (author_name/text_files structure)"
    )
    parser.add_argument(
        "--output", required=True,
        help="Output CSV file path"
    )
    parser.add_argument(
        "--feature-set", default="standard",
        choices=["minimal", "standard", "comprehensive"],
        help="Feature set to extract (default: standard)"
    )
    parser.add_argument(
        "--binary", default=None,
        help="Path to provenance binary (auto-detected if not specified)"
    )
    args = parser.parse_args()

    binary = args.binary or find_provenance_binary()
    print(f"Using binary: {binary}")

    samples = collect_corpus(args.corpus_dir)
    print(f"Found {len(samples)} files from {len(set(a for a, _ in samples))} authors")

    if not samples:
        print("ERROR: No samples found in corpus directory.", file=sys.stderr)
        sys.exit(1)

    # Extract features for each sample
    results = []
    all_feature_names = set()
    for i, (author, filepath) in enumerate(samples):
        print(f"  [{i+1}/{len(samples)}] Processing {filepath}...")
        features = extract_features_for_file(binary, filepath, args.feature_set)
        if features is not None:
            all_feature_names.update(features.keys())
            results.append((author, filepath, features))

    print(f"\nSuccessfully processed {len(results)}/{len(samples)} files")
    print(f"Total features: {len(all_feature_names)}")

    if not results:
        print("ERROR: No features extracted.", file=sys.stderr)
        sys.exit(1)

    # Write CSV
    sorted_features = sorted(all_feature_names)
    with open(args.output, "w", newline="") as f:
        writer = csv.writer(f)
        header = ["label", "source"] + sorted_features
        writer.writerow(header)

        for author, filepath, features in results:
            row = [author, filepath]
            for feat_name in sorted_features:
                row.append(features.get(feat_name, 0.0))
            writer.writerow(row)

    print(f"Features saved to {args.output}")


if __name__ == "__main__":
    main()
