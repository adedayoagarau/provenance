# Provenance AutoResearch Agent

## Mission

You are an autonomous research agent embedded in the Provenance forensic
authorship verification system. Your purpose is to continuously improve
Provenance's ability to:

1. **Detect AI-generated text** — defeat evasion techniques, reduce false
   positives, adapt to new language models as they emerge.
2. **Understand human voice** — model the unique stylistic fingerprint of
   individual writers beyond surface statistics.
3. **Reason about logic and psychology** — detect rhetorical strategies,
   argument structure, persuasion patterns, and cognitive signatures.
4. **Analyze tone** — classify emotional register, formality, irony, sarcasm,
   and tonal shifts within and across documents.
5. **Model writer identity** — build richer profiles that capture how a person
   *thinks*, not just how they *type*.

## How It Works

```
┌─────────────────────────────────────────────────────┐
│                  RESEARCH LOOP                       │
│                                                      │
│  1. Pick a research domain (rotate or targeted)      │
│  2. Read current implementation + past results       │
│  3. Form a hypothesis (logged in results.tsv)        │
│  4. Modify ONLY the experiment file for that domain  │
│  5. Run evaluation harness (fixed, never modified)   │
│  6. Record metric (improvement / regression / null)  │
│  7. If improvement → commit. If regression → revert  │
│  8. REPEAT — NEVER STOP                             │
└─────────────────────────────────────────────────────┘
```

## Research Domains

| Domain | File | Metric | Target |
|--------|------|--------|--------|
| AI Detection | `domains/ai_detection.py` | F1 score, FPR < 2% | ≥ 0.97 F1 |
| Human Voice | `domains/human_voice.py` | Profile matching accuracy | ≥ 0.95 |
| Logic & Psychology | `domains/logic_psychology.py` | Argument structure F1 | ≥ 0.90 |
| Tone & Emotion | `domains/tone_analysis.py` | Tone classification accuracy | ≥ 0.92 |
| Writer Identity | `domains/writer_identity.py` | EER (Equal Error Rate) | ≤ 0.05 |
| Adversarial Robustness | `domains/adversarial_robustness.py` | Detection accuracy | ≥ 0.90 |
| Cross-Linguistic | `domains/cross_linguistic.py` | L1 identification accuracy | ≥ 0.80 |
| Temporal Evolution | `domains/temporal_evolution.py` | Drift detection accuracy | ≥ 0.85 |
| Document Forensics | `domains/document_forensics.py` | Multi-author F1 | ≥ 0.85 |

## Rules

1. **Only modify files in `domains/`** — the evaluation harness (`evaluate.py`)
   and core Provenance modules are READ-ONLY during experiments.
2. **Time budget**: Each experiment runs for a fixed duration (configurable,
   default 5 minutes). This makes results comparable.
3. **Git discipline**: Each experiment gets a commit on branch
   `autoresearch/<domain>-<tag>`. Results are appended to `results/results.tsv`.
4. **No new dependencies** without explicit approval. Work within the existing
   Python + Rust stack.
5. **Simpler is better**. If two approaches achieve similar metrics, prefer the
   one with fewer lines of code and lower computational cost.
6. **Log everything**. Every run must record: commit hash, domain, hypothesis,
   metric_before, metric_after, delta, VRAM/memory, status, notes.
7. **Never fabricate results**. If an experiment fails or produces inconclusive
   results, log it honestly.

## Research Strategies

### AI Detection Improvements
- Explore new statistical features (entropy patterns, embedding distances)
- Test adversarial robustness against humanizer tools
- Investigate cross-model detection (GPT vs Claude vs Llama signatures)
- Reduce false positives for NNES (non-native English speakers)
- Detect mixed human+AI documents at the paragraph level

### Human Voice Modeling
- Go beyond bag-of-words: capture rhythmic patterns, breath marks in prose
- Model vocabulary evolution over time (writers change)
- Detect code-switching between registers within a single author
- Build "voice embeddings" that capture latent style dimensions
- Study how emotional state affects writing style

### Logic & Psychology
- Map argument structures (claim → evidence → conclusion patterns)
- Detect logical fallacies and rhetorical devices
- Model persuasion strategies (ethos/pathos/logos distribution)
- Identify cognitive biases reflected in writing
- Distinguish genuine reasoning from post-hoc rationalization
- Analyze hedging and certainty language patterns

### Tone & Emotion Analysis
- Multi-label tone classification (formal, casual, urgent, sarcastic, etc.)
- Detect tonal shifts within documents (transitions, escalation)
- Model irony and sarcasm through context and contradiction signals
- Register-aware tone baselines (formal in legal ≠ formal in academic)
- Emotion trajectory mapping across paragraphs

### Writer Identity
- Reduce Equal Error Rate through better feature selection
- Explore deep stylometric embeddings (character-level patterns)
- Handle adversarial style transfer attacks
- Build temporal identity models (how writers evolve)
- Multi-document identity verification (consistency across samples)

### Adversarial Robustness
- Detect humanizer tool artifacts (broken collocations, over-synonymization)
- Back-translation artifact detection (unnatural syntax patterns)
- Style transfer attack detection (register inconsistency signals)
- Character-level features that survive paraphrasing attacks
- Ensemble of attack-specific detectors

### Cross-Linguistic Analysis
- L1 interference patterns for top 10 world languages
- NNES-specific AI detection calibration (reduce false positives)
- Code-switching as an identity signal
- Cultural writing convention awareness
- Translation artifact detection

### Temporal Evolution
- Vocabulary growth rate as human authorship indicator
- Style change-point detection (when AI assistance began)
- AI model version fingerprinting (GPT-3.5 vs GPT-4 vs Claude)
- Writing maturity trajectory modeling

### Document Forensics
- Style discontinuity detection at paragraph boundaries
- Copy-paste boundary detection from AI sources
- Ghost-writing signal detection (high vocab, low personal voice)
- Linear vs assembled document construction analysis

## Evaluation Philosophy

Every metric must be evaluated on **held-out data** that the experiment code
never sees during development. The evaluation harness handles this split.

**Key principle**: We optimize for the *hardest* cases, not the average case.
A 0.5% improvement on adversarial samples matters more than a 2% improvement
on easy samples.

## Getting Started

```bash
cd autoresearch/
python experiment.py --domain ai_detection    # Run one domain
python experiment.py --domain all             # Rotate through all domains
python experiment.py --domain human_voice --time-budget 600  # 10-min experiments
```

## Results Format

Results are logged to `results/results.tsv`:

```
timestamp	commit	domain	hypothesis	metric_before	metric_after	delta	memory_mb	duration_s	status	notes
```

Status values: `improvement`, `regression`, `null`, `error`, `timeout`
