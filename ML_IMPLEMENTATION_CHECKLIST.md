# ML PIPELINE IMPLEMENTATION CHECKLIST

**For use during development:** Check off each item as completed.  
**Validation:** Each phase has explicit go/no-go criteria.

---

## PHASE 1: DATA & FEATURES (Weeks 1-8)

### Week 1-2: Training Data Collection Pipeline

#### Data Collection Infrastructure
- [ ] Establish connections to ArXiv API (academic papers)
- [ ] Establish connections to PubMed API (medical papers)
- [ ] Establish connections to Reuters/AP news feeds (licensed or public)
- [ ] Establish connections to Project Gutenberg (creative writing)
- [ ] Establish connections to GitHub API (technical documentation)
- [ ] Establish SEC Edgar API integration (business documents)
- [ ] Establish Reddit API access (social media)
- [ ] Establish Twitter API access (social media)

#### AI Text Generation
- [ ] Set up OpenAI API account with GPT-4 quota
- [ ] Set up Anthropic API account (Claude)
- [ ] Set up Google Gemini API account
- [ ] Set up Llama access (via Together.ai or Replicate)
- [ ] Set up Mistral API account
- [ ] Create prompt templates for each LLM (zero-shot, few-shot, persona, anti-detection, CoT)
- [ ] Create topic generation: 500+ topic prompts across registers

#### Humanizer Integration
- [ ] QuillBot account setup with batch processing
- [ ] Undetectable.ai account setup
- [ ] WordAI account setup
- [ ] Manual editing infrastructure (Google Forms, Mechanical Turk, or contractor network)

#### Data Ingestion Pipeline
- [ ] Build `collect_training_data.py` script
- [ ] Implement retry logic for API failures
- [ ] Implement rate limiting (respect API quotas)
- [ ] Implement deduplication (hash-based content matching)
- [ ] Implement basic quality checks (length, language, encoding)
- [ ] Build monitoring for collection pipeline

**Go/No-Go Criteria (end of Week 2):**
- Collected ≥1,000 samples per source
- 0 failures in deduplication (all hashes unique)
- Pipeline latency <1s per document
- Storage: <100GB for raw data

---

### Week 3-4: Data Quality Validation & Labeling

#### Topic Annotation
- [ ] Train DistilBERT classifier on 5,000 labeled samples
- [ ] Apply classifier to full dataset
- [ ] Manual QA on 500 random samples (assess classifier accuracy)
- [ ] Reject if <85% accuracy; retrain if needed

#### Register Annotation  
- [ ] Create Likert scale annotation tool (1=casual, 5=legal)
- [ ] Hire annotators (or use contractor pool) for 10,000 samples
- [ ] Compute inter-rater agreement (Krippendorff's α)
- [ ] Reject if <0.75 agreement; clarify guidelines if needed
- [ ] Apply register labels to full dataset using trained classifier

#### Temporal Metadata
- [ ] Extract publication date from all sources (or estimate from content)
- [ ] Verify >95% of human text is from 2020+
- [ ] Record AI text generation timestamp
- [ ] Organize data into quarterly buckets for temporal splits

#### Content Quality Checks
- [ ] Filter out documents <200 words (too short)
- [ ] Filter out documents >50,000 words (too long, computational overhead)
- [ ] Language detection (keep English only)
- [ ] Remove duplicates using MinHash similarity (threshold: >95%)
- [ ] Manual spot-check: 100 random samples per source (read & verify quality)

**Go/No-Go Criteria (end of Week 4):**
- Topic classifier accuracy ≥85%
- Register annotation inter-rater agreement ≥0.75
- Processed dataset: 50K+ human, 75K+ AI (MVP targets)
- Zero duplicates detected in final dataset
- 0 errors in temporal metadata for >99% of samples

---

### Week 5-6: Feature Extraction Pipeline Implementation

#### Core Feature Extractors
- [ ] Implement lexical diversity features (TTR, MTLD, HDD, VOCD)
- [ ] Implement syntactic features (POS trigrams, parse depth)
- [ ] Implement semantic features (coherence, topic entropy)
- [ ] Implement burstiness features (burstiness coefficient, sentence rhythm)
- [ ] Implement function word features (frequency counts)
- [ ] Implement all existing Provenance features (from `src/analysis/`)

#### Feature Extraction Pipeline
- [ ] Create `features/extractors.py` module
- [ ] Implement batching for efficiency (process 100 docs at a time)
- [ ] Implement caching layer (store extracted features in SQLite or HDF5)
  - HDF5 recommended: `h5py` library, ~10 bytes per feature per doc
- [ ] Implement progress tracking with `tqdm`
- [ ] Implement error handling and logging

#### Feature Quality Checks
- [ ] Run feature extraction on 1,000 test documents
- [ ] Verify no NaN/Inf values (handle edge cases)
- [ ] Spot-check distributions match linguistic expectations
- [ ] Benchmark: measure extraction time per document
  - Target: <0.3s per document (5,000 docs = 25 min on single core)

**Go/No-Go Criteria (end of Week 6):**
- Feature extraction pipeline completes 5,000 docs without error
- Zero NaN/Inf values in feature matrices
- Feature extraction latency <0.3s/doc (single core)
- All feature distributions reasonable (no extreme outliers)
- Caching system reduces recomputation time by >90%

---

### Week 7-8: Feature Selection & Validation

#### Correlation Filtering
- [ ] Compute feature correlation matrix (75+ initial features)
- [ ] Identify pairs with r > 0.95 correlation
- [ ] Keep higher-discriminative feature from each pair (measure by Cohen's d)
- [ ] Document decisions in `features/correlation_analysis.txt`

#### Mutual Information Filtering
- [ ] Compute MI between each feature and target (human/AI label)
- [ ] Rank features by MI
- [ ] Keep top 200 features
- [ ] Visualize MI distribution (histograms)

#### LASSO Regularization
- [ ] Split data into 5-fold CV sets
- [ ] For each fold:
  - [ ] Fit logistic regression with LASSO (alpha tuned via `LassoCV`)
  - [ ] Record non-zero feature coefficients
  - [ ] Record alpha value chosen
- [ ] Select features that appear in ≥4/5 folds
- [ ] Result: ~80-100 candidate features

#### Recursive Feature Elimination (RFE)
- [ ] Use `sklearn.feature_selection.RFE` with SVM estimator
- [ ] Target: eliminate 10% of features per iteration
- [ ] Record number of iterations needed
- [ ] Final result: ~50-75 features

#### Stability Selection
- [ ] Run feature selection on 100 bootstrap samples (80% of data)
- [ ] Count how many times each feature selected
- [ ] Keep features selected >80 times (80% of bootstrap samples)
- [ ] Final feature set: 50-75 features

#### Feature Documentation
- [ ] Create `features/final_feature_list.json`:
  ```json
  {
    "features": [
      {"name": "ttr", "category": "lexical_diversity", "cohen_d": 0.45},
      ...
    ],
    "count": 68,
    "selection_date": "2024-04-30"
  }
  ```
- [ ] Document why each feature was selected
- [ ] Plan register-specific feature additions (10-35 per register)

**Go/No-Go Criteria (end of Week 8):**
- Final feature count: 50-75 features
- Feature set reproducible (same features on random shuffles of data)
- Each feature has Cohen's d > 0.3 (meaningful effect size)
- Feature documentation complete
- Feature extraction time with final set: <0.5s per document

---

## PHASE 2: MODEL DEVELOPMENT (Weeks 9-16)

### Week 9-10: Model Training Pipeline

#### XGBoost Setup
- [ ] Implement XGBoost training with exact hyperparams from guide:
  ```python
  {
      'max_depth': 6,
      'learning_rate': 0.1,
      'n_estimators': 500,
      'subsample': 0.8,
      'colsample_bytree': 0.8,
      'reg_alpha': 0.1,
      'reg_lambda': 1.0,
      'scale_pos_weight': 1.25,
      'objective': 'binary:logistic',
      'eval_metric': 'auc'
  }
  ```
- [ ] Implement early stopping (monitor validation AUC, patience=20)
- [ ] Implement stratified train/val split (80/20)
- [ ] Implement feature normalization (Z-score, fitted on training set only)
- [ ] Save trained model to disk (`models/xgboost_model`)

#### Neural Network Setup
- [ ] Implement 3-layer MLP with exact architecture from guide:
  - Input: 68-75 features
  - Hidden layers: 128 → 64 → 32 neurons
  - Dropout: 0.3
  - Batch normalization: enabled
  - Output: 1 sigmoid neuron (binary classification)
- [ ] Implement training loop with:
  - Optimizer: Adam (lr=0.001)
  - Loss: Binary cross-entropy
  - Early stopping: monitor validation loss, patience=15
  - Batch size: 32
- [ ] Implement learning rate scheduling (reduce on plateau)
- [ ] Save trained model to disk (`models/neural_net_model`)

#### SVM Setup
- [ ] Implement SVM with RBF kernel
- [ ] Tune C hyperparameter via cross-validation (grid: [0.1, 1, 10])
- [ ] Train on full training set
- [ ] Save trained model to disk (`models/svm_model`)

#### Training Validation
- [ ] Run 5-fold cross-validation on training data
- [ ] Report mean ± std for: AUC, accuracy, precision, recall
- [ ] Check for signs of overfitting (validation < train by >5%)
- [ ] Save CV results to `models/cv_results.json`

**Go/No-Go Criteria (end of Week 10):**
- All 3 models trained without errors
- XGBoost training AUC > 0.96
- Neural Net training loss converging smoothly
- SVM training AUC > 0.94
- No overfitting detected (validation AUC within 3% of training)

---

### Week 11-12: Calibration & Threshold Optimization

#### Temperature Scaling
- [ ] Split validation set 50/50 into calibration sets
- [ ] Fit TemperatureScaling on first half
- [ ] Compute calibration temperature value
- [ ] Save temperature to `models/calibrator.pkl`

#### Isotonic Regression
- [ ] Fit IsotonicRegression on second calibration set
- [ ] Ensure `out_of_bounds='clip'` to handle extrapolation
- [ ] Test calibration on holdout test set
- [ ] Save isotonic regressor to `models/calibrator.pkl`

#### Calibration Quality
- [ ] Compute ECE on test set after both calibration stages
- [ ] Target: ECE < 0.05
- [ ] If ECE > 0.05, try Platt scaling as alternative
- [ ] Generate calibration curves (reliability diagram)
- [ ] Document calibration process in `evaluation/calibration_analysis.pkl`

#### Threshold Optimization
- [ ] Compute ROC curve on validation set
- [ ] Find exact threshold where FPR = 2%
- [ ] Record corresponding TPR (target: ≥95%)
- [ ] If TPR < 95%, investigate why (feature quality? model architecture?)
- [ ] Compute thresholds per register if fairness analysis shows >5% FPR variance
- [ ] Save thresholds to `models/thresholds.json`

#### Threshold Validation
- [ ] Apply found threshold to test set
- [ ] Verify TPR ≥ 95% and FPR ≈ 2%
- [ ] Compute confidence intervals (bootstrap 1,000 times)
- [ ] Document in `evaluation/threshold_analysis.json`

**Go/No-Go Criteria (end of Week 12):**
- ECE < 0.05 after calibration
- TPR@2%FPR ≥ 95% on validation set
- Threshold optimization complete & documented
- Calibration consistent across different threshold values

---

### Week 13-14: Ensemble Development & Validation

#### Ensemble Assembly
- [ ] Load all 3 trained models (XGBoost, NN, SVM)
- [ ] Load calibrator
- [ ] Implement ensemble prediction:
  ```python
  def ensemble_predict(text):
      features = extract_features(text)
      xgb_pred = xgb_model.predict_proba(features)[1]
      nn_pred = nn_model.predict(features)[0]
      svm_pred = svm_model.predict_proba(features)[1]
      
      # Weighted average
      ensemble_score = 0.7 * xgb_pred + 0.2 * nn_pred + 0.1 * svm_pred
      
      # Calibrate
      return calibrator.calibrate(ensemble_score)
  ```
- [ ] Test ensemble on validation set

#### Ensemble Validation
- [ ] Measure ensemble AUC, precision, recall
- [ ] Compare to individual models (should improve)
- [ ] Measure prediction variance across ensemble members (indicator of uncertainty)
- [ ] Compute weighted feature importance across ensemble
- [ ] Document ensemble design in `models/ensemble_design.md`

#### Feature Importance Analysis
- [ ] Extract feature importance from XGBoost
- [ ] Compute permutation feature importance on test set
- [ ] Rank features by importance
- [ ] Document in `evaluation/feature_importance.json`
- [ ] Identify redundant features (may eliminate in next iteration)

#### Ablation Study
- [ ] Remove each feature group (lexical, syntactic, semantic, burstiness, function_words)
- [ ] Measure performance drop for each removal
- [ ] Document in `evaluation/ablation_study.json`
- [ ] Identify critical vs. nice-to-have features

**Go/No-Go Criteria (end of Week 14):**
- Ensemble AUC > individual models
- Ensemble TPR@2%FPR ≥ 95%
- Feature importance analysis complete
- Ablation study shows no single feature group is critical (diversity good)

---

### Week 15-16: Comprehensive Evaluation & Ablation

#### Test Set Evaluation
- [ ] Apply ensemble to full test set (50K samples)
- [ ] Compute all primary metrics: TPR@2%FPR, ECE, AUC, F1
- [ ] Record results in `evaluation/test_set_results.json`

#### Specialized Test Sets
- [ ] Apply ensemble to adversarial test set (humanizer tools)
  - Target: TPR ≥ 80%
- [ ] Apply ensemble to academic test set
  - Target: FPR < 3%
- [ ] Apply ensemble to social media test set
  - Target: FPR < 3%
- [ ] Apply ensemble to long-form test set (>3000 words)
  - Target: Same as main test set
- [ ] Apply ensemble to emerging model test set
  - Target: TPR ≥ 90%
- [ ] Document in `evaluation/specialized_test_results.json`

#### Fairness Analysis
- [ ] Compute FPR by register (academic, social, business, etc)
- [ ] Compute FPR by text length bucket
- [ ] Compute FPR by non-native speaker samples (if available)
- [ ] Report max FPR variance across groups
- [ ] If variance > thresholds, implement per-group calibration

#### Final Validation Report
- [ ] Create `evaluation/final_validation_report.md`
- [ ] Include:
  - Test set metrics (all primary + secondary)
  - Fairness analysis
  - Feature importance
  - Calibration analysis
  - Comparison to baselines (if available)
  - Recommendations for production deployment

**Go/No-Go Criteria (end of Week 16):**
- TPR@2%FPR ≥ 95% on main test set ✓ (GO/NO-GO)
- ECE < 0.05 ✓ (GO/NO-GO)
- AUC > 0.98 ✓
- Adversarial TPR ≥ 80% ✓
- FPR fairness: max variance <3x ✓
- Final validation report complete ✓

**If any GO/NO-GO criterion fails: STOP. Do NOT proceed to Phase 3. Retrain with different approach.**

---

## PHASE 3: PRODUCTION ENGINEERING (Weeks 17-22)

### Week 17-18: ONNX Export & Rust Inference

#### ONNX Export
- [ ] Export XGBoost to ONNX format
  - Library: `skl2onnx`
  - Input shape: (N, 68-75 features)
  - Output: binary classification probabilities
  - Save to `models/provenance_xgboost.onnx`
- [ ] Export Neural Network to ONNX
  - Use PyTorch's native ONNX export or `torch.onnx.export`
  - Save to `models/provenance_nn.onnx`
- [ ] Validate ONNX models (check inputs/outputs match Python versions)
- [ ] Test inference equivalence (Python vs ONNX predictions within 1e-5)

#### Rust Inference Pipeline
- [ ] Add `ort` crate to Provenance Cargo.toml (`ort = "2.0"`)
- [ ] Implement ProvenanceModel struct in `src/ml/inference.rs`:
  ```rust
  pub struct ProvenanceModel {
      session: ort::Session,
      feature_extractor: FeatureExtractor,
      calibrator: Calibrator,
      scaler: ZScoreNormalizer,
  }
  ```
- [ ] Implement `predict()` method
- [ ] Test inference on 1,000 sample documents
- [ ] Benchmark: measure inference latency
  - Feature extraction: <1.5s for 5,000 words
  - Model inference: <50ms
  - Total: <2s

#### Model Versioning
- [ ] Create ModelVersion struct with metadata:
  - version string (e.g., "v2024.04.30")
  - feature names (ordered list)
  - model hash (SHA256 of ONNX)
  - calibration parameters
  - performance metrics
  - compatibility info
- [ ] Implement version loading/validation
- [ ] Store version info in `models/model_version.json`

**Go/No-Go Criteria (end of Week 18):**
- ONNX inference predictions match Python within 1e-5
- Inference latency <2s for 5,000 words
- Model versioning system working
- No crashes on edge cases (empty text, very long text, special characters)

---

### Week 19-20: Monitoring & Alerting Infrastructure

#### Monitoring Metrics Setup
- [ ] Implement prediction score distribution tracking
  - Hourly histograms, percentiles (p5, p25, p50, p75, p95)
  - Tier distribution (% in HUMAN / LIKELY / MIXED / AI_ASSISTED / AI_GENERATED)
- [ ] Implement feature drift monitoring
  - Daily rolling mean/std for all 70 features
  - Weekly correlation matrix
  - PSI (Population Stability Index) computation
- [ ] Implement performance monitoring
  - Inference latency (p50, p95, p99)
  - Throughput (requests/sec)
  - Error rate tracking
  - Timeout rate tracking

#### Drift Detection
- [ ] Implement drift alert thresholds:
  - FPR > 2.5%: Alert (target is 2%, allow 0.5% buffer)
  - PSI > 0.1: Alert on >20% of features
  - KS statistic > 0.15: Alert
  - ECE > 0.05 weekly: Alert
- [ ] Set up alert destinations (email, Slack, dashboard)

#### Validation Harness
- [ ] Build `ValidationMonitor` struct (daily validation against known samples)
  - 5,000 known-human documents (validation set)
  - 5,000 known-AI documents (validation set)
  - Daily scoring
  - FPR/TPR computation
  - Alert if FPR > 2.5% or TPR < 92%
- [ ] Automate daily validation runs
- [ ] Log results to database

#### Monitoring Dashboard
- [ ] Set up Grafana or similar visualization tool
- [ ] Create dashboards for:
  - Prediction distribution (over time)
  - Feature drift indicators
  - Inference latency
  - FPR/TPR trends
- [ ] Make dashboards accessible to team

**Go/No-Go Criteria (end of Week 20):**
- All monitoring metrics collecting data successfully
- Drift detection working (test with synthetic drift)
- Daily validation harness running automatically
- Team has access to monitoring dashboard
- Alert system tested and functional

---

### Week 21: Performance Optimization & Stress Testing

#### Latency Optimization
- [ ] Profile inference pipeline (which step is slowest?)
- [ ] Optimize feature extraction (vectorize if possible)
- [ ] Optimize ONNX model loading (load once, reuse)
- [ ] Test on various hardware (CPU, GPU if available)
- [ ] Benchmark throughput at scale (100, 1000, 10000 documents)

#### Memory Profiling
- [ ] Measure peak memory usage per inference
- [ ] Ensure <500MB per worker
- [ ] Test with long documents (10,000+ words)
- [ ] Verify no memory leaks in long-running process

#### Stress Testing
- [ ] Test inference with edge cases:
  - Very short text (<50 words)
  - Very long text (>20,000 words)
  - Special characters, non-ASCII
  - Multiple languages (should handle gracefully)
  - Extreme feature values (trigger clipping if needed)
- [ ] Load testing: concurrent requests (100, 1000, 10000)
- [ ] Measure response time distribution under load

#### Documentation
- [ ] Write `DEPLOYMENT.md` with:
  - System requirements
  - Installation instructions
  - Configuration guide
  - Performance characteristics
  - Troubleshooting guide

**Go/No-Go Criteria (end of Week 21):**
- Latency meets budget (<2s for 5,000 words)
- Memory usage <500MB per worker
- Stress testing: system handles 100 concurrent requests
- All edge cases handled gracefully
- Deployment documentation complete

---

### Week 22: Security Review & Adversarial Testing

#### Security Review
- [ ] Review model for adversarial robustness
- [ ] Check for input validation (prevent injection attacks)
- [ ] Verify no sensitive data in model or features
- [ ] Test sandboxing (if running untrusted code)
- [ ] Document security assumptions

#### Adversarial Testing
- [ ] Test on humanizer-processed samples
  - QuillBot: target TPR ≥ 75%
  - Undetectable.ai: target TPR ≥ 75%
  - WordAI: target TPR ≥ 75%
  - Manual paraphrasing: target TPR ≥ 80%
- [ ] Identify any tool-specific blind spots
- [ ] If blind spot exists: add tool-specific feature or retrain ensemble

#### Red Team Exercise
- [ ] Conduct red team session (2-3 people)
- [ ] Try to fool the model with various strategies:
  - Synonym substitution
  - Sentence reordering
  - Style transfer
  - Prompt engineering
- [ ] Document all successful attacks
- [ ] Develop countermeasures for high-success attacks

#### Robustness Validation
- [ ] Create `RobustnessValidator` for feature stability
  - Test features under synonym substitution
  - Test features under paraphrasing
  - Test features under reordering
  - Compute stability correlation per feature
  - Flag features with low stability (<0.7)
- [ ] Document in `evaluation/robustness_analysis.json`

**Go/No-Go Criteria (end of Week 22):**
- No critical security vulnerabilities found
- Humanizer detection TPR ≥ 75% per tool
- Robustness analysis complete
- Red team findings documented & addressed
- Ready for limited deployment

---

## PHASE 4: DEPLOYMENT (Weeks 23-26)

### Week 23: Staged Rollout to Internal Systems

- [ ] Deploy to internal testing system (100% of internal documents)
- [ ] Monitor for 7 days
- [ ] Verify no alerts triggered
- [ ] Collect feedback from internal team
- [ ] Adjust thresholds if needed (should not change)

### Week 24: A/B Testing Framework

- [ ] Implement A/B testing infrastructure
- [ ] Route 10% of traffic to new model (weeks 24-25)
- [ ] Collect metrics: FPR, TPR, user feedback
- [ ] Route 50% of traffic to new model (week 25)
- [ ] Verify metrics stable, user feedback positive

### Week 25: Customer Pilot Program

- [ ] Select 5-10 friendly customers
- [ ] Deploy to pilot group (100% of their documents)
- [ ] Monitor daily
- [ ] Collect feedback
- [ ] Address any issues

### Week 26: Full Production Deployment

- [ ] Roll out to all customers
- [ ] Monitor closely (daily validation)
- [ ] Be prepared to rollback if FPR > 2.5%

---

## SUCCESS CRITERIA SUMMARY

### Pre-Deployment (Must pass all before Week 17)
- [ ] TPR@2%FPR ≥ 95% ← GO/NO-GO GATE 1
- [ ] ECE < 0.05 ← GO/NO-GO GATE 2
- [ ] Inference latency <2 seconds for 5K words
- [ ] Adversarial TPR ≥ 80%
- [ ] Feature coverage >99% (minimal NaN)
- [ ] Reproducible results (same random seed)

### Post-Deployment (First 4 weeks)
- [ ] FPR ≤ 2.5% (alert if higher)
- [ ] No customer-reported false positives
- [ ] Inference latency p99 < 3 seconds
- [ ] Feature drift (PSI) < 0.1 on >80% of features
- [ ] Score distribution stable (KS < 0.15)

### Ongoing (Monthly checks)
- [ ] TPR ≥ 93% (allow 2% degradation)
- [ ] ECE < 0.07 (allow 0.02 degradation)
- [ ] No unplanned system downtime
- [ ] Feature drift monitoring active

---

**Created:** 2026-03-24  
**Last updated:** 2026-03-24  
**Status:** Ready for Phase 1 start

