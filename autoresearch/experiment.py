#!/usr/bin/env python3
"""Provenance AutoResearch Experiment Runner.

This is the main entry point for the autonomous research loop. It:
1. Selects a research domain (or rotates through all)
2. Picks a hypothesis from that domain
3. Runs the experiment with a fixed time budget
4. Records results to results.tsv
5. If improvement: keeps changes. If regression: reverts.
6. Repeats indefinitely.

Usage:
    python experiment.py --domain ai_detection       # Single domain
    python experiment.py --domain all                # Rotate all domains
    python experiment.py --domain all --cycles 10    # Run 10 cycles
    python experiment.py --domain human_voice --time-budget 600

This file is READ-ONLY during experiments. The agent modifies only
files in the domains/ directory.
"""

import argparse
import json
import os
import subprocess
import sys
import time
import traceback
from datetime import datetime, timezone
from pathlib import Path

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

from autoresearch.config import (
    AUTORESEARCH_DIR, DOMAINS_DIR, RESULTS_DIR, RESULTS_TSV,
    RESULTS_HEADER, TIME_BUDGET, MIN_IMPROVEMENT_THRESHOLD,
    DOMAIN_ORDER, DOMAINS, PROVENANCE_BIN, TEST_SUITE_DIR,
)
from autoresearch.domains import DomainState, ExperimentResult


# ─── Domain Registry ─────────────────────────────────────────────────────────

def get_researcher(domain: str):
    """Dynamically load the researcher class for a domain."""
    if domain == "ai_detection":
        from autoresearch.domains.ai_detection import AIDetectionResearcher
        return AIDetectionResearcher(PROVENANCE_BIN, str(TEST_SUITE_DIR))
    elif domain == "human_voice":
        from autoresearch.domains.human_voice import HumanVoiceResearcher
        return HumanVoiceResearcher()
    elif domain == "logic_psychology":
        from autoresearch.domains.logic_psychology import LogicPsychologyResearcher
        return LogicPsychologyResearcher()
    elif domain == "tone_analysis":
        from autoresearch.domains.tone_analysis import ToneAnalysisResearcher
        return ToneAnalysisResearcher()
    elif domain == "writer_identity":
        from autoresearch.domains.writer_identity import WriterIdentityResearcher
        return WriterIdentityResearcher()
    elif domain == "adversarial_robustness":
        from autoresearch.domains.adversarial_robustness import AdversarialRobustnessResearcher
        return AdversarialRobustnessResearcher()
    elif domain == "cross_linguistic":
        from autoresearch.domains.cross_linguistic import CrossLinguisticResearcher
        return CrossLinguisticResearcher()
    elif domain == "temporal_evolution":
        from autoresearch.domains.temporal_evolution import TemporalEvolutionResearcher
        return TemporalEvolutionResearcher()
    elif domain == "document_forensics":
        from autoresearch.domains.document_forensics import DocumentForensicsResearcher
        return DocumentForensicsResearcher()
    else:
        raise ValueError(f"Unknown domain: {domain}")


# ─── All Available Domains ───────────────────────────────────────────────────

ALL_DOMAINS = [
    "ai_detection",
    "human_voice",
    "logic_psychology",
    "tone_analysis",
    "writer_identity",
    "adversarial_robustness",
    "cross_linguistic",
    "temporal_evolution",
    "document_forensics",
]


# ─── Results Logging ─────────────────────────────────────────────────────────

def init_results_file():
    """Initialize results TSV if it doesn't exist."""
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    if not RESULTS_TSV.exists():
        RESULTS_TSV.write_text(RESULTS_HEADER + "\n")


def log_result(result: ExperimentResult, hypothesis_desc: str):
    """Append an experiment result to the TSV log."""
    commit = get_current_commit()
    timestamp = datetime.now(timezone.utc).isoformat()

    row = "\t".join([
        timestamp,
        commit,
        result.domain,
        hypothesis_desc[:100],
        f"{result.metric_before:.6f}",
        f"{result.metric_after:.6f}",
        f"{result.delta:+.6f}",
        f"{result.memory_mb:.1f}",
        f"{result.duration_seconds:.1f}",
        result.status,
        result.notes[:200].replace("\t", " ").replace("\n", " "),
    ])

    with open(RESULTS_TSV, "a") as f:
        f.write(row + "\n")

    print(f"  Result: {result.status} | delta={result.delta:+.6f} | "
          f"metric={result.metric_after:.6f} | {result.duration_seconds:.1f}s")


def get_current_commit() -> str:
    """Get the current git commit hash."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, timeout=5,
            cwd=str(AUTORESEARCH_DIR.parent),
        )
        return result.stdout.strip() if result.returncode == 0 else "unknown"
    except Exception:
        return "unknown"


# ─── Git Operations ──────────────────────────────────────────────────────────

def create_experiment_branch(domain: str, hypothesis_id: str) -> str:
    """Create a git branch for this experiment."""
    branch_name = f"autoresearch/{domain}-{hypothesis_id}"
    try:
        subprocess.run(
            ["git", "checkout", "-b", branch_name],
            capture_output=True, text=True, timeout=10,
            cwd=str(AUTORESEARCH_DIR.parent),
        )
    except Exception:
        pass  # Branch might already exist
    return branch_name


def commit_experiment(domain: str, hypothesis_id: str, result: ExperimentResult):
    """Commit experiment changes."""
    try:
        subprocess.run(
            ["git", "add", str(DOMAINS_DIR / f"{domain}.py"), str(RESULTS_TSV)],
            capture_output=True, text=True, timeout=10,
            cwd=str(AUTORESEARCH_DIR.parent),
        )
        msg = (f"autoresearch({domain}): {result.status} "
               f"[{hypothesis_id}] delta={result.delta:+.4f}")
        subprocess.run(
            ["git", "commit", "-m", msg],
            capture_output=True, text=True, timeout=10,
            cwd=str(AUTORESEARCH_DIR.parent),
        )
    except Exception as e:
        print(f"  Warning: git commit failed: {e}")


def revert_experiment():
    """Revert domain file changes (regression)."""
    try:
        subprocess.run(
            ["git", "checkout", "--", str(DOMAINS_DIR)],
            capture_output=True, text=True, timeout=10,
            cwd=str(AUTORESEARCH_DIR.parent),
        )
    except Exception:
        pass


# ─── Experiment Loop ─────────────────────────────────────────────────────────

def run_single_experiment(domain: str, time_budget: int, data_dir: Path) -> ExperimentResult:
    """Run a single experiment for a domain."""
    print(f"\n{'='*60}")
    print(f"  Domain: {domain}")
    print(f"  Time budget: {time_budget}s")
    print(f"{'='*60}")

    researcher = get_researcher(domain)
    hypotheses = researcher.get_hypotheses()

    if not hypotheses:
        print(f"  No hypotheses available for {domain}")
        return ExperimentResult(
            hypothesis_id="none", domain=domain,
            metric_before=0.0, metric_after=0.0, delta=0.0,
            duration_seconds=0.0, memory_mb=0.0,
            status="error", notes="No hypotheses available",
        )

    # Pick the next untested hypothesis (round-robin based on results log)
    tested = get_tested_hypotheses(domain)
    hypothesis = None
    for h in hypotheses:
        if h.id not in tested:
            hypothesis = h
            break
    if hypothesis is None:
        # All tested, restart from highest-impact
        hypothesis = hypotheses[0]

    print(f"  Hypothesis: {hypothesis.id}")
    print(f"  Description: {hypothesis.description[:80]}...")
    print(f"  Expected impact: {hypothesis.expected_impact}")
    print(f"  Complexity: {hypothesis.complexity}")
    print()

    # Run the experiment
    result = researcher.run_experiment(hypothesis, data_dir, time_budget)

    # Log results
    log_result(result, hypothesis.description)

    return result


def get_tested_hypotheses(domain: str) -> set:
    """Get set of hypothesis IDs already tested for a domain."""
    tested = set()
    if not RESULTS_TSV.exists():
        return tested

    for line in RESULTS_TSV.read_text().strip().split("\n")[1:]:  # Skip header
        parts = line.split("\t")
        if len(parts) >= 3 and parts[2] == domain:
            # Extract hypothesis ID from the description or notes
            for part in parts:
                if part.startswith(domain[:3]):
                    tested.add(part)
    return tested


def run_experiment_loop(
    domains: list[str],
    time_budget: int,
    max_cycles: int = 0,
    data_dir: Optional[Path] = None,
):
    """Run the autonomous experiment loop."""
    init_results_file()

    if data_dir is None:
        data_dir = TEST_SUITE_DIR

    cycle = 0
    domain_index = 0
    total_improvements = 0
    total_experiments = 0

    print("\n" + "="*60)
    print("  PROVENANCE AUTORESEARCH AGENT")
    print(f"  Domains: {', '.join(domains)}")
    print(f"  Time budget: {time_budget}s per experiment")
    print(f"  Max cycles: {'unlimited' if max_cycles == 0 else max_cycles}")
    print(f"  Data directory: {data_dir}")
    print("="*60)

    try:
        while max_cycles == 0 or cycle < max_cycles:
            domain = domains[domain_index % len(domains)]

            try:
                result = run_single_experiment(domain, time_budget, data_dir)
                total_experiments += 1

                if result.status == "improvement":
                    total_improvements += 1
                    print(f"  >>> IMPROVEMENT! Committing changes.")
                    commit_experiment(domain, result.hypothesis_id, result)
                elif result.status == "regression":
                    print(f"  <<< Regression. Reverting changes.")
                    revert_experiment()
                else:
                    print(f"  --- {result.status.capitalize()}. No changes.")

            except KeyboardInterrupt:
                raise
            except Exception as e:
                print(f"  ERROR in {domain}: {e}")
                traceback.print_exc()

            domain_index += 1
            cycle += 1

            # Print progress
            if cycle % len(domains) == 0:
                print(f"\n  === Cycle {cycle // len(domains)} complete ===")
                print(f"  Total experiments: {total_experiments}")
                print(f"  Improvements: {total_improvements}")
                rate = total_improvements / max(total_experiments, 1)
                print(f"  Improvement rate: {rate:.1%}")
                print()

    except KeyboardInterrupt:
        print(f"\n\n  Stopped by user after {total_experiments} experiments.")
        print(f"  Improvements: {total_improvements}")
    finally:
        print(f"\n  Results saved to: {RESULTS_TSV}")


# ─── CLI ─────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Provenance AutoResearch Experiment Runner",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python experiment.py --domain ai_detection          # Single domain
  python experiment.py --domain all                   # All domains
  python experiment.py --domain all --cycles 20       # 20 cycles
  python experiment.py --domain tone_analysis,human_voice  # Multiple
  python experiment.py --list-domains                 # List all domains
  python experiment.py --list-hypotheses ai_detection # List hypotheses
        """,
    )
    parser.add_argument(
        "--domain", type=str, default="all",
        help="Domain(s) to research (comma-separated, or 'all')",
    )
    parser.add_argument(
        "--time-budget", type=int, default=TIME_BUDGET,
        help=f"Time budget per experiment in seconds (default: {TIME_BUDGET})",
    )
    parser.add_argument(
        "--cycles", type=int, default=0,
        help="Max cycles (0 = unlimited, default: 0)",
    )
    parser.add_argument(
        "--data-dir", type=str, default=None,
        help="Data directory for evaluation (default: test_suite/)",
    )
    parser.add_argument(
        "--list-domains", action="store_true",
        help="List all available research domains",
    )
    parser.add_argument(
        "--list-hypotheses", type=str, default=None,
        help="List hypotheses for a specific domain",
    )

    args = parser.parse_args()

    if args.list_domains:
        print("\nAvailable Research Domains:")
        print("-" * 60)
        for domain in ALL_DOMAINS:
            researcher = get_researcher(domain)
            count = len(researcher.get_hypotheses())
            print(f"  {domain:<30} ({count} hypotheses)")
        print()
        return

    if args.list_hypotheses:
        domain = args.list_hypotheses
        researcher = get_researcher(domain)
        hypotheses = researcher.get_hypotheses()
        print(f"\nHypotheses for {domain}:")
        print("-" * 60)
        for h in hypotheses:
            print(f"\n  [{h.id}] {h.description}")
            print(f"    Impact: {h.expected_impact} | Complexity: {h.complexity}")
            print(f"    Approach: {h.approach[:100]}...")
        print()
        return

    # Parse domains
    if args.domain == "all":
        domains = ALL_DOMAINS
    else:
        domains = [d.strip() for d in args.domain.split(",")]
        for d in domains:
            if d not in ALL_DOMAINS:
                print(f"Error: Unknown domain '{d}'")
                print(f"Available: {', '.join(ALL_DOMAINS)}")
                sys.exit(1)

    data_dir = Path(args.data_dir) if args.data_dir else TEST_SUITE_DIR

    run_experiment_loop(
        domains=domains,
        time_budget=args.time_budget,
        max_cycles=args.cycles,
        data_dir=data_dir,
    )


# Allow optional import
from typing import Optional

if __name__ == "__main__":
    main()
