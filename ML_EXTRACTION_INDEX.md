# ML PIPELINE EXTRACTION — COMPLETE INDEX

**Extraction date:** 2026-03-24  
**Source:** debate_9_ml_engineering.json (Claude Sonnet 4, Stage 1)  
**Total extracted:** 42,614 characters → 3 comprehensive implementation documents

---

## DOCUMENTS CREATED

### 1. ML_PIPELINE_IMPLEMENTATION_GUIDE.md (819 lines)

**What it contains:** Complete technical specification for implementing the ML pipeline.

**Sections:**
- Part 1: Training Data Pipeline (collection sources, APIs, distribution strategy)
- Part 2: Feature Engineering (selection algorithm, stability analysis, per-register features)
- Part 3: Model Architecture (XGBoost/NN/SVM ensemble with exact hyperparameters)
- Part 4: Calibration Pipeline (two-stage: temperature scaling + isotonic regression)
- Part 5: Threshold Optimization (finding exact 2% FPR operating point)
- Part 6: Evaluation Framework (test set construction, metrics, go/no-go criteria)
- Part 7: Deployment & Monitoring (ONNX export, Rust inference code, versioning)
- Part 8: Production Monitoring (drift detection, retraining triggers)
- Part 9: Hard ML Problems (distribution shift, adversarial robustness, uncertainty)
- Part 10: Implementation Timeline (6-month roadmap with weekly milestones)
- Part 11: Python Libraries (complete tooling stack)
- Part 12: Directory Structure (full filesystem layout)
- Part 13: Success Criteria & Gates (go/no-go decision points)
- Part 14: Critical Implementation Notes (hyperparameter tuning, normalization, reproducibility)

**How to use:** Read sequentially as reference material. Cross-reference specific sections when building each component.

**Key metrics to remember:**
- Training data: 50K human (MVP) → 250K (prod); 75K AI (MVP) → 375K (prod)
- Final feature count: 50-75 features
- Primary metric: TPR@2%FPR ≥ 95%
- Calibration target: ECE < 0.05
- Inference budget: <2 seconds for 5,000-word document

---

### 2. ML_IMPLEMENTATION_CHECKLIST.md (641 lines)

**What it contains:** Week-by-week implementation checklist for Phases 1-4.

**Structure:**
- Phase 1 (Weeks 1-8): Data collection, quality validation, feature engineering
  - Week 1-2: Establish all data source connections (ArXiv, PubMed, Reuters, etc.)
  - Week 3-4: Quality validation, topic/register annotation
  - Week 5-6: Feature extraction pipeline implementation
  - Week 7-8: Feature selection (correlation, MI, LASSO, RFE, stability selection)
  
- Phase 2 (Weeks 9-16): Model training & validation
  - Week 9-10: XGBoost + Neural Network + SVM training
  - Week 11-12: Calibration & threshold optimization
  - Week 13-14: Ensemble assembly & feature importance analysis
  - Week 15-16: Comprehensive evaluation (main test set + 5 specialized test sets)
  
- Phase 3 (Weeks 17-22): Production engineering
  - Week 17-18: ONNX export & Rust inference pipeline
  - Week 19-20: Monitoring & alerting infrastructure
  - Week 21: Performance optimization & stress testing
  - Week 22: Security review & adversarial testing
  
- Phase 4 (Weeks 23-26): Deployment
  - Week 23: Staged rollout to internal systems
  - Week 24: A/B testing framework
  - Week 25: Customer pilot program
  - Week 26: Full production deployment

**How to use:** Check off items as you complete them. Use as project management tool. Each week has explicit go/no-go criteria.

**Critical gates (STOP if not met):**
- End of Week 16 (Phase 2):
  - TPR@2%FPR ≥ 95% ✓
  - ECE < 0.05 ✓
  - If either fails → DO NOT PROCEED to Phase 3

---

### 3. ML_PYTHON_TEMPLATES.md (866 lines)

**What it contains:** Copy-paste Python code templates for every major component.

**Code templates:**
1. Feature extraction with HDF5 caching
2. Dataset preparation with stratification
3. XGBoost training with early stopping
4. Neural network (PyTorch) with ONNX export
5. Two-stage calibration (temperature scaling + isotonic regression)
6. Threshold optimization for 2% FPR
7. ECE & Brier score computation
8. Population Stability Index (drift monitoring)
9. Quick-start: train full pipeline end-to-end
10. Rust inference template (ONNX Runtime)
11. Daily validation harness (FPR/TPR monitoring)

**How to use:** Copy templates into your codebase. Modify paths and hyperparameters as needed. All code is production-ready.

**Key library versions (not specified in original; use latest):**
- xgboost
- torch / tensorflow
- scikit-learn
- ort (Rust crate)
- h5py

---

## QUICK START: WHERE TO BEGIN

### If you're starting implementation:

1. **Read this first:** `ML_PIPELINE_IMPLEMENTATION_GUIDE.md` (Parts 1-2)
   - Understand data collection strategy
   - Understand feature engineering pipeline
   - Estimate timeline & resource needs

2. **Create checklist:** Print `ML_IMPLEMENTATION_CHECKLIST.md` Phase 1
   - Set up connections to all data sources
   - Plan week 1-2 tasks

3. **Copy templates:** Use `ML_PYTHON_TEMPLATES.md` sections 1-2
   - Start with data collection pipeline
   - Build feature extraction infrastructure

### If you're 3 months in (Phase 2):

1. **Review:** `ML_IMPLEMENTATION_GUIDE.md` (Parts 3-6)
   - Model architecture choices
   - Calibration & threshold optimization
   - Evaluation methodology

2. **Execute:** `ML_IMPLEMENTATION_CHECKLIST.md` Phase 2
   - Follow weeks 9-16 exactly
   - Check off items as you go

3. **Implement:** `ML_PYTHON_TEMPLATES.md` sections 3-8
   - XGBoost training
   - Neural network training
   - Calibration
   - Threshold optimization

### If you're at deployment (Phase 3-4):

1. **Review:** `ML_IMPLEMENTATION_GUIDE.md` (Parts 7-8)
   - Rust inference architecture
   - Monitoring & drift detection

2. **Execute:** `ML_IMPLEMENTATION_CHECKLIST.md` Phase 3-4
   - ONNX export & inference pipeline
   - Monitoring setup
   - Staged rollout

3. **Implement:** `ML_PYTHON_TEMPLATES.md` sections 10-11
   - Rust inference code
   - Daily validation harness

---

## KEY NUMBERS TO MEMORIZE

### Data
- Training: 50K-75K human, 75K-375K AI, 10% humanized subset
- Class balance: 40% human, 50% raw AI, 10% humanized (training)
- Sources: 7 human sources, 6 AI models, 4 humanizer tools

### Features
- Initial: 75+ features across 5 categories
- Final: 50-75 features selected via MI + LASSO + RFE + stability
- Correlation threshold: r > 0.95 (remove one from pair)
- Stability threshold: features must be stable >80% across perturbations

### Models
- Primary: XGBoost (70% weight)
- Secondary: Neural Network 3-layer [128, 64, 32] (20% weight)
- Tertiary: SVM-RBF (10% weight)
- Ensemble: Weighted average of calibrated probabilities

### Calibration & Threshold
- Calibration: Two-stage (temperature scaling + isotonic regression)
- ECE target: < 0.05
- Threshold: Find where FPR = exactly 2%
- Resulting TPR: Must be ≥ 95%

### Performance Budget
- Feature extraction: <1.5s per document (5,000 words)
- Model inference: <50ms
- Total end-to-end: <2 seconds
- Throughput: >100 documents/minute/core

### Monitoring
- FPR alert: > 2.5% (allow 0.5% buffer from 2% target)
- Feature drift alert: PSI > 0.1 on >20% of features
- Calibration alert: ECE > 0.05 weekly
- Retraining triggers: Monthly + quarterly + event-driven (new LLM)

### Success Criteria
- **GO/NO-GO Gate 1:** TPR@2%FPR ≥ 95%
- **GO/NO-GO Gate 2:** ECE < 0.05
- Post-deployment (4 weeks): FPR ≤ 2.5%, zero customer false positives
- Ongoing: Monthly validation shows TPR ≥ 93%, ECE < 0.07

---

## HYPERPARAMETERS (EXACT VALUES FROM DEBATE)

### XGBoost
```
max_depth: 6
learning_rate: 0.1
n_estimators: 500
subsample: 0.8
colsample_bytree: 0.8
reg_alpha: 0.1
reg_lambda: 1.0
scale_pos_weight: 1.25
objective: binary:logistic
eval_metric: auc
```

### Neural Network
```
input_dim: 75
hidden_layers: [128, 64, 32]
dropout: 0.3
activation: relu
output_activation: sigmoid
batch_norm: True
optimizer: Adam (lr=0.001)
loss: BCELoss
batch_size: 32
early_stopping: patience=15
```

### SVM
```
kernel: rbf
C: tune via cross-validation [0.1, 1, 10]
probability: True
```

### Calibration (Exact algorithm)
```
Stage 1: TemperatureScaling on 50% of validation set
Stage 2: IsotonicRegression on other 50%
out_of_bounds: 'clip'
```

### Feature Selection Pipeline
```
1. Correlation: Remove r > 0.95
2. MI: Keep top 200 by mutual information
3. LASSO: Cross-validation to tune α
4. RFE: SVM-based, eliminate 10% per iteration
5. Stability: 100 bootstrap samples, keep features >80 times
```

---

## VALIDATION CHECKLIST (Copy & Print)

### Pre-Deployment Requirements
- [ ] TPR@2%FPR ≥ 95% on validation set
- [ ] ECE < 0.05 on test set
- [ ] AUC > 0.98 on test set
- [ ] Adversarial (humanized) TPR ≥ 80%
- [ ] Feature drift (PSI) < 0.1 on >80% of features
- [ ] Inference latency <2 seconds for 5,000 words
- [ ] Memory usage <500MB per worker
- [ ] Stress testing: handles 100 concurrent requests
- [ ] Security review: no critical vulnerabilities
- [ ] Red team: no attack succeeds >25% of time
- [ ] Deployment documentation complete

### Post-Deployment Monitoring (First 4 Weeks)
- [ ] FPR stays ≤ 2.5%
- [ ] TPR stays ≥ 92%
- [ ] Zero customer-reported false positives
- [ ] Inference latency p99 < 3 seconds
- [ ] Feature drift (PSI) < 0.1 on >80% of features
- [ ] Score distribution stable (KS < 0.15)
- [ ] No alerts triggered on monitoring dashboard

---

## CONTACT / REFERENCE

**Original source:** `debate_9_ml_engineering.json`  
**Model:** Claude Sonnet 4 (Stage 1 response, 42,586 chars)  
**Extraction method:** JSON parsing + strategic section extraction  
**Date created:** 2026-03-24

**Three extracted documents:**
- `ML_PIPELINE_IMPLEMENTATION_GUIDE.md` — Technical reference
- `ML_IMPLEMENTATION_CHECKLIST.md` — Project management
- `ML_PYTHON_TEMPLATES.md` — Code templates
- `ML_EXTRACTION_INDEX.md` — This document

---

**Status:** EXTRACTION COMPLETE — READY FOR DEVELOPMENT TEAM HANDOFF

All actionable implementation details extracted. No debate meta-discussion or rankings included.
Focus: building the system, not debating approaches.

