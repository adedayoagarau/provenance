# ML PIPELINE IMPLEMENTATION GUIDE
## Extracted from debate_9_ml_engineering.json (Claude Sonnet 4 Stage 1)

**Document generated:** 2026-03-24  
**Source:** Production ML Engineering Plan — 42,614 chars of actionable implementation detail  
**Status:** Ready for developer implementation

---

## PART 1: TRAINING DATA PIPELINE

### 1.1 Dataset Construction (MVP Requirements)

#### Human Text Requirements
- **Minimum viable:** 50,000 samples across registers
- **Production target:** 250,000+ samples
- **Distribution by source (% of total):**
  - Academic papers (25%): ArXiv, PubMed, university repositories
  - News articles (20%): Reuters, AP, verified journalist bylines
  - Creative writing (15%): Project Gutenberg, verified author works
  - Business communication (15%): SEC filings, press releases, verified corporate comms
  - Social media (10%): Verified human Twitter threads, Reddit posts with human verification
  - Technical documentation (10%): GitHub READMEs, Stack Overflow answers from high-rep users
  - Legal documents (5%): Court filings, legal briefs from known firms

#### AI Text Requirements
- **Minimum viable:** 75,000 samples (1.5:1 AI:human ratio for class imbalance)
- **Production target:** 375,000+ samples
- **Model coverage (% of AI samples):**
  - GPT-4/4-turbo (25%): Various temperature settings (0.2-1.0)
  - Claude-3 family (20%): Sonnet, Opus, Haiku
  - Gemini Pro/Ultra (15%)
  - Llama-2/3 70B+ (15%)
  - Mistral 7B/8x7B (10%)
  - GPT-3.5-turbo (10%)
  - Emerging models (5%): Keep buffer for new releases

#### Prompting Strategy Matrix (% of AI samples)
- Zero-shot direct (30%): "Write an article about X"
- Few-shot with examples (25%)
- Persona prompts (20%): "As a journalist, write..."
- Anti-detection prompts (15%): "Write like a human", "vary sentence length"
- Chain-of-thought (10%): "First outline, then write"

#### Humanizer-Processed Samples
- **Target:** 25,000 samples (10% of AI dataset)
- **Tools to process through:** QuillBot, Undetectable.ai, WordAI, manual editing
- **Verification strategy:** Ensure processing changed stylometric profile (compute feature deltas)

#### Class Balance Strategy
- **Training set:** 40% human, 50% raw AI, 10% humanized AI
- **Validation/Test set:** 50% human, 50% AI (including humanized)
- **Oversampling during training:** Oversample human text to achieve 2% FPR target

### 1.2 Data Quality Controls

#### Topic Confounding Prevention
- **Topic annotation:** Tag all samples with primary topic using DistilBERT classifier
- **Balanced sampling:** Ensure each topic has ≥30% human, ≥30% AI representation
- **Cross-topic validation:** Reserve 20% of each topic for final testing
- **Topic-stratified splits:** K-fold splits maintain topic distribution

#### Temporal Confounding Controls
- **Human text:** Only use samples from 2020+ (post-GPT era)
- **Timestamps:** Record creation date for all samples
- **Temporal splits:** Train on older data, validate on newer (simulate deployment reality)
- **Quarterly updates:** Refresh dataset every 3 months with new samples

#### Register Confounding Prevention
- **Register annotation:** Manual annotation of formality (1-5 scale: casual to legal)
- **Register balancing:** Each register must have ≥25% human samples
- **Register-specific thresholds:** Different calibration per register if accuracy analysis shows drift

---

## PART 2: FEATURE ENGINEERING PIPELINE

### 2.1 Feature Selection Algorithm

```
1. Correlation filtering: Remove features with r > 0.95 correlation
2. Mutual information: Top 200 features by MI with target
3. LASSO regularization: α tuned via cross-validation
4. Recursive feature elimination: RFE with SVM, eliminate 10% per iteration
5. Stability selection: Bootstrap 100 times, keep features selected >80% of time
```

**Target feature count:** 50-75 features (sufficient signal, avoid overfitting, fast inference)

### 2.2 Feature Categories (Distribution)

| Category | Count | Examples |
|----------|-------|----------|
| Lexical diversity | 8-10 | TTR, MTLD, HDD, VOCD |
| Syntactic patterns | 15-20 | POS trigrams, parse depth, constituent ratios |
| Semantic coherence | 8-12 | Coherence, topic entropy, semantic density |
| Burstiness/rhythm | 10-15 | Burstiness coefficient, sentence rhythm, pause patterns |
| Function word patterns | 10-15 | Function word frequencies, stopword patterns |

### 2.3 Correlated Feature Handling

- **Clustering:** Group features by correlation (r > 0.8), pick best from each cluster
- **PCA alternative:** For highly correlated groups, replace with first principal component
- **Domain knowledge rule:** Preserve interpretable features over purely statistical ones

### 2.4 Register-Specific Features

- **Core features:** Same 40 features across all registers
- **Register-specific additions:** 10-35 additional features per register
  - **Academic:** Citation patterns, technical vocabulary density
  - **Social:** Emoji usage, abbreviation patterns, informality markers
  - **Business:** Jargon density, formality markers, document structure patterns

### 2.5 Feature Stability & Drift Monitoring

**Stability metrics to track:**
- **Feature drift:** Monthly tracking of feature mean/std
- **Discriminative power:** Monthly re-evaluation of feature importance in retrained models
- **Alert thresholds:** >20% change in feature importance OR >2σ change in distribution

**LLM Evolution Response:**
- **New model evaluation:** Test feature effectiveness on new LLM outputs within 48 hours of release
- **Feature ranking updates:** Monthly re-ranking of feature importance
- **Rapid response features:** Maintain list of 20 "emergency" features that work across model generations

**Retraining cadence:**
- **Monthly:** Feature importance re-evaluation
- **Quarterly:** Full feature selection pipeline re-run
- **Event-driven:** New major LLM release triggers immediate 48-hour evaluation

---

## PART 3: MODEL ARCHITECTURE & TRAINING

### 3.1 Primary Model: XGBoost

```python
xgb_params = {
    'max_depth': 6,
    'learning_rate': 0.1,
    'n_estimators': 500,
    'subsample': 0.8,
    'colsample_bytree': 0.8,
    'reg_alpha': 0.1,           # L1 regularization
    'reg_lambda': 1.0,          # L2 regularization
    'scale_pos_weight': 1.25,   # Slight bias toward recall
    'objective': 'binary:logistic',
    'eval_metric': 'auc'
}
```

**Python libraries:**
- `xgboost` — Main gradient boosting
- `scikit-learn` — Cross-validation, metrics
- `optuna` — Hyperparameter optimization

### 3.2 Secondary Model: Neural Network (for ONNX export)

```python
nn_architecture = {
    'input_dim': 75,            # Final feature count
    'hidden_layers': [128, 64, 32],
    'dropout': 0.3,
    'activation': 'relu',
    'output_activation': 'sigmoid',
    'batch_norm': True
}
```

**Python libraries:**
- `torch` or `tensorflow` — Neural network implementation
- `skl2onnx` or `torch.onnx` — Export to ONNX format

### 3.3 Tertiary Model: SVM (RBF kernel)

- **Purpose:** Robustness to outliers, different inductive bias
- **Rationale:** Ensemble diversity improves performance on unseen distributions

### 3.4 Ensemble Strategy

```
Weighted average of calibrated probabilities:
- XGBoost:     70% weight (best overall performance)
- Neural Net:  20% weight (different inductive bias)
- SVM-RBF:     10% weight (robustness to outliers)
```

**Model selection criteria (in order):**
1. **TPR@2%FPR** (primary): Must exceed 95%
2. **Calibration quality (ECE):** < 0.05
3. **Inference speed:** <100ms for feature extraction + inference
4. **Robustness:** <5% performance drop on adversarial test set
5. **Interpretability:** Feature importance must be stable and sensible

---

## PART 4: CALIBRATION PIPELINE

### 4.1 Two-Stage Calibration Approach

```python
class TwoStageCalibrator:
    def __init__(self):
        self.temperature_scaler = TemperatureScaling()
        self.isotonic_calibrator = IsotonicRegression(out_of_bounds='clip')
    
    def fit(self, logits, labels, validation_split=0.3):
        # Stage 1: Temperature scaling on validation set
        val_logits, cal_logits, val_labels, cal_labels = train_test_split(
            logits, labels, test_size=validation_split
        )
        self.temperature_scaler.fit(val_logits, val_labels)
        temp_calibrated = self.temperature_scaler.transform(cal_logits)
        
        # Stage 2: Isotonic regression on remaining data
        self.isotonic_calibrator.fit(temp_calibrated, cal_labels)
```

**Python libraries:**
- `sklearn.calibration.CalibratedClassifierCV`
- `sklearn.isotonic.IsotonicRegression`

### 4.2 Calibration Target

- **Expected Calibration Error (ECE):** < 0.05
- **Brier Score:** < 0.02
- **Definition:** Model confidence should match actual accuracy (e.g., 90% confidence → 90% accuracy)

---

## PART 5: THRESHOLD OPTIMIZATION

### 5.1 Business-Objective Threshold Selection

```python
def business_objective(thresholds, predictions, labels, costs):
    """
    Optimize for business value:
    - False positive cost: High (customer trust damage)
    - False negative cost: Medium (missed detection)
    - True positive value: High (correct detection)
    """
    fpr, fnr = compute_error_rates(thresholds, predictions, labels)
    return costs['fp'] * fpr + costs['fn'] * fnr - costs['tp'] * (1 - fnr)

# Cost matrix (customer-configurable)
default_costs = {
    'fp': 10.0,    # False positive penalty
    'fn': 3.0,     # False negative penalty  
    'tp': 5.0,     # True positive reward
}
```

### 5.2 Primary Threshold: 2% FPR Operating Point

**Algorithm:**
1. Compute ROC curve across all threshold values
2. Find threshold where FPR = exactly 2%
3. Record resulting TPR (target: ≥95%)
4. Use this threshold as default in production

**Python libraries:**
- `sklearn.metrics.roc_curve` — Compute threshold points
- `scipy.optimize.brentq` — Find exact 2% FPR threshold

### 5.3 Register-Specific Thresholds

- Compute separate thresholds per register if accuracy variance >5%
- Use register classifier to determine which threshold to apply at inference time

---

## PART 6: EVALUATION FRAMEWORK

### 6.1 Test Set Construction

**Test set size:** 50,000 samples (for robust metrics)

**Stratification dimensions:**
- Source model (15% each major LLM)
- Register (proportional to expected production usage)
- Length (20% per bucket: <500, 500-1000, 1000-2000, 2000-3000, 3000+)
- Difficulty (20% easy, 60% medium, 20% hard — assessed by human review)
- Temporal (quarterly splits: train on Q1-Q2, test on Q3-Q4)

**Leakage prevention:**
- **Author separation:** No author appears in both train and test
- **Temporal separation:** Test set is 3+ months newer than train
- **Source separation:** Different data sources where possible
- **Content hashing:** No duplicate or near-duplicate content

### 6.2 Specialized Test Sets

Build these in parallel with main test set:

| Name | Size | Purpose |
|------|------|---------|
| Adversarial | 10,000 | Samples from humanizer tools (QuillBot, Undetectable.ai, etc) |
| Academic | 15,000 | From academic sources (ArXiv, PubMed) |
| Social media | 10,000 | From social platforms |
| Long-form | 5,000 | >3,000 words (test scalability) |
| Emerging model | 5,000 | From newest LLMs (e.g., GPT-4.5, Claude 3.5) |

### 6.3 Primary Metrics (in order of importance)

| Metric | Target | Definition |
|--------|--------|-----------|
| **TPR@2%FPR** | ≥95% | **GO/NO-GO METRIC** — True positive rate when FPR = 2% |
| **ECE** | <0.05 | Expected Calibration Error — matches confidence to accuracy |
| **AUC-ROC** | >0.98 | Area under ROC curve |
| **Precision@90%Recall** | >92% | Precision when recall = 90% |
| **F1 Score** | >0.95 | Harmonic mean of precision & recall |

### 6.4 Secondary Metrics

- **Accuracy:** Overall correctness
- **Brier Score:** Probabilistic accuracy (lower is better)
- **Log Loss:** Probability quality
- **Specificity:** True negative rate (1 - FPR)
- **Matthews Correlation Coefficient:** Balanced metric (works with imbalanced data)

### 6.5 Fairness Metrics

- **FPR by register:** Max 3x difference between registers
- **FPR by length:** Max 2x difference between length buckets  
- **FPR by non-native speakers:** Max 1.5x difference vs. native speakers
- **Performance parity:** TPR difference <5% across demographic groups

### 6.6 Go/No-Go Criteria for Deployment

**Must meet BOTH:**
- TPR@2%FPR ≥ 95%
- ECE < 0.05

If either fails, do NOT deploy. Retrain with different hyperparameters or data.

### 6.7 K-Fold Cross-Validation

- **Folds:** 5-fold stratified CV (maintain class balance in each fold)
- **Reporting:** Mean ± std of primary metrics across folds
- **Early stopping:** If any fold fails TPR@2%FPR threshold, investigate why

---

## PART 7: DEPLOYMENT & MONITORING

### 7.1 ONNX Export Process

**Python workflow:**
```python
import xgboost as xgb
import skl2onnx
from skl2onnx.common.data_types import FloatTensorType

# Convert XGBoost to ONNX
initial_type = [('float_input', FloatTensorType([None, 75]))]
onnx_model = skl2onnx.convert_sklearn(xgb_model, initial_types=initial_type)

# Save ONNX model
with open('provenance_model.onnx', 'wb') as f:
    f.write(onnx_model.SerializeToString())
```

**Rust inference (ort crate):**
```rust
use ort::{Environment, SessionBuilder, Value, Tensor};

pub struct ProvenanceModel {
    session: ort::Session,
    feature_extractor: FeatureExtractor,
    calibrator: Calibrator,
    scaler: ZScoreNormalizer,
}

impl ProvenanceModel {
    pub fn predict(&self, text: &str) -> Result<f32, ProvenanceError> {
        let features = self.feature_extractor.extract(text)?;
        let normalized = self.scaler.transform(&features)?;
        let input_tensor = Tensor::from_array((&[], &[normalized]))?;
        let outputs = self.session.run(vec![input_tensor])?;
        let raw_score = outputs[0].extract_tensor()?.view()[0];
        Ok(self.calibrator.calibrate(raw_score))
    }
}
```

### 7.2 Performance Budget

- **Feature extraction:** <1.5 seconds for 5,000 words
- **Model inference:** <50ms (ONNX via ort)
- **Total pipeline:** <2 seconds end-to-end
- **Memory usage:** <500MB per inference worker
- **Throughput:** >100 documents/minute/core

### 7.3 Model Versioning Metadata

```rust
pub struct ModelVersion {
    pub version: String,                    // "v2024.03.15"
    pub features: Vec<String>,              // Feature names in order
    pub model_hash: String,                 // SHA256 of ONNX model
    pub calibration_params: CalibrationParams,
    pub performance_metrics: PerformanceMetrics,
    pub compatibility: CompatibilityInfo,
}
```

---

## PART 8: PRODUCTION MONITORING

### 8.1 Core Metrics Dashboard

```yaml
monitoring_metrics:
  prediction_distribution:
    - score_histogram: hourly buckets
    - score_percentiles: p5, p25, p50, p75, p95
    - tier_distribution: percentage in each tier (HUMAN / LIKELY / MIXED / AI_ASSISTED / AI_GENERATED)
    
  feature_drift:
    - feature_means: daily rolling average
    - feature_stdevs: daily rolling standard deviation
    - correlation_matrix: weekly feature correlations
    - psi_score: Population Stability Index (alert if > 0.1)
    
  performance_monitoring:
    - inference_latency: p50, p95, p99 (ms)
    - throughput: requests/second
    - error_rate: failed predictions / total predictions
    - timeout_rate: percentage timing out
```

### 8.2 Drift Detection Triggers

**Alert conditions (any one triggers retraining evaluation):**
- FPR breach: > 2.5% (target: 2%)
- Feature drift: PSI > 0.1 on > 20% of features
- Score distribution shift: KS statistic > 0.15 (vs. baseline)
- Calibration degradation: weekly ECE > 0.05
- New LLM release: evaluate within 48 hours

### 8.3 Daily Validation Harness

```rust
pub struct ValidationMonitor {
    human_validation_set: Vec<Document>,  // 5,000 known-human docs
    ai_validation_set: Vec<Document>,     // 5,000 known-AI docs
    target_fpr: f32,                      // 2%
    target_tpr: f32,                      // 95%
    
    pub fn daily_validation(&self, model: &ProvenanceModel) -> ValidationReport {
        let human_scores = self.score_documents(&self.human_validation_set, model);
        let ai_scores = self.score_documents(&self.ai_validation_set, model);
        
        ValidationReport {
            fpr: self.compute_fpr(&human_scores),
            tpr: self.compute_tpr(&ai_scores),
            timestamp: Utc::now(),
            alert_triggered: fpr > 0.025 || tpr < 0.92,
        }
    }
}
```

### 8.4 Retraining Pipeline

**Triggers:**
- Monthly: Always run feature importance re-evaluation
- Quarterly: Full feature selection pipeline re-run
- FPR breach: Immediate retraining evaluation
- New LLM release: Immediate evaluation
- TPR drop: If <93%, trigger retraining

**Retraining workflow:**
1. Collect new validation data from production (last 30 days)
2. Update feature distributions and compute feature drift
3. Run feature selection pipeline
4. Train new candidate model on all available data
5. Validate against go/no-go criteria
6. Shadow deployment (run alongside current model for 1 week)
7. A/B test (gradual rollout to 10% → 50% → 100% of traffic)

---

## PART 9: ADDRESSING THE HARD ML PROBLEMS

### 9.1 Distribution Shift Robustness (for unseen LLMs)

**Problem:** Each new LLM generation changes the "AI" distribution fundamentally.

**Solution: Adversarial Meta-Learning**

```python
class AdversarialMetaLearner:
    """
    Train model to be robust to unseen AI distributions by simulating
    distribution shifts during training.
    """
    def __init__(self):
        self.base_models = ['gpt4', 'claude3', 'gemini', 'llama3']
        self.perturbation_strategies = [
            'temperature_shift',     # Different sampling temperatures
            'prompt_variation',      # Different prompt styles  
            'post_processing',       # Different editing styles
            'hybrid_generation'      # Human-AI collaboration
        ]
    
    def create_meta_training_splits(self, data):
        """Create training splits that simulate new model emergence"""
        meta_splits = []
        for held_out_model in self.base_models:
            # Train on all models except held_out_model
            train_data = data.exclude_model(held_out_model)
            
            # Create synthetic "new model" data by perturbing held_out_model
            synthetic_new_model = self.create_synthetic_distribution(
                data.filter_model(held_out_model)
            )
            
            # Test on both original and synthetic
            test_data = data.filter_model(held_out_model) + synthetic_new_model
            meta_splits.append((train_data, test_data))
        
        return meta_splits
    
    def train_robust_model(self, meta_splits):
        """Train ensemble that performs well across all splits"""
        ensemble_weights = self.optimize_ensemble_weights(meta_splits)
        return RobustEnsemble(self.base_models, ensemble_weights)
```

### 9.2 Uncertainty-Aware Features

Focus on linguistic universals that should remain stable:
- **Basic syntax patterns:** Subject-verb-object ordering
- **Function word usage:** Frequency of "the", "a", "and", etc.
- **Fundamental coherence measures:** Semantic continuity

**Avoid model-specific artifacts:**
- Specific repetition patterns (too brittle)
- Model-specific vocabulary preferences (will change with new LLMs)
- Generation-specific formatting quirks (encoding dependent)

### 9.3 Robust Feature Selection Process

```python
def select_robust_features(feature_candidates, robustness_tests):
    """
    Select features that remain stable under various perturbations.
    """
    robust_features = []
    
    for feature in feature_candidates:
        robustness_scores = []
        
        for test in robustness_tests:
            # Test feature stability under perturbation
            original_values = [feature.extract(doc) for doc in test.original_docs]
            perturbed_values = [feature.extract(doc) for doc in test.perturbed_docs]
            
            # Compute stability metric (rank correlation)
            stability = compute_rank_correlation(original_values, perturbed_values)
            robustness_scores.append(stability)
        
        # Keep feature if robust across all tests
        if min(robustness_scores) > ROBUSTNESS_THRESHOLD:  # e.g., 0.75
            robust_features.append(feature)
    
    return robust_features

robustness_tests = [
    SynonymSubstitutionTest(substitution_rate=0.05),
    ParaphrasingTest(tools=['quillbot', 'wordai']),
    SentenceReorderingTest(),
    PunctuationVariationTest(),
    WhitespaceVariationTest(),
]
```

### 9.4 Adversarial Testing

```python
class AdversarialEvaluator:
    def __init__(self):
        self.humanizer_tools = ['quillbot', 'undetectable', 'wordai', 'paraphraser']
        self.edit_strategies = ['light', 'medium', 'heavy']
        
    def evaluate_robustness(self, model, test_set):
        results = {}
        for tool in self.humanizer_tools:
            tool_samples = test_set.filter(tool=tool)
            results[f'{tool}_tpr'] = model.true_positive_rate(tool_samples)
            results[f'{tool}_score_shift'] = self.compute_score_shift(
                test_set.original, tool_samples, model
            )
        return results
```

**Go/No-Go for adversarial:**
- Detection rate on humanized samples: ≥80%
- No tool-specific blind spots (TPR per tool within 10% of overall)

---

## PART 10: IMPLEMENTATION TIMELINE

### Phase 1 (Months 1-2): Data & Features
- **Week 1-2:** Training data collection pipeline
- **Week 3-4:** Data quality validation and labeling
- **Week 5-6:** Feature extraction pipeline implementation
- **Week 7-8:** Feature selection and validation

### Phase 2 (Months 3-4): Model Development
- **Week 9-10:** Model training pipeline (XGBoost + Neural Net + SVM)
- **Week 11-12:** Calibration and threshold optimization
- **Week 13-14:** Ensemble development and validation
- **Week 15-16:** Comprehensive evaluation and ablation studies

### Phase 3 (Month 5): Production Engineering
- **Week 17-18:** ONNX export and Rust inference pipeline
- **Week 19-20:** Monitoring and alerting infrastructure
- **Week 21:** Performance optimization and stress testing
- **Week 22:** Security review and adversarial testing

### Phase 4 (Month 6): Deployment
- **Week 23:** Staged rollout to internal systems
- **Week 24:** A/B testing framework implementation
- **Week 25:** Customer pilot program
- **Week 26:** Full production deployment

---

## PART 11: KEY PYTHON LIBRARIES & TOOLS

### Data Processing
- `pandas` — Data manipulation, stratified splits
- `numpy` — Numerical computing
- `scikit-learn` — Feature engineering, preprocessing, metrics

### Feature Extraction
- `nltk` — POS tagging, tokenization
- `spacy` — NLP, entity extraction (if semantic features used)
- `textstat` — Readability metrics
- `scipy.stats` — Statistical computations

### Model Training
- `xgboost` — Primary model
- `torch` or `tensorflow` — Neural network (secondary)
- `scikit-learn` — SVM, preprocessing, cross-validation
- `optuna` — Hyperparameter optimization
- `optuna[sklearn]` — Integration for hyperparameter tuning

### Calibration & Evaluation
- `sklearn.calibration` — Temperature scaling, isotonic regression
- `scipy.special` — Sigmoid, logistic functions
- `sklearn.metrics` — ROC, precision-recall, calibration metrics

### ONNX & Deployment
- `skl2onnx` — Convert scikit-learn to ONNX
- `onnx` — ONNX format library (validation)
- `ort` (Rust crate) — ONNX Runtime for inference

### Monitoring & Evaluation
- `tqdm` — Progress bars
- `matplotlib`, `seaborn` — Visualization
- `pytest` — Unit testing
- `Great Expectations` — Data quality validation (optional)

---

## PART 12: DIRECTORY STRUCTURE

```
ml_pipeline/
├── data/
│   ├── raw/
│   │   ├── human/          # Raw human text sources
│   │   ├── ai/             # Raw AI-generated text
│   │   └── humanized/      # AI text processed through humanizers
│   ├── processed/
│   │   ├── train.jsonl     # Training set (stratified)
│   │   ├── val.jsonl       # Validation set
│   │   ├── test.jsonl      # Test set (held-out)
│   │   ├── test_adversarial.jsonl
│   │   ├── test_academic.jsonl
│   │   └── ...
│   └── metadata/
│       ├── topic_labels.json
│       ├── register_labels.json
│       └── temporal_splits.json
│
├── features/
│   ├── extractors.py       # Feature extraction code
│   ├── cache/              # Cached feature vectors
│   │   ├── train_features.npy
│   │   ├── train_features.pkl
│   │   └── ...
│   └── feature_list.json   # Final selected features (75 features)
│
├── models/
│   ├── training.py         # Training pipeline
│   ├── xgboost_model/      # Trained XGBoost
│   ├── neural_net_model/   # Trained neural net
│   ├── ensemble.py         # Ensemble logic
│   ├── calibrator.pkl      # Two-stage calibrator
│   └── onnx_model.onnx     # Exported ONNX model
│
├── evaluation/
│   ├── metrics.py          # Evaluation code
│   ├── results/
│   │   ├── primary_metrics.json
│   │   ├── fairness_metrics.json
│   │   ├── calibration_analysis.pkl
│   │   └── roc_curve.png
│   └── test_reports/
│
├── notebooks/
│   ├── 01_data_exploration.ipynb
│   ├── 02_feature_engineering.ipynb
│   ├── 03_model_training.ipynb
│   └── 04_evaluation.ipynb
│
├── scripts/
│   ├── collect_training_data.py
│   ├── prepare_datasets.py
│   ├── train_model.py
│   ├── evaluate_model.py
│   ├── export_onnx.py
│   └── monitor_drift.py
│
├── tests/
│   ├── test_features.py
│   ├── test_models.py
│   ├── test_calibration.py
│   └── test_metrics.py
│
├── config/
│   ├── data_config.yaml     # Data pipeline config
│   ├── feature_config.yaml  # Feature extraction config
│   ├── model_config.yaml    # Model hyperparameters
│   └── eval_config.yaml     # Evaluation thresholds
│
└── README.md
```

---

## PART 13: SUCCESS CRITERIA & GO/NO-GO GATES

### Pre-Deployment Gate (Phase 4 Entry)
- TPR@2%FPR ≥ 95% on held-out test set
- ECE < 0.05 across all registers
- Inference time <2 seconds for 5,000-word documents
- Adversarial TPR ≥ 80% (humanized samples)
- Feature importance stable week-to-week

### Post-Deployment Monitoring (First 4 weeks)
- FPR stays ≤2.5% (allow 0.5% buffer)
- No customer-reported false positives
- Inference latency p99 < 3 seconds
- Feature drift (PSI) < 0.1 on >80% of features
- Score distribution stable (KS statistic < 0.15)

### Rolling Success Metrics
- Monthly: TPR ≥ 93% (allow 2% degradation from training)
- Monthly: ECE < 0.07 (allow 0.02 degradation)
- Quarterly: Full retraining evaluation cycle

---

## PART 14: CRITICAL IMPLEMENTATION NOTES

### Hyperparameter Tuning
Do NOT hand-tune; use Optuna with early stopping:
```python
def objective(trial):
    params = {
        'max_depth': trial.suggest_int('max_depth', 5, 10),
        'learning_rate': trial.suggest_float('learning_rate', 0.01, 0.5),
        'n_estimators': trial.suggest_int('n_estimators', 100, 1000),
    }
    # Train and return cross-validation score
    return cross_val_score(params, X, y).mean()

study = optuna.create_study(direction='maximize')
study.optimize(objective, n_trials=100, n_jobs=-1)
```

### Feature Normalization (Critical!)
Use Z-score normalization, fitted on training set ONLY:
```python
scaler = StandardScaler()
X_train_scaled = scaler.fit_transform(X_train)
X_test_scaled = scaler.transform(X_test)  # Use training scaler!
```

### Class Imbalance Handling
Use combination of:
1. `scale_pos_weight` in XGBoost (set to 1.25-1.5)
2. Stratified K-fold splits
3. Oversampling human text during training (not SMOTE; just repeat)

### Reproducibility
Set seeds everywhere:
```python
import random, numpy as np, torch
random.seed(42)
np.random.seed(42)
torch.manual_seed(42)
```

---

**Document prepared:** 2026-03-24  
**Status:** Ready for handoff to development team  
**Next step:** Begin Phase 1 (Month 1-2) data collection pipeline

