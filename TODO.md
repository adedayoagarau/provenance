# PROVENANCE — Research-Validated Development Roadmap

**Last Updated:** 2026-03-22
**Status:** All 25 Phases Implemented — Foundation through Enterprise
**Research Basis:** Burrows (2002), Stamatatos (2009, 2017), Koppel et al. (2009), PAN competition methods

> Feature priorities reordered based on published stylometry research.
> Function words and character n-grams are proven most effective for authorship attribution.
> Topic/semantic content analysis is deprioritized as it confounds authorship signals.

---

## Phase 0: Cleanup & Foundation Fixes ✓

### 0.1 Remove false async
- [x] Remove `async` from all function signatures
- [x] Remove `tokio` dependency
- [x] Use plain `fn main()`

### 0.2 Activate error types
- [x] Wire `ProvenanceError` into all modules (replace bare `anyhow`)
- [x] Add `IoWithPath` variant for contextual file errors
- [x] Add `read_file()` / `read_file_string()` helpers with path context
- [x] Keep `anyhow` only in `main.rs` for top-level

### 0.3 Activate config system
- [x] Add TOML config loading (`Config::load()`, `Config::load_default()`)
- [x] Add `toml` crate dependency
- [x] Serde defaults for all config fields
- [x] Feature set selection field (minimal/standard/comprehensive)
### 0.4 Remove unused dependencies
- [x] Remove `tokio`
- [x] Remove `zip` (replaced by docx-rs later)
- [x] Remove `xml-rs` (unused)

### 0.5 Add CI
- [x] GitHub Actions workflow: build, test, clippy, rustfmt

### 0.6 Clean up analysis signatures
- [x] Analysis functions return values directly (not `Result`) since they're pure computations
- [x] Remove `anyhow` from analysis modules
- [x] Update tests and benchmarks

---

## Phase 1: Text Extraction Pipeline ✓

> 13 format extractors, magic byte detection, NFC normalization, encoding detection.

### 1.1 Core Formats (Tier 1) ✓
- [x] **Plain text** — encoding detection (UTF-8, UTF-16 LE/BE, Windows-1252, BOM)
- [x] **Markdown** — pulldown-cmark parser, excludes code blocks
- [x] **HTML** — scraper-based, excludes script/style/noscript
- [x] **PDF** — lopdf text extraction, encryption detection
- [x] **DOCX** — ZIP + XML parser (w:t elements)
### 1.2 Extended Formats (Tier 2) ✓
- [x] **RTF** — Custom parser handling control words, unicode escapes, Windows-1252
- [x] **ODT** — ZIP + content.xml parser (text:p/text:h elements)
- [x] **EPUB** — ZIP + OPF manifest + spine-ordered XHTML extraction
- [x] **EML** — mailparse for MIME multipart, prefers text/plain, falls back to HTML stripping
- [x] **MBOX** — Multi-message mailbox parsing with per-message extraction
- [x] **LaTeX** — Custom stripper: removes commands, math, comments, keeps prose
- [x] **JSON chat exports** — Auto-detects Discord/Slack/Telegram/Twitter structures
- [x] **CSV chat exports** — Column auto-detection for message content

### 1.3 Extraction Infrastructure ✓
- [x] Format auto-detection via magic bytes (PDF, ZIP/DOCX/ODT/EPUB, RTF, EML, MBOX, LaTeX)
- [x] Extension fallback for detection
- [x] ZIP-based format disambiguation (DOCX vs ODT vs EPUB)
- [x] NFC unicode normalization on all extracted text
- [x] Line ending normalization (\r\n → \n)
- [x] Whitespace collapse (max 2 consecutive newlines)
- [x] 14 tests covering all formats + edge cases

### 1.4 Future Format Extensions (Not Yet Implemented)
- [ ] **DOC** (legacy binary format) — no mature Rust crate, consider external tool
- [ ] **Scrivener** (.scriv/.scrivx) — XML manifest + RTF content files
- [ ] **Apple Pages** — ZIP + protobuf (complex, low priority)
- [ ] **reStructuredText** — rust-rst crate when mature
- [ ] **AsciiDoc** — asciidocr crate
- [ ] **ENEX** (Evernote) — XML-based, parse with serde
- [ ] **MSG** (Outlook) — msg_parser crate
- [ ] Language detection — `whatlang` crate (auto-detect input language)
---

## Phase 2: Core Stylometric Features (Research-Validated) ✓

### 2.1 Function word analysis ✓
- [x] 200+ English function words (determiners, prepositions, conjunctions, pronouns, auxiliaries, adverbs)
- [x] Frequency distribution normalized per 1000 words
- [x] Function word ratio + diversity metrics
- [x] **File**: `src/analysis/function_words.rs`
- [x] 2 unit tests

### 2.2 Character n-gram analysis ✓
- [x] Character-level n-grams (2, 3, 4, 5-grams)
- [x] Word-level bigrams
- [x] Top-500 frequency profiles per n-value
- [x] **File**: `src/analysis/ngrams.rs`
- [x] 5 unit tests

### 2.3 Vocabulary richness (corrected metrics) ✓
- [x] MATTR (Moving Average Type-Token Ratio, window=500)
- [x] Yule's K (frequency spectrum-based, length-independent)
- [x] Honoré's R (hapax-based richness)
- [x] Brunet's W (N^(V^-0.172))
- [x] Dis-legomena count + hapax/dis ratio
- [x] Word length distribution histogram
- [x] TTR flagged as length-dependent (kept for backward compat)
- [x] **File**: `src/analysis/lexical.rs`
### 2.4 Sentence structure enhancements ✓
- [x] Sentence type classification (declarative, interrogative, exclamatory)
- [x] Sentence opening word patterns (first-word frequency map)
- [x] Passive voice detection heuristic (be-form + past participle)
- [x] Sentence length distribution (word-count histogram)
- [x] Improved sentence splitting (abbreviation-aware)
- [x] **File**: `src/analysis/syntactic.rs`

### 2.5 Advanced punctuation & formatting ✓
- [x] Em-dash, en-dash, hyphen counting (including --- and -- patterns)
- [x] Ellipsis counting (… and ...)
- [x] Parenthetical expression frequency
- [x] Quote mark style (single vs double, smart vs straight)
- [x] Contraction detection (60+ common contractions)
- [x] Hedge word frequency (40+ markers: perhaps, maybe, somewhat, etc.)
- [x] Intensifier frequency (35+ markers: very, extremely, absolutely, etc.)
- [x] Paragraph structure: count, length variance, short paragraph ratio
- [x] **File**: `src/analysis/stylometric.rs`

### 2.6 Discourse markers & readability ✓
- [x] 40+ single-word discourse markers + 25+ multi-word phrases
- [x] Normalized frequency per 1000 words
- [x] Flesch-Kincaid Grade Level
- [x] Gunning Fog Index
- [x] Coleman-Liau Index
- [x] Automated Readability Index
- [x] Syllable counting heuristic
- [x] Topic keyword extraction REMOVED (confounds authorship signals)
- [x] **File**: `src/analysis/semantic.rs` (rewritten)
### 2.7 Profile & comparison updates ✓
- [x] AuthorProfile expanded to 22 averaged features (was 9)
- [x] Comparison engine uses 18 feature distances (was 6)
- [x] Report renderer shows all new analysis sections

---

## Phase 3: Distance Metrics & Comparison Engine ✓

### 3.1 Burrows' Delta ✓
- [x] Classic Delta (Manhattan distance on z-scores) — Burrows 2002
- [x] Cosine Delta (Wurzburg variant, default) — Evert et al. 2017
- [x] Euclidean Delta (Linear Delta)
- [x] Two-document and corpus-based z-normalization
- [x] Multi-candidate ranking by Cosine Delta
- [x] **File**: `src/identity/delta.rs`
- [x] 4 unit tests (identity, positivity, symmetry, ranking)

### 3.2 Statistical distance measures ✓
- [x] Cosine similarity + cosine distance
- [x] Manhattan (L1) distance
- [x] Euclidean (L2) distance
- [x] Kullback-Leibler divergence
- [x] Jensen-Shannon divergence (symmetric KL)
- [x] Chi-squared distance
- [x] **File**: `src/identity/distances.rs`
- [x] 6 unit tests (mathematical correctness)
### 3.3 Feature vector construction ✓
- [x] Unified FeatureVector from all analysis modules
- [x] 3 configurable feature sets: Minimal / Standard / Comprehensive
- [x] Z-score normalization via CorpusStats
- [x] Serializable (Serde)
- [x] **File**: `src/identity/features.rs`
- [x] 2 unit tests (corpus stats, feature set sizes)

### 3.4 Enhanced confidence scoring ✓
- [x] Delta-to-probability sigmoid calibration
- [x] Text-length penalty (6 tiers from <100 to 2000+ words)
- [x] Feature availability factor
- [x] **File**: `src/identity/confidence.rs` (rewritten)

---

## Phase 4: ML Pipeline (Classical) ✓

### 4.1 Feature engineering pipeline ✓
- [x] Automated text → feature vector extraction
- [x] Feature caching (compute once, reuse)
- [x] Feature metadata (name, cost, reliability)
- [x] **File**: `src/ml/features.rs`

### 4.2 Classical ML models in Rust ✓
- [x] SVM with RBF kernel (PAN competition winner) — via `linfa` feature flag
- [x] K-Nearest Neighbors (built-in, no external deps)
- [x] Logistic Regression (calibrated probabilities) — via `linfa` feature flag
- [x] **Dependency**: `linfa` + sub-crates (optional feature)
- [x] **Files**: `src/ml/models.rs`, `src/ml/pipeline.rs`
### 4.3 Evaluation framework ✓
- [x] K-fold cross-validation (stratified)
- [x] Metrics: accuracy, precision, recall, F1, EER
- [x] Confusion matrices with Display formatting
- [x] Per-author performance breakdown
- [x] **File**: `src/ml/evaluation.rs`

### 4.4 Python training scripts ✓
- [x] `training/requirements.txt` — scikit-learn, pandas, numpy, matplotlib, onnx
- [x] `training/extract_features.py` — call Rust binary, save CSV
- [x] `training/train_svm.py` — SVM with grid search, export ONNX
- [x] `training/train_ensemble.py` — gradient boosting + random forest, export ONNX
- [x] `training/evaluate.py` — evaluation with visualization (confusion matrix, ROC, EER)
- [ ] `training/README.md` — workflow instructions

### 4.5 ONNX inference in Rust ✓
- [x] Load ONNX models from Python training
- [x] Batch and single-document inference
- [x] Model registry (versioned model files with manifest.json)
- [x] **Dependency**: `ort` crate (optional `onnx` feature flag)
- [x] **File**: `src/ml/inference.rs`

---

## Phase 5: Data Pipeline & Datasets ✓

### 5.1 Dataset collection framework ✓
- [x] `data/README.md` — dataset documentation
- [x] Script to download PAN competition datasets (`data/scripts/download_pan.py`)
- [x] Script to process Project Gutenberg texts (integrated in download_pan.py)
- [x] Data format spec (one dir per author, text files within)
### 5.2 Data preprocessing ✓
- [x] Corpus loading and indexing
- [x] Deduplication (SHA-256 hash-based)
- [x] Train/validation/test split utilities (stratified)
- [x] Data quality scoring (word count, character diversity, sentence count)
- [x] **Files**: `src/data/mod.rs`, `src/data/corpus.rs`, `src/data/split.rs`

### 5.3 Benchmark datasets ✓
- [x] Small built-in benchmark (5 authors, 10 texts each)
- [x] Store in `tests/fixtures/benchmark/`
- [x] Automated benchmark runner (`tests/test_benchmark.rs`)

---

## Phase 6: File Forensics Enhancement ✓

### 6.1 Magic byte detection ✓
- [x] Read first N bytes, match known signatures (PDF, ZIP, RTF, BOM)
- [x] PDF (%PDF-), ZIP (PK), DOCX (ZIP+word/document.xml), RTF ({\\rtf)
- [x] Handle extension vs. actual format mismatch with warnings
- [x] Encoding detection (UTF-8, UTF-16 LE/BE, BOM, Windows-1252)
- [x] **File**: `src/forensics/format.rs` (rewritten)

### 6.2 Document metadata extraction ✓
- [x] DOCX metadata (author, title, revision count) via ZIP+XML parsing
- [x] PDF metadata (creator, producer, dates, page count) via lopdf
- [x] Custom metadata key-value extraction
- [x] **File**: `src/forensics/metadata.rs` (rewritten)

### 6.3 Tampering detection ✓
- [x] Date inconsistency detection (created > modified, future dates, metadata vs filesystem)
- [x] Tool fingerprint analysis (Microsoft Word, LibreOffice, Google Docs, LaTeX)- [x] Hidden content detection (zero-width characters, unusual whitespace)
- [x] Metadata anomaly detection (revision count vs file size)
- [x] Risk scoring (0.0–1.0)
- [x] **File**: `src/forensics/tampering.rs`

### 6.4 Enhanced timeline ✓
- [x] DOCX revision history extraction (interpolated from revision count)
- [x] Multi-source timeline correlation (filesystem + document metadata)
- [x] Timeline anomaly scoring with explanations
- [x] Future date detection, creation-after-modification detection
- [x] **File**: `src/forensics/timeline.rs` (rewritten)

---

## Phase 7: Anomaly & Multi-Author Detection ✓

### 7.1 Sliding window analysis ✓
- [x] Configurable window size (default: 500 words, slide by 100)
- [x] Per-window feature extraction with any feature set
- [x] Anomaly score per window (RMS z-score)
- [x] Top deviating features per window
- [x] **File**: `src/identity/anomaly.rs` (rewritten)

### 7.2 Style change point detection ✓
- [x] Binary segmentation algorithm (recursive)
- [x] Change points with confidence scores (sigmoid-calibrated)
- [x] Shift type classification (gradual vs abrupt)
- [x] Key features driving each change point
- [x] **File**: `src/identity/changepoint.rs`
### 7.3 Multi-author segmentation ✓
- [x] Per-segment author attribution via feature distance
- [x] Multiple candidate profile support with confidence margins
- [x] Author count estimation
- [x] Full pipeline: window analysis → change points → segments → attribution
- [x] **File**: `src/identity/segmentation.rs`

---

## Phase 8: Reporting & Output ✓

### 8.1 Multi-format output ✓
- [x] `--format json|text|html` CLI flag on `analyze` and `forensics` commands
- [x] JSON report with full feature vectors (serde serialization)
- [x] **File**: `src/scoring/report.rs` (rewritten with render_format dispatcher)

### 8.2 HTML report ✓
- [x] Self-contained HTML with embedded CSS (no external dependencies)
- [x] Feature comparison tables with bar charts
- [x] Confidence visualization (color-coded high/medium/low)
- [x] Tampering findings table with severity highlighting
- [x] **File**: `src/scoring/html_report.rs`

### 8.3 Audit trail ✓
- [x] Software version, input file hash, timestamp
- [x] Complete analysis parameters (config, feature set)
- [x] ISO 8601 timestamps for reproducibility
- [x] **File**: `src/scoring/engine.rs` (AuditTrail struct)

---
## Phase 9: Security Hardening ✓

### 9.1 Input validation ✓
- [x] File size limits (configurable, default 100MB)
- [x] Zip bomb detection (compression ratio + decompressed size limits)
- [x] XML bomb detection (entity count, recursive entity patterns)
- [x] Path traversal prevention (archive entries, null bytes, drive letters)
- [x] Filename sanitization
- [x] Wired into extraction pipeline
- [x] **File**: `src/utils/validation.rs`

### 9.2 Fuzz testing ✓
- [x] `cargo-fuzz` targets: text extraction, analysis, validation, profile deserialization
- [x] **Directory**: `fuzz/fuzz_targets/`

### 9.3 Property-based testing ✓
- [x] `proptest` crate for determinism, serialization roundtrips
- [x] Analysis determinism (same input → same output)
- [x] Feature vector JSON roundtrip
- [x] Validation never panics on arbitrary input
- [x] Distance metric properties (self-similarity, symmetry)
- [x] **File**: `tests/test_proptest.rs`

---

## Phase 10: Performance ✓

### 10.1 Parallel processing ✓
- [x] `rayon` for parallel feature extraction (6 analysis layers run concurrently via `rayon::join`)
- [x] Parallel multi-file processing (`build_profile` uses `par_iter` for sample extraction + analysis)
- [x] Parallel multi-candidate comparison (`rank_candidates` uses `par_iter`, window analysis parallelized)
### 10.2 Memory optimization ✓
- [x] Streaming text processing for large documents (memmap2-backed plaintext extraction above 1 MB)
- [x] Memory-mapped file I/O (`memmap2`) for format detection and large plaintext files

### 10.3 Benchmarks ✓
- [x] Per-module benchmarks (lexical, syntactic, semantic, stylometric, function_words, ngrams)
- [x] End-to-end pipeline benchmark (analyze_text at 100/500/1000/5000 words)
- [x] Memory usage tracking (feature vector size scaling benchmark)
- [x] Performance regression detection in CI (benchmark compile check + PR artifact upload)

---

## Phase 11: Testing Strategy

### Unit tests per module
- [ ] `src/extraction/` — one test per format + edge cases
- [ ] `src/analysis/` — known-value tests for every metric
- [ ] `src/identity/` — profile roundtrip, delta correctness, confidence calibration
- [ ] `src/forensics/` — metadata, tampering detection
- [ ] `src/ml/` — model train/predict roundtrip, evaluation metrics

### Integration tests
- [x] Lexical analysis basic tests (3 tests)
- [x] Crypto hash known-value tests (2 tests)
- [ ] Full pipeline: file → extract → analyze → compare → report
- [ ] Profile workflow: samples → profile → compare → result
- [ ] CLI integration tests
- [ ] Multi-format document tests

### Regression tests
- [ ] Golden output files for known inputs
- [ ] Benchmark dataset accuracy tracking
---

## Phase 12: DOCX Forensic Parser ✓

> Deep forensic analysis of Microsoft Word documents via RSID traces, formatting consistency, and construction profiling.

### 12.1 RSID analysis ✓
- [x] Extract paragraph-level and run-level RSIDs from document.xml
- [x] Detect contiguous RSID blocks (`RsidBlock`) — same-RSID paragraph sequences
- [x] Detect paste events — large single-RSID blocks indicating bulk insertion
- [x] RSID diversity scoring (unique RSIDs / paragraph count)
- [x] RSID concentration analysis (dominant RSID percentage)
- [x] **File**: `src/forensics/docx/rsid.rs`

### 12.2 Formatting consistency analysis ✓
- [x] Cross-paragraph formatting comparison
- [x] Style uniformity scoring
- [x] **File**: `src/forensics/docx/formatting.rs`

### 12.3 Structural forensics ✓
- [x] Document structure analysis (sections, headings, body)
- [x] **File**: `src/forensics/docx/structure.rs`

### 12.4 Document Construction Profile ✓
- [x] `ConstructionPattern` enum: Organic, BulkInsertion, Hybrid, Insufficient
- [x] Combines RSID diversity, editing time, formatting consistency, revision scatter
- [x] Classification with confidence scoring
- [x] **File**: `src/forensics/docx/profile.rs`
---

## Phase 13: Register-Aware Text Analysis ✓

> Classifies text into linguistic registers and compares metrics against per-register baselines.

### 13.1 Register classifier ✓
- [x] 7 main registers: Academic, Literary, Technical, Journalistic, Professional, Casual, Educational
- [x] 20+ subtypes (e.g., Academic → Legal/Scientific/Humanities)
- [x] Confidence-scored classification
- [x] **File**: `src/analysis/register.rs`

### 13.2 Per-register baselines ✓
- [x] Expected mean and stddev for metrics within each register
- [x] Z-score comparison (actual vs expected)
- [x] Assessment levels: Expected, Atypical, Anomalous
- [x] `RegisterBaselineReport` with counts per assessment level
- [x] **File**: `src/analysis/baselines.rs`

---

## Phase 14: Scoring Model ✓

> Unified scoring with Authorship Confidence Score, Process Integrity Index, and anomaly flags.

### 14.1 Authorship Confidence Score (ACS) ✓
- [x] Score 0-100 with confidence interval
- [x] Evidence source tracking
- [x] Component score breakdown
- [x] Framed as positive authorship claim, NOT "AI probability"
- [x] **File**: `src/scoring/acs.rs`
### 14.2 Process Integrity Index (PII) ✓
- [x] Score 0-100 measuring completeness of process evidence
- [x] Considers metadata, RSID richness, formatting, revision count
- [x] Tells evaluators how much to trust the ACS
- [x] **File**: `src/scoring/pii.rs`

### 14.3 Anomaly flag system ✓
- [x] Localized anomaly flags with type, location, severity
- [x] Recommended actions per flag
- [x] Framed as questions, not accusations
- [x] **File**: `src/scoring/anomalies.rs`

### 14.4 Unified score aggregation ✓
- [x] `UnifiedScore` combining forensics, analysis, comparison, register, baselines, ACS, PII, anomalies
- [x] `AuditTrail` for reproducibility (version, file hash, timestamp, feature set)
- [x] **File**: `src/scoring/engine.rs`

---

## Phase 15: Content Design Layer ✓

> Evaluator-facing reports with reliability disclosures and responsible framing.

### 15.1 Evaluator report generation ✓
- [x] Executive summary, register context, authorship assessment, integrity assessment
- [x] Anomaly narrative (human-readable)
- [x] Recommended actions
- [x] **File**: `src/scoring/content_design.rs`

### 15.2 Reliability disclosures ✓
- [x] Explicit disclosure of what the tool can and cannot do
- [x] Framing principles enforced: present evidence, never claim AI detection, disclose limitations
- [x] Reports suitable for academic evaluators and forensic experts
---

## Phase 16: Full Pipeline Integration ✓

> Wires all 6 analysis layers into end-to-end workflow.

- [x] Layer 1: File forensics (`forensics::examine`)
- [x] Layer 2: Text extraction + stylometric analysis
- [x] Layer 3: Identity comparison (optional author profile)
- [x] Layer 4: Register classification + baseline comparison
- [x] Layer 5: DOCX forensics (if applicable)
- [x] Layer 6: Scoring model aggregation + report rendering
- [x] `analyze_with_format()` in **lib.rs**

---

## Phase 17: Feature Importance & Explainability ✓

> Explains WHY the system reached its score — which features drove the decision.

### 17.1 Feature importance ranking ✓
- [x] Ranked features by discriminative power with direction (Above/Below/Match)
- [x] Supporting features vs diverging features breakdown
- [x] Agreement ratio computation
- [x] **File**: `src/identity/explainability.rs`

### 17.2 Decision narratives ✓
- [x] Human-readable explanation per scoring decision
- [x] Per-feature interpretation (query value, profile value, distance, meaning)
- [x] **File**: `src/identity/explainability.rs`
### 17.3 Feature vector expansion ✓
- [x] 3 configurable sets: Minimal (~200), Standard (~400), Comprehensive (~740) features
- [x] `FeatureSet` enum: Minimal (function words), Standard (+ scalar metrics), Comprehensive (+ character n-grams)
- [x] **File**: `src/identity/features.rs`

---

## Phase 18: Multi-Candidate Authorship Ranking ✓

> Ranks multiple author candidates against a query document.

- [x] `RankingResult` with sorted candidates, top margin, definitiveness
- [x] `CandidateRanking` with rank position, author name, relative score (0-1)
- [x] Per-candidate explanation with comparison details
- [x] Margin between top candidates indicates ranking confidence
- [x] **File**: `src/identity/ranking.rs`

---

## Phase 19: CLI Rank Command ✓

> Multi-candidate authorship comparison from the command line.

- [x] `provenance rank --file doc.txt --profiles a.json b.json c.json`
- [x] Text, JSON, and HTML output formats
- [x] Comparative benchmarking across candidates

---

## Phase 20: Batch Analysis ✓

> Directory-level document processing with parallel execution.

- [x] `provenance batch --dir ./docs/ [--profile author.json]`
- [x] Rayon-parallelized multi-file processing
- [x] JSON and text summary output
- [x] `batch_analyze()` function in **lib.rs**
---

## Phase 21: Provenance Certificate ✓

> Tamper-evident attestation of analysis results using Merkle trees and Ed25519 signatures.

### 21.1 Certificate structure ✓
- [x] `ProvenanceCertificate` — version, certificate_id, issued_at, software_version
- [x] `DocumentAttestation` — file name, SHA-256 hash, size, format, word count
- [x] `AnalysisSummary` — ACS score, PII score, register, anomaly count, construction pattern
- [x] `ProcessSummary` — forensics/profile availability, RSID sessions, formatting consistency
- [x] **File**: `src/crypto/certificate.rs`

### 21.2 Cryptographic binding ✓
- [x] Merkle tree binds all certificate fields
- [x] Ed25519 signature over Merkle root
- [x] Anyone with public key can verify no field has been modified
- [x] **Files**: `src/crypto/merkle.rs`, `src/crypto/keys.rs`, `src/crypto/verify.rs`

### 21.3 CLI commands ✓
- [x] `provenance generate-key` — create Ed25519 signing keypair
- [x] `provenance generate-certificate --file doc --key key.pem`
- [x] `provenance verify-certificate --certificate cert.json [--document doc]`

---

## Phase 22-23: Process Capture & Adversarial Robustness ✓

### 22.1 Process capture agents ✓
- [x] `CaptureEvent` with 20+ event types (Keystroke, Paste, Delete, Undo, Save, FormatChange, AiSuggestionAccepted, etc.)
- [x] `CaptureSession` storing ordered events from one writing period
- [x] `ProcessMetrics` — typing speed, paste ratio, undo rate, focus loss, AI content ratio, revision intensity
- [x] Scrivener project watcher (`provenance capture-watch --project ./mybook.scriv`)
- [x] Import capture sessions (`provenance import-capture --file session.json`)
- [x] **Files**: `src/capture/events.rs`, `src/capture/session.rs`, `src/capture/metrics.rs`, `src/capture/scrivener.rs`
### 23.1 Adversarial evasion framework ✓
- [x] `EvasionScenario` — attack types, targeted signals, difficulty, prevalence
- [x] Attack types: Humanizer, ManualEditing, PromptEngineering, StyleTransfer, Paraphrasing, BackTranslation, MetadataStripping, DocumentReconstruction
- [x] Robustness matrix: Provenance resilience per attack type
- [x] **File**: `src/adversarial/evasion.rs`

### 23.2 Humanizer detection ✓
- [x] Detects artifacts from humanizer tools (Undetectable.ai, QuillBot, StealthGPT, etc.)
- [x] Confidence-scored detection with suspected tool identification
- [x] `provenance detect-humanizer --file doc.txt`
- [x] **File**: `src/adversarial/humanizer.rs`

### 23.3 Bias audit ✓
- [x] Bias audit across demographics and registers
- [x] `provenance bias-audit`
- [x] **File**: `src/adversarial/bias.rs`

---

## Phase 24: API and Platform Layer ✓

> REST API built with Axum for programmatic access to all Provenance functionality.

### 24.1 HTTP server ✓
- [x] Axum-based server with middleware (CORS, tracing, body limits)
- [x] Feature-gated (`--features server`)
- [x] Configurable host, port, rate limits, max body size
- [x] **File**: `src/api/server.rs`
### 24.2 API endpoints ✓
- [x] `POST /api/v1/analyze` — synchronous document analysis
- [x] `POST /api/v1/analyze/async` — async job submission
- [x] `POST /api/v1/forensics` — DOCX forensics
- [x] `GET /api/v1/jobs` / `GET /api/v1/jobs/{id}` — job status/results
- [x] `POST /api/v1/profile/build` — build author profile
- [x] `POST /api/v1/profile/compare` — compare document against profile
- [x] `POST /api/v1/certificate/generate` — generate certificate
- [x] `POST /api/v1/certificate/verify` — verify certificate
- [x] `POST /api/v1/detect-humanizer` — humanizer artifact detection
- [x] `GET /api/v1/adversarial/robustness` — robustness matrix
- [x] `GET /api/v1/adversarial/bias` — bias audit
- [x] **File**: `src/api/handlers.rs`

### 24.3 Authentication & rate limiting ✓
- [x] API key authentication (optional, configurable)
- [x] Per-key rate limiting (requests per minute)
- [x] **Files**: `src/api/auth.rs`, `src/api/jobs.rs`

---

## Phase 25: Enterprise Integrations ✓

> Institution-wide deployment via LMS, publishing, and workspace connectors.

### 25.1 LMS integration (LTI 1.3) ✓
- [x] `LtiPlatformRegistration` — issuer, auth/token endpoints, JWKS URI, client ID
- [x] Supported LMS: Canvas, Blackboard, Moodle, Brightspace, Sakai, Generic
- [x] LTI 1.3 protocol with AGS (grade passback) and NRPS (roster access)
- [x] **File**: `src/enterprise/lms.rs`
### 25.2 Batch platform API ✓
- [x] `BatchSubmission` with entries, webhook callbacks, batch options
- [x] Configurable concurrency, continue-on-error, export format (CSV/JSON/XML)
- [x] Usage billing integration
- [x] **File**: `src/enterprise/platform.rs`

### 25.3 Publishing system connectors ✓
- [x] Supported systems: Editorial Manager, ScholarOne, OJS, ManuscriptManager, WordPress, CustomWebhook
- [x] Webhook-based integration with trigger events (ManuscriptSubmitted, RevisionSubmitted, etc.)
- [x] **File**: `src/enterprise/publishing.rs`

### 25.4 Google Workspace integration ✓
- [x] Domain-wide delegation or individual OAuth for Google Workspace for Education
- [x] Scopes: Docs, Drive, Classroom (student submissions), user info
- [x] FERPA-compliant data handling for educational institutions
- [x] **File**: `src/enterprise/workspace.rs`

---

## Dependency Roadmap

| Phase | Add | Remove |
|-------|-----|--------|
| 0 | `toml` | `tokio`, `zip`, `xml-rs` |
| 1 | `pulldown-cmark`, `lopdf`, `scraper`, `zip`, `rtf-parser`, `mailparse`, `csv`, `regex`, `encoding_rs`, `unicode-normalization` | |
| 3 | `ndarray` | |
| 4 | `linfa` + sub-crates (optional), `ort` (optional) | |
| 10 | `rayon`, `memmap2` | |
| 12 | `quick-xml` (DOCX parsing) | |
| 21 | `ed25519-dalek`, `rand`, `base64` | |
| 24 | `axum`, `tokio`, `tower`, `tower-http`, `uuid` (optional, feature-gated) | |

---

*This roadmap is research-validated against published stylometry literature. All 25 phases are implemented and merged to Main. Feature priorities reflect proven effectiveness, not intuitive appeal.*