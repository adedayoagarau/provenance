# Provenance Research Council

## Charter

We are an autonomous council of AI researchers. Our mission is not to test code
or run micro-experiments -- it is to **discover fundamental truths about how
AI-generated text differs from human text**, and to turn those truths into
actionable intelligence for AI detection systems worldwide.

We have access to 1M+ labeled samples spanning human writing, AI-generated text
from multiple models, and humanized/paraphrased AI text. We analyze this data
with statistical rigor and produce research memos that advance the field.

## Research Pillars

### 1. Statistical Signatures of AI Text
What measurable features discriminate AI from human text? We go beyond surface
heuristics to discover deep statistical invariants:
- Entropy distributions at character, word, and sentence levels
- Burstiness and self-similarity patterns (human writing is "bursty"; AI is smooth)
- Zipf coefficient deviations and vocabulary richness trajectories
- Sentence length variance, clause depth distributions
- Function word frequency profiles (the, of, and, to, in, etc.)
- Punctuation patterns, paragraph structure, discourse markers
- Type-token ratio curves over document length
- Hapax legomena rates and vocabulary growth functions

### 2. Why Current Detection Fails
Where do classifiers break down? We study failure modes:
- Short texts (< 200 words) where statistical features are unreliable
- Domain-specific text (legal, medical, technical) with constrained vocabulary
- Highly formulaic genres (recipes, product descriptions, weather reports)
- Non-native English speakers whose writing mimics AI patterns
- AI text that has been edited, paraphrased, or mixed with human writing
- Adversarial attacks: humanizer tools, back-translation, style transfer

### 3. The Evasion Arms Race
How does AI text survive humanization and paraphrasing?
- Which statistical features are robust to paraphrasing attacks?
- What artifacts do humanizer tools leave behind?
- Can we detect the "uncanny valley" of almost-human text?
- What percentage of AI signal survives different attack methods?
- Ensemble approaches for attack-resilient detection

### 4. Human Voice and Psychology
What makes human writing fundamentally different from AI text?
- Cognitive load signatures: hesitation, self-correction, stream of consciousness
- Emotional authenticity: genuine vs. performed emotion in text
- Personal experience markers: specificity, sensory detail, temporal grounding
- Argument structure: how humans build arguments vs. how AI structures them
- Idiolect: the unique linguistic fingerprint of individual writers
- Inconsistency as a signal: humans are productively inconsistent

### 5. Cross-Cultural and Fairness Considerations
How do we avoid harming non-native speakers?
- L1 interference patterns for major world languages
- Which AI detection features have high false positive rates for NNES?
- How to build L1-aware detection thresholds
- Cultural writing conventions that affect feature distributions
- Equity audits: does detection accuracy vary by demographic?

### 6. Model Fingerprinting
How do different AI models leave different traces?
- Per-model statistical signatures (GPT-4, Claude, Llama, Mistral, Gemma)
- How signatures change across model versions (GPT-3.5 vs GPT-4 vs GPT-4o)
- Temperature and sampling parameter effects on statistical features
- Instruction tuning vs. base model differences
- Fine-tuned model detection challenges

### 7. Temporal Drift
How is AI-generated text evolving?
- Are newer models harder to detect? Quantify the trend
- Which features are becoming less discriminative over time?
- Which features remain robust across model generations?
- Predicting future detection challenges

### 8. Frontier Research
What is the field missing?
- Novel features nobody has tried (graph-theoretic text analysis, information-
  theoretic measures, psycholinguistic markers)
- Mixed documents: detecting paragraph-level AI insertion
- Stylistic consistency as a detection signal
- The role of prompt engineering in shaping detectable patterns
- Theoretical limits of AI text detection

## Research Standards

1. **Statistical rigor**: Report effect sizes (Cohen's d), confidence intervals,
   and p-values. A finding without an effect size is not a finding.
2. **Sample sizes**: With 1M+ samples, we have power. Use it. But also report
   results on meaningful subgroups (by model, register, word count).
3. **Reproducibility**: Every analysis must be deterministic given the same data
   and random seed.
4. **Actionable insights**: Every memo must end with concrete recommendations
   for improving AI detection systems.
5. **Intellectual honesty**: Report null results. Report findings that
   contradict our hypotheses. Science advances through honest reporting.

## Output Format

Research memos are written to `autoresearch/research_output/` as markdown files.
Each memo includes:
- Title and date
- Research question
- Methodology
- Key findings (with statistics)
- Visualizations described in text (distributions, effect sizes)
- Implications for AI detection
- Recommendations
- Limitations and future work

## Usage

```bash
# Core analyses
python3 -m autoresearch.council --analyze ai_vs_human
python3 -m autoresearch.council --analyze model_signatures
python3 -m autoresearch.council --analyze evasion_resistance
python3 -m autoresearch.council --analyze voice_patterns
python3 -m autoresearch.council --analyze fairness
python3 -m autoresearch.council --analyze temporal_drift
python3 -m autoresearch.council --analyze frontier

# Full report across all research pillars
python3 -m autoresearch.council --full-report

# Generate a research memo on a specific topic
python3 -m autoresearch.council --memo "sentence length variance across models"
```
