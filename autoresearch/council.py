#!/usr/bin/env python3
"""Provenance Research Council -- Main Engine.

The council coordinates autonomous researchers to analyze 1M+ labeled
text samples and generate research memos with actionable insights.

Usage:
    python3 -m autoresearch.council --analyze ai_vs_human
    python3 -m autoresearch.council --analyze model_signatures
    python3 -m autoresearch.council --full-report
    python3 -m autoresearch.council --memo "topic"
"""

import argparse
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from autoresearch.config import RESEARCH_DOMAINS, RESEARCH_OUTPUT_DIR, LOG_LEVEL
from autoresearch.researchers import write_memo

logging.basicConfig(
    level=getattr(logging, LOG_LEVEL),
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("research_council")


def get_researcher(domain: str):
    """Load a researcher by domain name."""
    info = RESEARCH_DOMAINS[domain]
    mod_name = info["researcher"]

    if mod_name == "statistical_features":
        from autoresearch.researchers.statistical_features import StatisticalFeaturesResearcher
        return StatisticalFeaturesResearcher()
    elif mod_name == "model_signatures":
        from autoresearch.researchers.model_signatures import ModelSignaturesResearcher
        return ModelSignaturesResearcher()
    elif mod_name == "evasion_research":
        from autoresearch.researchers.evasion_research import EvasionResearcher
        return EvasionResearcher()
    elif mod_name == "voice_psychology":
        from autoresearch.researchers.voice_psychology import VoicePsychologyResearcher
        return VoicePsychologyResearcher()
    elif mod_name == "fairness_equity":
        from autoresearch.researchers.fairness_equity import FairnessEquityResearcher
        return FairnessEquityResearcher()
    elif mod_name == "temporal_analysis":
        from autoresearch.researchers.temporal_analysis import TemporalAnalysisResearcher
        return TemporalAnalysisResearcher()
    elif mod_name == "frontier_ideas":
        from autoresearch.researchers.frontier_ideas import FrontierIdeasResearcher
        return FrontierIdeasResearcher()
    else:
        raise ValueError(f"Unknown researcher: {mod_name}")


def run_analysis(domain: str, sample_size: int = 0):
    """Run a single research domain analysis."""
    info = RESEARCH_DOMAINS[domain]
    print(f"\n{'='*70}")
    print(f"  RESEARCH COUNCIL: {info['description']}")
    print(f"{'='*70}\n")

    researcher = get_researcher(domain)
    memo_path = researcher.run(sample_size=sample_size)
    print(f"\n  Memo written to: {memo_path}")
    return memo_path


def run_full_report(sample_size: int = 0):
    """Run all research domains and generate a summary report."""
    print("\n" + "="*70)
    print("  PROVENANCE RESEARCH COUNCIL -- FULL REPORT")
    print(f"  {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}")
    print("="*70)

    domains = sorted(RESEARCH_DOMAINS.keys(), key=lambda d: RESEARCH_DOMAINS[d]["priority"])
    memos = []

    for domain in domains:
        try:
            path = run_analysis(domain, sample_size=sample_size)
            memos.append((domain, path))
        except Exception as e:
            logger.error("Domain %s failed: %s", domain, e)
            memos.append((domain, None))

    # Generate summary
    summary_lines = ["## Research Council Report Summary\n"]
    for domain, path in memos:
        status = f"[{path.name}](./{path.name})" if path else "FAILED"
        summary_lines.append(f"- **{domain}**: {status}")

    summary = "\n".join(summary_lines)
    summary_path = write_memo("Full Research Council Report", summary, "council")
    print(f"\n  Summary report: {summary_path}")
    print(f"  Total memos generated: {sum(1 for _, p in memos if p)}/{len(memos)}")


def generate_memo(topic: str, sample_size: int = 0):
    """Generate a focused research memo on a specific topic."""
    from autoresearch.researchers import load_dataset, compute_features_batch, write_memo

    print(f"\n  Generating research memo on: {topic}")
    df = load_dataset(sample_size=sample_size or 50000)
    df = compute_features_batch(df)

    # Run basic AI vs human comparison focused on the topic
    from autoresearch.researchers import compare_groups, TEXT_FEATURES

    findings = []
    for feature in TEXT_FEATURES:
        result = compare_groups(df, feature)
        if result.get("status") == "ok":
            findings.append(result)

    findings.sort(key=lambda f: abs(f.get("cohens_d", 0)), reverse=True)

    memo = f"## Research Question\n\n{topic}\n\n"
    memo += "## Key Findings\n\n"
    for f in findings[:10]:
        d = f["cohens_d"]
        direction = "higher" if d > 0 else "lower"
        memo += (f"- **{f['feature']}**: Human text is "
                 f"{abs(f['mean_a'] - f['mean_b']):.4f} {direction} "
                 f"(d={d:+.3f}, p={f['p_value']:.2e})\n")

    memo += "\n## Implications\n\nThese findings suggest...\n"
    path = write_memo(f"Memo: {topic}", memo, "council")
    print(f"  Written to: {path}")


def main():
    parser = argparse.ArgumentParser(description="Provenance Research Council")
    parser.add_argument("--analyze", type=str, help="Run a specific research domain")
    parser.add_argument("--full-report", action="store_true", help="Run all domains")
    parser.add_argument("--memo", type=str, help="Generate a memo on a topic")
    parser.add_argument("--sample", type=int, default=0,
                       help="Sample size (0=use config defaults)")
    parser.add_argument("--list", action="store_true", help="List research domains")
    args = parser.parse_args()

    if args.list:
        print("\nResearch Domains:")
        for name, info in sorted(RESEARCH_DOMAINS.items(), key=lambda x: x[1]["priority"]):
            print(f"  {name:<25} {info['description']}")
        return

    if args.analyze:
        if args.analyze not in RESEARCH_DOMAINS:
            print(f"Unknown domain: {args.analyze}")
            print(f"Available: {', '.join(RESEARCH_DOMAINS.keys())}")
            sys.exit(1)
        run_analysis(args.analyze, sample_size=args.sample)
    elif args.full_report:
        run_full_report(sample_size=args.sample)
    elif args.memo:
        generate_memo(args.memo, sample_size=args.sample)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
