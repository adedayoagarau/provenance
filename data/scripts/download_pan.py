#!/usr/bin/env python3
"""Download and prepare PAN competition datasets for authorship attribution.

PAN (Plagiarism Analysis, Authorship Identification, Author Profiling)
is the standard evaluation framework for authorship attribution research.

Usage:
    python download_pan.py --year 2020 --output data/pan2020
    python download_pan.py --year 2021 --output data/pan2021 --task closed

Note: PAN datasets require registration at https://pan.webis.de/
This script provides the framework for organizing downloaded data
into the corpus_dir/<author>/<files> structure used by Provenance.
"""

import argparse
import json
import os
import sys
from pathlib import Path


# PAN dataset info (URLs require registration)
PAN_DATASETS = {
    2020: {
        "name": "PAN 2020 Authorship Verification",
        "url": "https://pan.webis.de/clef20/pan20-web/author-identification.html",
        "format": "jsonl",
        "tasks": ["open", "closed"],
    },
    2021: {
        "name": "PAN 2021 Authorship Verification",
        "url": "https://pan.webis.de/clef21/pan21-web/author-identification.html",
        "format": "jsonl",
        "tasks": ["open", "closed"],
    },
    2022: {
        "name": "PAN 2022 Authorship Verification",
        "url": "https://pan.webis.de/clef22/pan22-web/author-identification.html",
        "format": "jsonl",
        "tasks": ["open", "closed"],
    },
}


def prepare_pan_jsonl(input_dir, output_dir):
    """Convert PAN JSONL format to Provenance corpus structure.

    PAN format: pairs.jsonl with {id, fandoms, pair: [text1, text2], same_author}
    Provenance format: output_dir/<author_id>/text_N.txt
    """
    pairs_file = Path(input_dir) / "pairs.jsonl"
    truth_file = Path(input_dir) / "truth.jsonl"

    if not pairs_file.exists():
        print(f"ERROR: {pairs_file} not found.", file=sys.stderr)
        print("Download the dataset from PAN first.", file=sys.stderr)
        return False

    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    author_counter = {}
    doc_id = 0

    with open(pairs_file) as f:
        for line in f:
            data = json.loads(line.strip())
            pair_id = data.get("id", doc_id)

            for i, text in enumerate(data.get("pair", [])):
                author_id = f"pair_{pair_id}_author_{i}"
                author_dir = output_path / author_id
                author_dir.mkdir(exist_ok=True)

                count = author_counter.get(author_id, 0)
                text_file = author_dir / f"text_{count:03d}.txt"
                text_file.write_text(text, encoding="utf-8")
                author_counter[author_id] = count + 1

            doc_id += 1

    print(f"Prepared {doc_id} pairs → {output_dir}")
    return True


def prepare_gutenberg_texts(input_dir, output_dir, min_words=1000):
    """Organize Project Gutenberg texts into corpus structure.

    Expected input: directory of text files named like 'AuthorName - Title.txt'
    or subdirectories per author.
    """
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    processed = 0
    for text_file in sorted(input_path.glob("*.txt")):
        name = text_file.stem
        if " - " in name:
            author, title = name.split(" - ", 1)
            author = author.strip().replace(" ", "_").lower()
        else:
            author = "unknown"

        # Check minimum word count
        text = text_file.read_text(encoding="utf-8", errors="replace")
        word_count = len(text.split())
        if word_count < min_words:
            continue

        author_dir = output_path / author
        author_dir.mkdir(exist_ok=True)

        dest = author_dir / text_file.name
        dest.write_text(text, encoding="utf-8")
        processed += 1

    print(f"Processed {processed} Gutenberg texts → {output_dir}")


def main():
    parser = argparse.ArgumentParser(
        description="Download and prepare authorship attribution datasets."
    )
    subparsers = parser.add_subparsers(dest="command")

    # PAN dataset
    pan_parser = subparsers.add_parser("pan", help="Prepare PAN competition data")
    pan_parser.add_argument("--year", type=int, choices=[2020, 2021, 2022], required=True)
    pan_parser.add_argument("--input", required=True, help="Directory with downloaded PAN data")
    pan_parser.add_argument("--output", required=True, help="Output corpus directory")

    # Gutenberg
    gut_parser = subparsers.add_parser("gutenberg", help="Prepare Project Gutenberg texts")
    gut_parser.add_argument("--input", required=True, help="Directory with Gutenberg .txt files")
    gut_parser.add_argument("--output", required=True, help="Output corpus directory")
    gut_parser.add_argument("--min-words", type=int, default=1000)

    # Info
    subparsers.add_parser("info", help="Show available datasets")

    args = parser.parse_args()

    if args.command == "pan":
        info = PAN_DATASETS.get(args.year, {})
        print(f"Dataset: {info.get('name', 'Unknown')}")
        print(f"Info: {info.get('url', 'N/A')}")
        prepare_pan_jsonl(args.input, args.output)

    elif args.command == "gutenberg":
        prepare_gutenberg_texts(args.input, args.output, args.min_words)

    elif args.command == "info":
        print("Available datasets:")
        for year, info in sorted(PAN_DATASETS.items()):
            print(f"  PAN {year}: {info['name']}")
            print(f"    URL: {info['url']}")
            print(f"    Tasks: {', '.join(info['tasks'])}")
        print()
        print("Project Gutenberg: https://www.gutenberg.org/")
        print("  Download .txt files and use 'gutenberg' command to organize.")
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
