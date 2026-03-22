# PROVENANCE — Research-Validated Development Roadmap

**Last Updated:** 2026-03-22
**Status:** Foundation Phase — Phase 0 Complete
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
- [x] Tool fingerprint analysis (Microsoft Word, LibreOffice, Google Docs, LaTeX)
- [x] Hidden content detection (zero-width characters, unusual whitespace)
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

## Phase 9: Security Hardening

### 9.1 Input validation
- [ ] File size limits (configurable, default 100MB)
- [ ] Zip bomb detection
- [ ] XML bomb detection
- [ ] Path traversal prevention

### 9.2 Fuzz testing
- [ ] `cargo-fuzz` targets for file parsing, text extraction, feature extraction, profile deser
- [ ] **Directory**: `fuzz/`

### 9.3 Property-based testing
- [ ] `proptest` crate for determinism, normalization, serialization roundtrips

---

## Phase 10: Performance

### 10.1 Parallel processing
- [ ] `rayon` for parallel feature extraction
- [ ] Parallel multi-file processing
- [ ] Parallel multi-candidate comparison

### 10.2 Memory optimization
- [ ] Streaming text processing for large documents
- [ ] Memory-mapped file I/O (`memmap2`)

### 10.3 Benchmarks
- [ ] Per-module benchmarks
- [ ] End-to-end pipeline benchmark
- [ ] Memory usage tracking
- [ ] Performance regression detection in CI

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

## Dependency Roadmap

| Phase | Add | Remove |
|-------|-----|--------|
| 0 | `toml` | `tokio`, `zip`, `xml-rs` |
| 1 | `pulldown-cmark`, `lopdf`, `scraper`, `zip`, `rtf-parser`, `mailparse`, `csv`, `regex`, `encoding_rs`, `unicode-normalization` | |
| 3 | `ndarray` | |
| 4 | `linfa` + sub-crates (optional), `ort` (optional) | |
| Future | `whatlang`, `msg_parser`, `proptest`, `rayon`, `memmap2` | |

---

*This roadmap is research-validated against published stylometry literature. Feature priorities reflect proven effectiveness, not intuitive appeal.*
