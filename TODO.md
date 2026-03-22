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

## Phase 2: Core Stylometric Features (Research-Validated)

> Ordered by proven effectiveness in authorship attribution research.

### 2.1 Function word analysis (HIGHEST PRIORITY)
- [ ] Curated list of 300+ English function words
- [ ] Frequency distribution (normalized per 1000 words)
- [ ] Most reliable stylometric feature class — used unconsciously, hard to fake
- [ ] **File**: `src/analysis/function_words.rs`
- [ ] Tests: known distributions against reference texts

### 2.2 Character n-gram analysis
- [ ] Character-level n-grams (2-gram through 5-gram)
- [ ] Word-level bigrams and trigrams
- [ ] N-gram frequency profiles (top-N most frequent)
- [ ] Language-independent, captures subword patterns
- [ ] **File**: `src/analysis/ngrams.rs`
- [ ] Tests: known n-gram counts for sample texts

### 2.3 Vocabulary richness (corrected metrics)
- [ ] MATTR (Moving Average Type-Token Ratio) — window-based, length-independent
- [ ] HD-D (Hypergeometric Distribution D) — sample-size independent
- [ ] Yule's K — vocabulary richness independent of text length
- [ ] Honoré's R — based on hapax legomena ratio
- [ ] Flag existing TTR as length-dependent (keep for backward compat)
- [ ] **File**: `src/analysis/lexical.rs` (enhance)
- [ ] Tests: verify length-independence

### 2.4 Sentence structure enhancements
- [ ] Sentence type classification (declarative, interrogative, exclamatory, imperative)
- [ ] Sentence opening word patterns
- [ ] Passive voice detection heuristic
- [ ] Sentence length distribution histogram
- [ ] **File**: `src/analysis/syntactic.rs` (enhance)

### 2.5 Advanced punctuation & formatting
- [ ] Em-dash vs en-dash vs hyphen usage
- [ ] Parenthetical expression frequency
- [ ] Ellipsis patterns
- [ ] Quote mark style (single vs double)
- [ ] Contraction usage (don't vs do not)
- [ ] Hedge word frequency (maybe, perhaps, somewhat)
- [ ] Intensifier frequency (very, extremely, absolutely)
- [ ] **File**: `src/analysis/stylometric.rs` (enhance)

### 2.6 Discourse marker analysis (replaces topic keywords)
- [ ] Catalog: however, therefore, moreover, in addition, etc.
- [ ] Frequency and position patterns
- [ ] Transitional phrases ARE stylometric (unlike topic keywords)
- [ ] Readability scores: Flesch-Kincaid, Gunning Fog, Coleman-Liau
- [ ] **File**: `src/analysis/semantic.rs` (rewrite)

---

## Phase 3: Distance Metrics & Comparison Engine

### 3.1 Burrows' Delta (FOUNDATIONAL)
- [ ] Select N most frequent words from corpus (typically 100-500)
- [ ] Compute z-scores for each word frequency
- [ ] Classic Delta — Manhattan distance (Burrows 2002)
- [ ] Cosine Delta — Wurzburg variant (often outperforms classic)
- [ ] Eder's Delta — more features with weighting
- [ ] **File**: `src/identity/delta.rs`
- [ ] **Dependency**: `ndarray` crate
- [ ] Tests: validate against known Federalist Papers attributions

### 3.2 Statistical distance measures
- [ ] Cosine similarity
- [ ] Kullback-Leibler divergence
- [ ] Jensen-Shannon divergence (symmetric KL)
- [ ] Chi-squared distance
- [ ] **File**: `src/identity/comparison.rs` (enhance)
- [ ] Tests: mathematical correctness with known inputs

### 3.3 Feature vector construction
- [ ] Unified feature vector from all analysis modules
- [ ] Feature normalization: z-score, min-max scaling
- [ ] Configurable feature sets: minimal / standard / comprehensive
- [ ] Serialization for profile storage
- [ ] **File**: `src/identity/features.rs`

### 3.4 Enhanced confidence scoring
- [ ] Likelihood ratio framework
- [ ] Confidence intervals (not just point estimates)
- [ ] Feature contribution breakdown
- [ ] Text-length adjusted confidence curves
- [ ] **File**: `src/identity/confidence.rs` (rewrite)

---

## Phase 4: ML Pipeline (Classical)

### 4.1 Feature engineering pipeline
- [ ] Automated text → feature vector extraction
- [ ] Feature caching (compute once, reuse)
- [ ] Feature metadata (name, cost, reliability)
- [ ] **File**: `src/ml/features.rs`

### 4.2 Classical ML models in Rust
- [ ] SVM with RBF kernel (PAN competition winner)
- [ ] Random Forest (interpretable baseline)
- [ ] Logistic Regression (calibrated probabilities)
- [ ] **Dependency**: `linfa` + sub-crates
- [ ] **Files**: `src/ml/models.rs`, `src/ml/pipeline.rs`

### 4.3 Evaluation framework
- [ ] K-fold cross-validation
- [ ] Metrics: accuracy, precision, recall, F1, ROC-AUC, EER
- [ ] Confusion matrices
- [ ] Per-author performance breakdown
- [ ] **File**: `src/ml/evaluation.rs`

### 4.4 Python training scripts
- [ ] `training/requirements.txt` — scikit-learn, pandas, numpy, matplotlib, onnx
- [ ] `training/extract_features.py` — call Rust binary, save CSV
- [ ] `training/train_svm.py` — SVM with grid search, export ONNX
- [ ] `training/train_ensemble.py` — gradient boosting, export ONNX
- [ ] `training/evaluate.py` — evaluation with visualization
- [ ] `training/README.md` — workflow instructions

### 4.5 ONNX inference in Rust
- [ ] Load ONNX models from Python training
- [ ] Batch and single-document inference
- [ ] Model registry (versioned model files)
- [ ] **Dependency**: `ort` crate
- [ ] **File**: `src/ml/inference.rs`

---

## Phase 5: Data Pipeline & Datasets

### 5.1 Dataset collection framework
- [ ] `data/README.md` — dataset documentation
- [ ] Script to download PAN competition datasets
- [ ] Script to process Project Gutenberg texts
- [ ] Data format spec (one dir per author, text files within)

### 5.2 Data preprocessing
- [ ] Corpus loading and indexing
- [ ] Deduplication
- [ ] Train/validation/test split utilities
- [ ] Data quality scoring (min words, encoding)
- [ ] **Files**: `src/data/mod.rs`, `src/data/corpus.rs`

### 5.3 Benchmark datasets
- [ ] Small built-in benchmark (5 authors, 10 texts each)
- [ ] Store in `tests/fixtures/benchmark/`
- [ ] Automated benchmark runner

---

## Phase 6: File Forensics Enhancement

### 6.1 Magic byte detection
- [ ] Read first N bytes, match known signatures
- [ ] PDF (%PDF-), ZIP (PK), DOCX (ZIP+[Content_Types].xml)
- [ ] Handle extension vs. actual format mismatch
- [ ] **File**: `src/forensics/format.rs` (enhance)

### 6.2 Document metadata extraction
- [ ] DOCX metadata (author, title, revision count) via docx-rs
- [ ] PDF metadata (creator, producer, dates) via lopdf
- [ ] **File**: `src/forensics/metadata.rs` (enhance)

### 6.3 Tampering detection
- [ ] Date inconsistency detection
- [ ] Tool fingerprint analysis
- [ ] Hidden content detection
- [ ] **File**: `src/forensics/tampering.rs`

### 6.4 Enhanced timeline
- [ ] DOCX revision history extraction
- [ ] Multi-source timeline correlation
- [ ] Timeline anomaly scoring
- [ ] **File**: `src/forensics/timeline.rs` (enhance)

---

## Phase 7: Anomaly & Multi-Author Detection

### 7.1 Sliding window analysis
- [ ] Configurable window size (default: 500 words, slide by 100)
- [ ] Per-window feature extraction
- [ ] Anomaly score per window position
- [ ] **File**: `src/identity/anomaly.rs` (rewrite)

### 7.2 Style change point detection
- [ ] Binary segmentation algorithm
- [ ] Change points with confidence scores
- [ ] **File**: `src/identity/changepoint.rs`

### 7.3 Multi-author segmentation
- [ ] Per-segment author attribution
- [ ] Multiple candidate profile support
- [ ] **File**: `src/identity/segmentation.rs`

---

## Phase 8: Reporting & Output

### 8.1 Multi-format output
- [ ] `--format json|text|html` CLI flag
- [ ] JSON report with full feature vectors
- [ ] **File**: `src/scoring/report.rs` (enhance)

### 8.2 HTML report
- [ ] Self-contained HTML with embedded CSS
- [ ] Feature comparison tables
- [ ] Confidence visualization
- [ ] Anomaly heatmap
- [ ] **File**: `src/scoring/html_report.rs`

### 8.3 Audit trail
- [ ] Software version, config hash, input file hash
- [ ] Complete analysis parameters
- [ ] Reproducibility guarantee

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
| 4 | `linfa` + sub-crates, `ort` | |
| Future | `whatlang`, `msg_parser`, `proptest`, `rayon`, `memmap2` | |

---

*This roadmap is research-validated against published stylometry literature. Feature priorities reflect proven effectiveness, not intuitive appeal.*
