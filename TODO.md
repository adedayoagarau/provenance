# PROVENANCE — Development Roadmap & Task Tracker

**Last Updated:** 2026-03-22
**Status:** Foundation Phase

---

## Phase 1: Project Foundation

### 1.1 Project Setup
- [x] Initialize Git repository
- [x] Create PROVENANCE.md handoff document
- [x] Initialize Cargo project with dependencies
- [x] Create module structure (forensics, analysis, identity, scoring, crypto, utils)
- [x] Define error type hierarchy (`ProvenanceError`)
- [x] Set up CLI with clap (analyze, profile, forensics subcommands)
- [x] Configure structured logging with tracing
- [x] Set up configuration management
- [x] Create .gitignore
- [x] Write initial integration tests (lexical, crypto)
- [x] Set up benchmark harness with criterion

### 1.2 CI/CD Pipeline
- [ ] GitHub Actions workflow for build + test
- [ ] Clippy linting in CI
- [ ] Rustfmt formatting checks
- [ ] Code coverage reporting
- [ ] Dependency audit (cargo-audit)
- [ ] Release build pipeline

### 1.3 Documentation Infrastructure
- [ ] Rustdoc configuration and generation
- [ ] Architecture decision records (ADR) directory
- [ ] Contributing guidelines
- [ ] License file (MIT)
- [ ] Changelog setup (keep-a-changelog format)

---

## Phase 2: File Forensics (Layer 1)

### 2.1 Metadata Extraction
- [ ] Basic filesystem metadata (size, dates, permissions)
  - [x] Implement `metadata::extract()` for filesystem metadata
  - [ ] Add Windows-specific metadata support (NTFS streams)
  - [ ] Add macOS-specific metadata support (extended attributes)
- [ ] Office document metadata extraction
  - [ ] DOCX metadata (author, title, revision count, creation/modification dates)
  - [ ] PDF metadata (creator, producer, creation date, modification date)
  - [ ] ODT metadata (OpenDocument format)
  - [ ] XLSX/PPTX metadata
- [ ] Image metadata (EXIF) for embedded images
- [ ] Email metadata (.eml, .msg) — sender, headers, routing
- [ ] Embedded object inventory (images, macros, fonts in documents)

### 2.2 File Format Analysis
- [x] Extension-based type detection
- [ ] Magic byte / file signature detection
  - [ ] PDF signature validation (%PDF-)
  - [ ] ZIP-based format detection (DOCX, XLSX, ODT, EPUB)
  - [ ] Plain text encoding detection (UTF-8, UTF-16, Latin-1, etc.)
  - [ ] BOM (Byte Order Mark) handling
- [ ] MIME type inference
- [ ] Nested/compound document detection (e.g., ZIP containing DOCX)
- [ ] Encoding validation and normalization
- [ ] File structure integrity validation per format

### 2.3 Integrity Verification
- [x] SHA-256 hash computation
- [ ] MD5 hash (for legacy compatibility/cross-reference)
- [ ] BLAKE3 hash (for performance-critical paths)
- [ ] File truncation detection
- [ ] Structural consistency checks per file type
  - [ ] PDF cross-reference table validation
  - [ ] DOCX ZIP structure validation
  - [ ] XML well-formedness in document formats
- [ ] Tampering indicators
  - [ ] Metadata date inconsistency detection
  - [ ] Tool fingerprint analysis (which software created/modified)
  - [ ] Hidden content detection (white text, tiny fonts, etc.)

### 2.4 Document Timeline Construction
- [x] Basic timeline from filesystem dates
- [ ] Revision history extraction from DOCX
- [ ] PDF incremental save history analysis
- [ ] Edit time tracking (total editing time from metadata)
- [ ] Timeline anomaly detection (e.g., creation date after modification date)
- [ ] Multi-file timeline correlation (cross-document analysis)

---

## Phase 3: Text Extraction Pipeline

### 3.1 Plain Text Extraction
- [x] Raw file read for .txt files
- [ ] Markdown stripping (extract text, remove markup)
- [ ] HTML text extraction (strip tags, decode entities)
- [ ] Whitespace normalization
- [ ] Unicode normalization (NFC/NFD)

### 3.2 Document Format Extraction
- [ ] PDF text extraction
  - [ ] Basic text stream extraction
  - [ ] Multi-column layout handling
  - [ ] Header/footer separation
  - [ ] Footnote/endnote extraction
  - [ ] Table text extraction
  - [ ] Embedded font decoding
- [ ] DOCX text extraction
  - [ ] Paragraph text extraction
  - [ ] Style/formatting preservation metadata
  - [ ] Comment extraction (as separate analysis layer)
  - [ ] Track changes extraction
  - [ ] Header/footer extraction
  - [ ] Table content extraction
- [ ] RTF text extraction
- [ ] ODT text extraction
- [ ] EPUB text extraction

### 3.3 Text Preprocessing
- [ ] Sentence boundary detection
- [ ] Paragraph boundary detection
- [ ] Section/chapter detection
- [ ] Quote/citation identification and separation
- [ ] Boilerplate removal (headers, footers, page numbers)
- [ ] Language detection (for multi-language documents)

---

## Phase 4: Text Analysis Engine (Layer 2)

### 4.1 Lexical Analysis
- [x] Word tokenization (unicode-segmentation)
- [x] Word frequency distribution
- [x] Type-token ratio (TTR)
- [x] Hapax legomena count and ratio
- [x] Average word length
- [ ] Vocabulary richness metrics
  - [ ] Yule's K measure
  - [ ] Simpson's Diversity Index
  - [ ] Brunet's W
  - [ ] Honoré's R
  - [ ] Sichel's S
- [ ] Word length distribution histogram
- [ ] N-gram analysis (bigrams, trigrams)
  - [ ] Character n-grams
  - [ ] Word n-grams
  - [ ] N-gram frequency profiles
- [ ] Rare word detection and profiling
- [ ] Domain-specific vocabulary detection
- [ ] Spelling variant tracking (British vs. American, etc.)
- [ ] Contraction usage patterns

### 4.2 Syntactic Analysis
- [x] Sentence count and average length
- [x] Sentence length variance
- [x] Words per sentence
- [ ] Sentence type classification
  - [ ] Declarative vs. interrogative vs. exclamatory vs. imperative
  - [ ] Simple vs. compound vs. complex vs. compound-complex
- [ ] Clause depth analysis
- [ ] Part-of-speech tagging
  - [ ] POS distribution profiling
  - [ ] POS n-gram patterns
  - [ ] Adjective/adverb usage ratios
  - [ ] Verb tense distribution
- [ ] Dependency parsing (sentence structure trees)
- [ ] Passive vs. active voice ratio
- [ ] Subordination index
- [ ] Coordination index
- [ ] Sentence opening patterns (how sentences begin)

### 4.3 Semantic Analysis
- [x] Paragraph count
- [x] Keyword extraction (frequency-based, stop word filtered)
- [ ] Topic modeling
  - [ ] Latent Dirichlet Allocation (LDA)
  - [ ] Topic coherence scoring
  - [ ] Topic transition mapping
- [ ] Argument structure analysis
  - [ ] Claim identification
  - [ ] Evidence/support pattern detection
  - [ ] Counter-argument recognition
- [ ] Semantic coherence scoring
  - [ ] Adjacent sentence similarity
  - [ ] Paragraph-level coherence
  - [ ] Document-level thematic consistency
- [ ] Discourse marker analysis
  - [ ] Transitional phrase catalog
  - [ ] Connective type distribution
  - [ ] Logical flow mapping
- [ ] Sentiment patterns across document sections
- [ ] Readability scores (Flesch-Kincaid, Gunning Fog, Coleman-Liau)

### 4.4 Stylometric Analysis
- [x] Punctuation frequency distribution
- [x] Average paragraph length
- [x] Exclamation/question/comma/semicolon ratios
- [ ] Advanced punctuation patterns
  - [ ] Em-dash vs. en-dash vs. hyphen usage
  - [ ] Parenthetical expression frequency
  - [ ] Ellipsis usage patterns
  - [ ] Colon vs. semicolon preference
  - [ ] Quote mark style (single vs. double, smart vs. straight)
- [ ] Formatting habits
  - [ ] List usage (bulleted, numbered, frequency)
  - [ ] Heading style and depth
  - [ ] Emphasis patterns (bold, italic, underline usage)
  - [ ] Whitespace habits (double spacing, paragraph indentation)
- [ ] Paragraph structure patterns
  - [ ] Opening sentence length distribution
  - [ ] Closing sentence patterns
  - [ ] Paragraph length variance
  - [ ] Short paragraph frequency
- [ ] Rhythm analysis
  - [ ] Sentence length sequence patterns
  - [ ] Syllable stress patterns
  - [ ] Reading pace variation
- [ ] Function word distribution (the, of, and, to, etc.)
- [ ] Hedge word usage (maybe, perhaps, somewhat, etc.)
- [ ] Intensifier usage (very, extremely, absolutely, etc.)

---

## Phase 5: Writer Identity System (Layer 3)

### 5.1 Profile Construction
- [x] Profile data structure with serialization
- [x] Profile building from multiple analysis results
- [x] Profile save/load (JSON)
- [ ] Minimum sample requirements validation
  - [ ] Minimum word count per sample
  - [ ] Minimum number of samples
  - [ ] Genre diversity scoring
- [ ] Feature vector construction
  - [ ] Feature normalization (z-score, min-max)
  - [ ] Feature selection (most discriminative features)
  - [ ] Dimensionality reduction (PCA)
- [ ] Profile confidence intervals per feature
- [ ] Temporal profile updates (profile evolution over time)
- [ ] Profile versioning and history
- [ ] Multi-genre profile support (academic, casual, technical, etc.)

### 5.2 Comparison Engine
- [x] Feature-by-feature distance computation
- [x] Normalized distance metrics
- [ ] Statistical distance measures
  - [ ] Cosine similarity
  - [ ] Manhattan distance
  - [ ] Burrows' Delta
  - [ ] Cosine Delta (Wurzburg variant)
  - [ ] Kullback-Leibler divergence
  - [ ] Chi-squared distance
  - [ ] Jensen-Shannon divergence
- [ ] Feature weighting system
  - [ ] Information gain weighting
  - [ ] Feature reliability weighting (based on sample variance)
  - [ ] Domain-specific weight profiles
- [ ] Multi-candidate comparison
  - [ ] Rank multiple authors by likelihood
  - [ ] Pairwise comparison matrices
  - [ ] Exclusion scoring (definitely not this author)
- [ ] Cross-genre comparison adjustments
- [ ] Text length normalization

### 5.3 Confidence Scoring
- [x] Basic confidence scoring with word count adjustment
- [x] Confidence level classification (High/Medium/Low/Insufficient)
- [ ] Bayesian confidence model
  - [ ] Prior probability incorporation
  - [ ] Likelihood ratio computation
  - [ ] Posterior probability estimation
- [ ] Calibration testing
  - [ ] Calibration curve generation
  - [ ] Brier score computation
  - [ ] Expected calibration error (ECE)
- [ ] Confidence interval estimation (not just point estimates)
- [ ] Feature contribution to confidence (which features drove the score)
- [ ] Text-length adjusted confidence curves
- [ ] Genre-adjusted confidence

### 5.4 Anomaly Detection
- [x] Basic deviation detection for key metrics
- [ ] Section-by-section analysis
  - [ ] Sliding window analysis over document
  - [ ] Section boundary detection
  - [ ] Per-section stylometric comparison
- [ ] Multi-author document detection
  - [ ] Style change point detection
  - [ ] Segment attribution (which sections match which author)
  - [ ] Transition zone identification
- [ ] Inserted content detection
  - [ ] Copied/pasted passage identification
  - [ ] Ghostwritten section identification
  - [ ] AI-generated section indicators
- [ ] Temporal style drift detection (has the author's style changed?)
- [ ] Statistical significance testing for anomalies

---

## Phase 6: Machine Learning Engineering

### 6.1 Data Collection & Curation
- [ ] Training data pipeline
  - [ ] Author corpus collection framework
  - [ ] Data cleaning and preprocessing pipeline
  - [ ] Deduplication (exact and near-duplicate)
  - [ ] Data quality scoring
- [ ] Dataset construction
  - [ ] Known-author corpora (Project Gutenberg, academic datasets)
  - [ ] PAN competition datasets integration
  - [ ] Multi-genre dataset assembly
  - [ ] Multi-language dataset assembly (future)
- [ ] Data labeling infrastructure
  - [ ] Author attribution labels
  - [ ] Genre labels
  - [ ] Time period labels
  - [ ] Writing context labels (academic, casual, professional)
- [ ] Data versioning (DVC or similar)
- [ ] Dataset statistics and bias analysis
  - [ ] Author representation balance
  - [ ] Genre distribution analysis
  - [ ] Text length distribution analysis
  - [ ] Temporal coverage analysis

### 6.2 Feature Engineering
- [ ] Feature extraction pipeline
  - [ ] Automated feature computation from raw text
  - [ ] Feature caching/memoization
  - [ ] Incremental feature updates
- [ ] Feature store
  - [ ] Persistent feature storage format
  - [ ] Feature versioning
  - [ ] Feature metadata (description, computation cost, reliability)
- [ ] Feature analysis
  - [ ] Feature correlation matrix
  - [ ] Feature importance ranking
  - [ ] Feature stability across document lengths
  - [ ] Feature discriminative power per author pair
- [ ] Feature set configurations
  - [ ] Minimal feature set (fast, baseline)
  - [ ] Standard feature set (balanced)
  - [ ] Comprehensive feature set (maximum accuracy)
  - [ ] Custom feature set support

### 6.3 Model Architecture
- [ ] Baseline models
  - [ ] K-Nearest Neighbors (KNN) with stylometric features
  - [ ] Support Vector Machine (SVM) with RBF kernel
  - [ ] Random Forest classifier
  - [ ] Logistic Regression (interpretable baseline)
- [ ] Advanced models
  - [ ] Gradient Boosted Trees (XGBoost / LightGBM via bindings)
  - [ ] Siamese network for author verification
  - [ ] Transformer-based embeddings (character-level or token-level)
  - [ ] Ensemble methods (combining multiple model outputs)
- [ ] Model configurations
  - [ ] Hyperparameter search framework
  - [ ] Cross-validation pipeline
  - [ ] Model selection criteria (accuracy vs. interpretability vs. speed)
- [ ] Verification vs. Attribution distinction
  - [ ] Closed-set attribution (which of these N authors wrote this?)
  - [ ] Open-set attribution (is the author in our known set?)
  - [ ] Verification (did author X write this? binary decision)

### 6.4 Training Pipeline
- [ ] Training loop implementation
  - [ ] Batch processing
  - [ ] Early stopping
  - [ ] Learning rate scheduling
  - [ ] Checkpointing
- [ ] Evaluation pipeline
  - [ ] Train/validation/test split strategy
  - [ ] K-fold cross-validation
  - [ ] Leave-one-out validation (for small author sets)
  - [ ] Stratified sampling
- [ ] Metrics computation
  - [ ] Accuracy, precision, recall, F1-score
  - [ ] ROC-AUC for verification tasks
  - [ ] Equal Error Rate (EER)
  - [ ] Confusion matrices
  - [ ] Per-author performance breakdown
- [ ] Experiment tracking
  - [ ] Run logging (hyperparameters, metrics, artifacts)
  - [ ] Experiment comparison dashboard
  - [ ] Reproducibility guarantees (seed management, deterministic ops)

### 6.5 Model Deployment
- [ ] Model serialization format
  - [ ] ONNX export support
  - [ ] Custom binary format for embedded models
  - [ ] Model compression (quantization, pruning)
- [ ] Inference pipeline
  - [ ] Batch inference
  - [ ] Single-document inference
  - [ ] Streaming inference (for very large documents)
- [ ] Model versioning and registry
- [ ] A/B testing framework for model comparison
- [ ] Model performance monitoring (drift detection)

---

## Phase 7: ML Research & Experimentation

### 7.1 Stylometric Research
- [ ] Evaluate feature sets from published research
  - [ ] Mosteller & Wallace (Federalist Papers features)
  - [ ] Burrows' Delta and variants
  - [ ] Koppel's method (feature set randomization)
  - [ ] Stamatatos' character n-gram approach
  - [ ] Kestemont's compression-based methods
- [ ] Novel feature exploration
  - [ ] Edit distance patterns (revision style)
  - [ ] Error pattern profiling (typos, grammatical errors)
  - [ ] Metaphor and analogy usage patterns
  - [ ] Hedging and certainty expression patterns
- [ ] Cross-domain transferability studies
  - [ ] Same author, different genres
  - [ ] Same author, different time periods
  - [ ] Same author, different audiences
- [ ] Adversarial robustness
  - [ ] Style obfuscation resistance
  - [ ] Paraphrasing attack resilience
  - [ ] AI-assisted disguise detection
  - [ ] Translation attack (writing in another language, then translating)

### 7.2 Deep Learning Research
- [ ] Embedding models
  - [ ] Character-level CNN/RNN for author embeddings
  - [ ] Sentence-level embeddings for style
  - [ ] Document-level style embeddings
- [ ] Contrastive learning approaches
  - [ ] Triplet loss for author verification
  - [ ] Contrastive loss for style similarity
  - [ ] Self-supervised pretraining on unlabeled text
- [ ] Attention-based models
  - [ ] Attention over stylometric features
  - [ ] Multi-head attention for different style aspects
  - [ ] Interpretable attention (which text segments matter most)
- [ ] Few-shot / zero-shot experiments
  - [ ] Meta-learning for new authors with few samples
  - [ ] Prototypical networks for author clustering
  - [ ] Transfer learning from large language models

### 7.3 Benchmark & Evaluation Research
- [ ] Establish internal benchmarks
  - [ ] Baseline performance on standard datasets
  - [ ] Comparison with existing tools (JGAAP, Stylo, etc.)
  - [ ] Human expert comparison
- [ ] Robustness benchmarks
  - [ ] Short text performance (tweets, messages, paragraphs)
  - [ ] Cross-genre performance
  - [ ] Adversarial text performance
  - [ ] Multilingual performance
- [ ] Scalability benchmarks
  - [ ] Large author pool (100+, 1000+ authors)
  - [ ] Large document processing
  - [ ] Real-time processing requirements

---

## Phase 8: Scoring & Reporting

### 8.1 Score Aggregation
- [x] Basic unified scoring from all layers
- [ ] Weighted score aggregation
  - [ ] Layer-specific weight configuration
  - [ ] Dynamic weighting based on available evidence
  - [ ] Feature importance integration
- [ ] Multi-dimensional scoring
  - [ ] Authorship probability score
  - [ ] Document integrity score
  - [ ] Analysis confidence score
  - [ ] Risk/uncertainty score
- [ ] Score calibration
  - [ ] Probability calibration (Platt scaling, isotonic regression)
  - [ ] Score threshold optimization
  - [ ] False positive / false negative trade-off analysis

### 8.2 Report Generation
- [x] Basic text report rendering
- [ ] Detailed report format
  - [ ] Executive summary section
  - [ ] Forensic findings section
  - [ ] Text analysis section with visualizations
  - [ ] Authorship conclusion with confidence intervals
  - [ ] Methodology description
  - [ ] Limitations and caveats
- [ ] Report output formats
  - [ ] Plain text (current)
  - [ ] JSON (machine-readable)
  - [ ] HTML (rich formatting, charts)
  - [ ] PDF (formal report)
  - [ ] Markdown
- [ ] Visualization support
  - [ ] Feature comparison charts
  - [ ] Confidence distribution plots
  - [ ] Timeline visualizations
  - [ ] Anomaly heatmaps
- [ ] Audit trail in reports
  - [ ] Complete analysis parameters
  - [ ] Software version and configuration
  - [ ] Reproducibility information

---

## Phase 9: API & Integration

### 9.1 Library API
- [x] Public API surface (`lib.rs`)
- [ ] Stable API design
  - [ ] Builder pattern for analysis configuration
  - [ ] Async API for long-running operations
  - [ ] Progress callback support
  - [ ] Cancellation support
- [ ] API documentation with examples
- [ ] Versioning strategy (semver)

### 9.2 REST API (Future)
- [ ] HTTP server implementation (axum or actix-web)
- [ ] API endpoint design
  - [ ] POST /analyze — submit document for analysis
  - [ ] POST /profile — create author profile
  - [ ] GET /profile/:id — retrieve author profile
  - [ ] POST /compare — compare document against profile
  - [ ] GET /report/:id — retrieve analysis report
- [ ] Authentication and authorization
- [ ] Rate limiting
- [ ] API documentation (OpenAPI/Swagger)

### 9.3 Plugin/Extension System (Future)
- [ ] Plugin trait definition
- [ ] Custom feature extractor plugins
- [ ] Custom comparison algorithm plugins
- [ ] Custom report format plugins
- [ ] Plugin discovery and loading

---

## Phase 10: Security & Hardening

### 10.1 Input Validation
- [ ] File size limits
- [ ] File type allowlisting
- [ ] Path traversal prevention
- [ ] Zip bomb detection
- [ ] XML bomb detection (billion laughs)
- [ ] Malformed file handling (fuzzing)

### 10.2 Cryptographic Security
- [x] SHA-256 hashing
- [x] Hash verification utility
- [ ] Profile integrity signing
- [ ] Report signing (non-repudiation)
- [ ] Secure key management
- [ ] Constant-time comparison for sensitive data

### 10.3 Privacy & Data Protection
- [ ] PII detection and redaction in reports
- [ ] Secure profile storage (encryption at rest)
- [ ] Audit logging for all data access
- [ ] Data retention policies
- [ ] Right-to-deletion support

### 10.4 Testing & Assurance
- [ ] Fuzz testing (cargo-fuzz)
  - [ ] Fuzz file parsing
  - [ ] Fuzz text analysis
  - [ ] Fuzz profile deserialization
- [ ] Property-based testing (proptest)
- [ ] Dependency vulnerability scanning
- [ ] SAST (Static Application Security Testing)
- [ ] Penetration testing plan

---

## Phase 11: Performance & Optimization

### 11.1 Benchmarking
- [x] Criterion benchmark harness
- [ ] Benchmark suite
  - [ ] File parsing benchmarks (per format)
  - [ ] Text analysis benchmarks (per module)
  - [ ] Profile comparison benchmarks
  - [ ] End-to-end analysis benchmarks
  - [ ] Memory usage benchmarks
- [ ] Performance regression detection in CI

### 11.2 Optimization
- [ ] Parallelized analysis (rayon)
  - [ ] Parallel feature extraction
  - [ ] Parallel multi-file processing
  - [ ] Parallel profile comparison (multi-candidate)
- [ ] Memory optimization
  - [ ] Streaming text processing (avoid loading full documents)
  - [ ] Arena allocation for temporary analysis data
  - [ ] Memory-mapped file I/O for large documents
- [ ] Caching
  - [ ] Analysis result caching
  - [ ] Feature computation caching
  - [ ] Profile comparison caching
- [ ] SIMD optimizations for numerical computations (where applicable)

---

## Phase 12: Testing Strategy

### 12.1 Unit Tests
- [ ] forensics module tests
  - [ ] metadata extraction tests (per file type)
  - [ ] integrity check tests
  - [ ] format detection tests
  - [ ] timeline construction tests
- [ ] analysis module tests
  - [ ] lexical analysis edge cases
  - [ ] syntactic analysis edge cases
  - [ ] semantic analysis tests
  - [ ] stylometric analysis tests
- [ ] identity module tests
  - [ ] profile build/save/load roundtrip
  - [ ] comparison accuracy tests
  - [ ] confidence scoring tests
  - [ ] anomaly detection tests
- [ ] crypto module tests
  - [x] SHA-256 known-value tests
  - [ ] Hash verification tests
- [ ] scoring module tests
  - [ ] Score aggregation tests
  - [ ] Report rendering tests

### 12.2 Integration Tests
- [x] Basic lexical analysis integration test
- [x] Crypto hash integration test
- [ ] Full pipeline test (file -> forensics -> analysis -> score -> report)
- [ ] Profile workflow test (samples -> profile -> compare -> result)
- [ ] CLI integration tests (clap argument parsing, subcommands)
- [ ] Multi-format document tests

### 12.3 Test Data & Fixtures
- [ ] Sample text files (various genres, lengths)
- [ ] Sample DOCX files
- [ ] Sample PDF files
- [ ] Known-author text pairs for verification testing
- [ ] Edge case documents (empty, very large, corrupted, adversarial)
- [ ] Golden output files for regression testing

---

*This document tracks all development tasks for Provenance. Check items as completed. Review and update regularly.*
