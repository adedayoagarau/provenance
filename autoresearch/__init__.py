"""Provenance AutoResearch — Autonomous AI Research Agent.

An autonomous research swarm that continuously improves Provenance's
capabilities across 9 research domains:

1. AI Detection — defeat evasion, reduce false positives
2. Human Voice — model unique stylistic fingerprints
3. Logic & Psychology — detect reasoning patterns, cognitive signatures
4. Tone & Emotion — classify register, formality, irony, sarcasm
5. Writer Identity — build rich identity models (EER minimization)
6. Adversarial Robustness — resist humanizers, paraphrasers, style transfer
7. Cross-Linguistic — handle NNES, L1 interference, code-switching
8. Temporal Evolution — model style drift, detect AI-assistance onset
9. Document Forensics — multi-author detection, copy-paste boundaries

Architecture follows the autoresearch pattern:
- program.md: Human instructions for the agent
- config.py: Configuration (humans edit this)
- experiment.py: Experiment runner (read-only during experiments)
- evaluate.py: Evaluation harness (read-only during experiments)
- domains/*.py: Research modules (agent edits these)
- results/results.tsv: Experiment log

Usage:
    python -m autoresearch.experiment --domain all
    python -m autoresearch.experiment --list-domains
    python -m autoresearch.evaluate
"""

__version__ = "0.1.0"
