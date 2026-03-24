# ML PIPELINE — PYTHON CODE TEMPLATES & QUICK START

**For developers implementing the pipeline.** Copy, modify, and integrate these templates into your codebase.

---

## 1. FEATURE EXTRACTION WITH CACHING

```python
# features/extractors.py
import numpy as np
import h5py
from pathlib import Path
from tqdm import tqdm
import pickle

class FeatureExtractor:
    def __init__(self, cache_dir: str = 'features/cache'):
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.feature_names = [
            'ttr', 'mtld', 'hdd', 'vocd',  # lexical diversity
            'pos_trigram_entropy', 'parse_depth', 'constituent_ratio',  # syntactic
            'coherence', 'topic_entropy', 'semantic_density',  # semantic
            # ... all 68-75 features
        ]
    
    def extract_batch(self, documents: list, use_cache=True) -> np.ndarray:
        """Extract features for batch of documents."""
        features_list = []
        cache_file = self.cache_dir / f'batch_{id(documents)}.h5'
        
        # Check cache first
        if use_cache and cache_file.exists():
            with h5py.File(cache_file, 'r') as f:
                return f['features'][:]
        
        # Extract features
        for doc in tqdm(documents, desc="Extracting features"):
            features = self._extract_single(doc)
            features_list.append(features)
        
        features_array = np.array(features_list)
        
        # Cache results
        if use_cache:
            with h5py.File(cache_file, 'w') as f:
                f.create_dataset('features', data=features_array, compression='gzip')
        
        return features_array
    
    def _extract_single(self, text: str) -> np.ndarray:
        """Extract features from single document."""
        features = {}
        
        # Lexical diversity
        words = text.lower().split()
        unique_words = len(set(words))
        features['ttr'] = unique_words / len(words) if words else 0
        # ... implement other features
        
        # Convert to ordered array matching self.feature_names
        return np.array([features.get(name, 0.0) for name in self.feature_names])


# Usage
extractor = FeatureExtractor()
documents = [doc1, doc2, doc3]
X = extractor.extract_batch(documents, use_cache=True)
print(f"Extracted shape: {X.shape}")  # (3, 68-75)
```

---

## 2. DATASET PREPARATION WITH STRATIFICATION

```python
# scripts/prepare_datasets.py
import pandas as pd
import numpy as np
from sklearn.model_selection import train_test_split, StratifiedKFold
import json

def prepare_datasets(data_file: str, output_dir: str = 'data/processed'):
    """Prepare train/val/test splits with stratification."""
    
    # Load data
    df = pd.read_json(data_file, lines=True)
    print(f"Loaded {len(df)} documents")
    
    # Ensure stratification columns exist
    assert 'label' in df.columns  # 0=human, 1=AI
    assert 'register' in df.columns
    assert 'length_bucket' in df.columns  # categorical
    
    # Create stratification column (combine register + length_bucket)
    df['strata'] = df['register'].astype(str) + '_' + df['length_bucket'].astype(str)
    
    # First split: train+val (80%) vs test (20%)
    train_val, test = train_test_split(
        df, 
        test_size=0.2, 
        random_state=42,
        stratify=df['strata']
    )
    
    # Second split: train (80%) vs val (20%)
    train, val = train_test_split(
        train_val,
        test_size=0.2,
        random_state=42,
        stratify=train_val['strata']
    )
    
    # Verify class balance
    print(f"Train set: {len(train)} docs, {train['label'].sum()} AI")
    print(f"Val set: {len(val)} docs, {val['label'].sum()} AI")
    print(f"Test set: {len(test)} docs, {test['label'].sum()} AI")
    
    # Save
    Path(output_dir).mkdir(parents=True, exist_ok=True)
    train.to_json(f'{output_dir}/train.jsonl', orient='records', lines=True)
    val.to_json(f'{output_dir}/val.jsonl', orient='records', lines=True)
    test.to_json(f'{output_dir}/test.jsonl', orient='records', lines=True)
    
    return train, val, test
```

---

## 3. XGBOOST TRAINING WITH EARLY STOPPING

```python
# models/training.py
import xgboost as xgb
from sklearn.preprocessing import StandardScaler
import numpy as np
import pickle

def train_xgboost(X_train, y_train, X_val, y_val):
    """Train XGBoost with early stopping."""
    
    # Normalize features (critical!)
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_val_scaled = scaler.transform(X_val)
    
    # Create DMatrix
    dtrain = xgb.DMatrix(X_train_scaled, label=y_train)
    dval = xgb.DMatrix(X_val_scaled, label=y_val)
    
    # Exact hyperparameters from guide
    params = {
        'max_depth': 6,
        'learning_rate': 0.1,
        'n_estimators': 500,
        'subsample': 0.8,
        'colsample_bytree': 0.8,
        'reg_alpha': 0.1,
        'reg_lambda': 1.0,
        'scale_pos_weight': 1.25,
        'objective': 'binary:logistic',
        'eval_metric': 'auc',
        'random_state': 42,
        'verbosity': 1,
    }
    
    # Train with early stopping
    evals = [(dtrain, 'train'), (dval, 'val')]
    evals_result = {}
    
    model = xgb.train(
        params,
        dtrain,
        num_boost_round=500,
        evals=evals,
        evals_result=evals_result,
        early_stopping_rounds=20,
        verbose_eval=10,
    )
    
    # Save model & scaler
    model.save_model('models/xgboost_model.json')
    with open('models/scaler.pkl', 'wb') as f:
        pickle.dump(scaler, f)
    
    print(f"Stopped at round {model.best_iteration}")
    print(f"Best validation AUC: {model.best_score:.4f}")
    
    return model, scaler, evals_result
```

---

## 4. NEURAL NETWORK (TORCH) WITH ONNX EXPORT

```python
# models/neural_net.py
import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader, TensorDataset

class ProvenanceNN(nn.Module):
    def __init__(self, input_dim=75, hidden_dims=[128, 64, 32], dropout=0.3):
        super().__init__()
        
        layers = []
        prev_dim = input_dim
        
        for hidden_dim in hidden_dims:
            layers.extend([
                nn.Linear(prev_dim, hidden_dim),
                nn.BatchNorm1d(hidden_dim),
                nn.ReLU(),
                nn.Dropout(dropout),
            ])
            prev_dim = hidden_dim
        
        # Output layer
        layers.append(nn.Linear(prev_dim, 1))
        layers.append(nn.Sigmoid())
        
        self.network = nn.Sequential(*layers)
    
    def forward(self, x):
        return self.network(x)

def train_neural_net(X_train, y_train, X_val, y_val, epochs=50):
    """Train neural network with early stopping."""
    
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
    model = ProvenanceNN().to(device)
    
    criterion = nn.BCELoss()
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    scheduler = optim.lr_scheduler.ReduceLROnPlateau(optimizer, mode='min', patience=5)
    
    # Data loaders
    train_dataset = TensorDataset(
        torch.FloatTensor(X_train),
        torch.FloatTensor(y_train).unsqueeze(1)
    )
    train_loader = DataLoader(train_dataset, batch_size=32, shuffle=True)
    
    val_dataset = TensorDataset(
        torch.FloatTensor(X_val),
        torch.FloatTensor(y_val).unsqueeze(1)
    )
    val_loader = DataLoader(val_dataset, batch_size=32, shuffle=False)
    
    best_val_loss = float('inf')
    patience_counter = 0
    
    for epoch in range(epochs):
        # Training
        model.train()
        train_loss = 0
        for X_batch, y_batch in train_loader:
            X_batch, y_batch = X_batch.to(device), y_batch.to(device)
            
            optimizer.zero_grad()
            outputs = model(X_batch)
            loss = criterion(outputs, y_batch)
            loss.backward()
            optimizer.step()
            
            train_loss += loss.item()
        
        # Validation
        model.eval()
        val_loss = 0
        with torch.no_grad():
            for X_batch, y_batch in val_loader:
                X_batch, y_batch = X_batch.to(device), y_batch.to(device)
                outputs = model(X_batch)
                loss = criterion(outputs, y_batch)
                val_loss += loss.item()
        
        val_loss /= len(val_loader)
        scheduler.step(val_loss)
        
        print(f"Epoch {epoch+1}/{epochs}, Train: {train_loss/len(train_loader):.4f}, Val: {val_loss:.4f}")
        
        # Early stopping
        if val_loss < best_val_loss:
            best_val_loss = val_loss
            patience_counter = 0
            torch.save(model.state_dict(), 'models/neural_net.pt')
        else:
            patience_counter += 1
            if patience_counter >= 15:
                print("Early stopping triggered")
                break
    
    # Export to ONNX
    model.load_state_dict(torch.load('models/neural_net.pt'))
    dummy_input = torch.randn(1, 75).to(device)
    torch.onnx.export(
        model,
        dummy_input,
        'models/provenance_nn.onnx',
        input_names=['features'],
        output_names=['score'],
        opset_version=11,
    )
    
    return model

```

---

## 5. CALIBRATION PIPELINE (TWO-STAGE)

```python
# models/calibration.py
from sklearn.calibration import CalibratedClassifierCV
from sklearn.isotonic import IsotonicRegression
import numpy as np
import pickle

class TwoStageCalibrator:
    def __init__(self):
        self.temp_scaler = None
        self.isotonic = None
    
    def fit(self, logits, labels, validation_split=0.3):
        """Fit temperature scaling then isotonic regression."""
        
        # Split into two calibration sets
        n = len(logits)
        split_idx = int(n * validation_split)
        indices = np.random.permutation(n)
        
        temp_idx = indices[:split_idx]
        iso_idx = indices[split_idx:]
        
        # Stage 1: Temperature scaling (on first set)
        from scipy.optimize import minimize
        
        def nll(t):
            """Negative log-likelihood with temperature."""
            scaled = logits[temp_idx] / t
            probs = 1 / (1 + np.exp(-scaled))
            return -np.mean(labels[temp_idx] * np.log(probs + 1e-10) + 
                           (1 - labels[temp_idx]) * np.log(1 - probs + 1e-10))
        
        result = minimize(nll, x0=1.0, method='Nelder-Mead')
        self.temperature = result.x[0]
        
        # Apply temperature to second set
        scaled_logits = logits[iso_idx] / self.temperature
        
        # Stage 2: Isotonic regression (on second set)
        self.isotonic = IsotonicRegression(out_of_bounds='clip')
        self.isotonic.fit(scaled_logits, labels[iso_idx])
    
    def calibrate(self, logit):
        """Apply both stages to a single logit."""
        scaled = logit / self.temperature
        return self.isotonic.predict([scaled])[0]
    
    def calibrate_batch(self, logits):
        """Apply both stages to batch of logits."""
        scaled = logits / self.temperature
        return self.isotonic.predict(scaled)
    
    def save(self, path):
        with open(path, 'wb') as f:
            pickle.dump(self, f)
    
    @staticmethod
    def load(path):
        with open(path, 'rb') as f:
            return pickle.load(f)
```

---

## 6. THRESHOLD OPTIMIZATION FOR 2% FPR

```python
# models/threshold_optimization.py
import numpy as np
from sklearn.metrics import roc_curve, confusion_matrix
import json

def find_threshold_for_target_fpr(y_true, y_scores, target_fpr=0.02):
    """Find threshold where FPR = exactly target_fpr."""
    
    fpr, tpr, thresholds = roc_curve(y_true, y_scores)
    
    # Find threshold closest to target FPR
    idx = np.argmin(np.abs(fpr - target_fpr))
    threshold = thresholds[idx]
    actual_fpr = fpr[idx]
    actual_tpr = tpr[idx]
    
    print(f"Target FPR: {target_fpr:.4f}")
    print(f"Actual FPR: {actual_fpr:.4f}")
    print(f"Corresponding TPR: {actual_tpr:.4f}")
    print(f"Threshold: {threshold:.4f}")
    
    # Validate: TPR should be ≥95%
    if actual_tpr < 0.95:
        print("WARNING: TPR < 95%. Model may not meet deployment criteria.")
    
    return threshold, actual_fpr, actual_tpr

def apply_threshold(y_scores, threshold):
    """Convert scores to binary predictions using threshold."""
    return (y_scores >= threshold).astype(int)

def compute_metrics_at_threshold(y_true, y_scores, threshold):
    """Compute FPR, TPR, precision, recall at given threshold."""
    y_pred = apply_threshold(y_scores, threshold)
    
    tn, fp, fn, tp = confusion_matrix(y_true, y_pred).ravel()
    
    fpr = fp / (fp + tn)
    tpr = tp / (tp + fn)
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0
    recall = tpr
    
    return {
        'fpr': fpr,
        'tpr': tpr,
        'precision': precision,
        'recall': recall,
        'f1': 2 * (precision * recall) / (precision + recall) if (precision + recall) > 0 else 0,
    }

# Usage
from sklearn.metrics import roc_curve

y_true = np.array([0, 1, 1, 0, 1, 0, 1, 1])
y_scores = np.array([0.1, 0.8, 0.9, 0.2, 0.7, 0.3, 0.85, 0.95])

threshold, fpr, tpr = find_threshold_for_target_fpr(y_true, y_scores, target_fpr=0.02)

# Save threshold
thresholds_dict = {
    'global': float(threshold),
    'register_specific': {}  # Add per-register thresholds if needed
}
with open('models/thresholds.json', 'w') as f:
    json.dump(thresholds_dict, f)
```

---

## 7. CALIBRATION QUALITY ASSESSMENT (ECE)

```python
# evaluation/calibration_metrics.py
import numpy as np
from sklearn.calibration import calibration_curve

def expected_calibration_error(y_true, y_pred_prob, n_bins=10):
    """Compute Expected Calibration Error."""
    
    bin_boundaries = np.linspace(0, 1, n_bins + 1)
    bin_lowers = bin_boundaries[:-1]
    bin_uppers = bin_boundaries[1:]
    
    accuracies = []
    confidences = []
    counts = []
    
    for lower, upper in zip(bin_lowers, bin_uppers):
        in_bin = (y_pred_prob > lower) & (y_pred_prob <= upper)
        prop_in_bin = in_bin.mean()
        
        if prop_in_bin > 0:
            accuracy_in_bin = y_true[in_bin].mean()
            avg_confidence_in_bin = y_pred_prob[in_bin].mean()
            
            accuracies.append(accuracy_in_bin)
            confidences.append(avg_confidence_in_bin)
            counts.append(prop_in_bin)
    
    ece = np.sum([prop * np.abs(acc - conf) 
                  for prop, acc, conf in zip(counts, accuracies, confidences)])
    
    return ece

def brier_score(y_true, y_pred_prob):
    """Compute Brier score (MSE of probability estimates)."""
    return np.mean((y_pred_prob - y_true) ** 2)

# Usage
ece = expected_calibration_error(y_true, y_pred_prob, n_bins=10)
brier = brier_score(y_true, y_pred_prob)

print(f"ECE: {ece:.4f} (target: <0.05)")
print(f"Brier: {brier:.4f} (target: <0.02)")
```

---

## 8. FEATURE DRIFT MONITORING (PSI)

```python
# scripts/monitor_drift.py
import numpy as np

def population_stability_index(reference_dist, current_dist, n_bins=10):
    """Compute PSI between reference and current distributions."""
    
    # Create bins using reference distribution
    bin_boundaries = np.percentile(reference_dist, np.linspace(0, 100, n_bins + 1))
    bin_boundaries[0] = -np.inf
    bin_boundaries[-1] = np.inf
    
    # Compute proportions in each bin
    ref_props = np.histogram(reference_dist, bins=bin_boundaries)[0] / len(reference_dist)
    cur_props = np.histogram(current_dist, bins=bin_boundaries)[0] / len(current_dist)
    
    # Avoid log(0)
    ref_props = np.clip(ref_props, 1e-10, 1)
    cur_props = np.clip(cur_props, 1e-10, 1)
    
    # Compute PSI
    psi = np.sum(cur_props * (np.log(cur_props / ref_props)))
    
    return psi

def check_drift_alert(features_reference, features_current, feature_names):
    """Check if any feature has drifted (PSI > 0.1)."""
    
    alerts = []
    for i, fname in enumerate(feature_names):
        psi = population_stability_index(
            features_reference[:, i],
            features_current[:, i]
        )
        
        if psi > 0.1:
            alerts.append({
                'feature': fname,
                'psi': psi,
                'alert': True
            })
    
    print(f"Features with drift (PSI > 0.1): {len(alerts)} / {len(feature_names)}")
    for alert in alerts[:5]:  # Show top 5
        print(f"  {alert['feature']}: PSI = {alert['psi']:.4f}")
    
    return alerts
```

---

## 9. QUICK-START: TRAIN FULL PIPELINE

```python
# scripts/train_model.py
"""
Quick-start script: train full ML pipeline from scratch.
Usage: python scripts/train_model.py
"""

import numpy as np
from pathlib import Path

# Import your modules
from features.extractors import FeatureExtractor
from models.training import train_xgboost
from models.neural_net import train_neural_net
from models.calibration import TwoStageCalibrator
from models.threshold_optimization import find_threshold_for_target_fpr
from evaluation.calibration_metrics import expected_calibration_error, brier_score

def main():
    # Paths
    data_dir = Path('data/processed')
    model_dir = Path('models')
    eval_dir = Path('evaluation')
    model_dir.mkdir(parents=True, exist_ok=True)
    eval_dir.mkdir(parents=True, exist_ok=True)
    
    print("=" * 80)
    print("PHASE 2: MODEL DEVELOPMENT")
    print("=" * 80)
    
    # 1. Load data
    print("\n[1/6] Loading data...")
    import json
    train_data = []
    with open(data_dir / 'train.jsonl') as f:
        for line in f:
            train_data.append(json.loads(line))
    
    val_data = []
    with open(data_dir / 'val.jsonl') as f:
        for line in f:
            val_data.append(json.loads(line))
    
    print(f"  Train: {len(train_data)} docs")
    print(f"  Val: {len(val_data)} docs")
    
    # 2. Extract features
    print("\n[2/6] Extracting features...")
    extractor = FeatureExtractor()
    X_train = extractor.extract_batch([d['text'] for d in train_data])
    X_val = extractor.extract_batch([d['text'] for d in val_data])
    y_train = np.array([d['label'] for d in train_data])
    y_val = np.array([d['label'] for d in val_data])
    
    print(f"  X_train shape: {X_train.shape}")
    print(f"  X_val shape: {X_val.shape}")
    
    # 3. Train XGBoost
    print("\n[3/6] Training XGBoost...")
    xgb_model, scaler, evals_result = train_xgboost(X_train, y_train, X_val, y_val)
    
    # 4. Train neural network
    print("\n[4/6] Training neural network...")
    nn_model = train_neural_net(X_train, y_train, X_val, y_val, epochs=50)
    
    # 5. Calibrate
    print("\n[5/6] Calibrating ensemble...")
    
    # Get raw predictions
    X_val_scaled = scaler.transform(X_val)
    import xgboost as xgb
    dval = xgb.DMatrix(X_val_scaled)
    xgb_logits = xgb_model.predict(dval)
    
    import torch
    nn_model.eval()
    with torch.no_grad():
        nn_logits = nn_model(torch.FloatTensor(X_val)).numpy().flatten()
    
    # Ensemble logits (average)
    ensemble_logits = 0.7 * xgb_logits + 0.3 * nn_logits
    
    # Calibrate
    calibrator = TwoStageCalibrator()
    calibrator.fit(ensemble_logits, y_val)
    calibrator.save(model_dir / 'calibrator.pkl')
    
    calibrated_probs = calibrator.calibrate_batch(ensemble_logits)
    ece = expected_calibration_error(y_val, calibrated_probs)
    brier = brier_score(y_val, calibrated_probs)
    print(f"  ECE: {ece:.4f} (target: <0.05)")
    print(f"  Brier: {brier:.4f} (target: <0.02)")
    
    # 6. Optimize threshold
    print("\n[6/6] Optimizing threshold for 2% FPR...")
    threshold, fpr, tpr = find_threshold_for_target_fpr(y_val, calibrated_probs, target_fpr=0.02)
    
    print("\n" + "=" * 80)
    print("TRAINING COMPLETE")
    print("=" * 80)
    print(f"TPR@2%FPR: {tpr:.4f} (target: ≥0.95) {'✓ PASS' if tpr >= 0.95 else '✗ FAIL'}")
    print(f"ECE: {ece:.4f} (target: <0.05) {'✓ PASS' if ece < 0.05 else '✗ FAIL'}")
    print("\nModels saved to:", model_dir)

if __name__ == '__main__':
    main()
```

---

## 10. PRODUCTION INFERENCE (RUST TEMPLATE)

```rust
// src/ml/inference.rs
use ort::{Environment, SessionBuilder, Value, Tensor};
use ndarray::Array1;
use std::path::Path;

pub struct ProvenanceModel {
    session: ort::Session,
    feature_extractor: FeatureExtractor,
    calibrator: Calibrator,
    scaler: ZScoreNormalizer,
    threshold: f32,
}

impl ProvenanceModel {
    pub fn load(model_path: &str, calibrator_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize ONNX runtime
        let environment = Environment::builder()
            .with_execution_providers([ExecutionProvider::cpu()])
            .build()?;
        
        // Load model
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;
        
        // Load calibrator and scaler
        let calibrator = Calibrator::load(calibrator_path)?;
        let scaler = ZScoreNormalizer::load_from_training_set()?;
        
        Ok(ProvenanceModel {
            session,
            feature_extractor: FeatureExtractor::new(),
            calibrator,
            scaler,
            threshold: 0.5,  // Updated from thresholds.json
        })
    }
    
    pub fn predict(&self, text: &str) -> Result<PredictionResult, Box<dyn std::error::Error>> {
        // 1. Extract features
        let features = self.feature_extractor.extract(text)?;
        
        // 2. Normalize
        let normalized = self.scaler.normalize(&features)?;
        
        // 3. Create input tensor
        let input_tensor = Tensor::from_array(([1, 68], normalized.to_vec()))?;
        
        // 4. Run inference
        let outputs = self.session.run(vec![input_tensor])?;
        let output_data = outputs[0].extract_tensor::<f32>()?;
        let raw_score = output_data.view()[0];
        
        // 5. Calibrate
        let calibrated_score = self.calibrator.calibrate(raw_score);
        
        // 6. Convert to tier
        let tier = match calibrated_score {
            s if s < 0.15 => Tier::HumanAuthored,
            s if s < 0.35 => Tier::LikelyHuman,
            s if s < 0.65 => Tier::Mixed,
            s if s < 0.85 => Tier::AiAssisted,
            _ => Tier::AiGenerated,
        };
        
        Ok(PredictionResult {
            score: calibrated_score,
            tier,
            confidence: (calibrated_score - 0.5).abs() * 2.0,
            timestamp: std::time::Instant::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Tier {
    HumanAuthored,
    LikelyHuman,
    Mixed,
    AiAssisted,
    AiGenerated,
}

#[derive(Debug)]
pub struct PredictionResult {
    pub score: f32,
    pub tier: Tier,
    pub confidence: f32,
    pub timestamp: std::time::Instant,
}
```

---

## 11. MONITORING HARNESS (DAILY VALIDATION)

```python
# scripts/daily_validation.py
"""
Run daily to validate model performance on known samples.
Alert if FPR > 2.5% or TPR < 92%.
"""

import json
import numpy as np
from datetime import datetime
from pathlib import Path

def load_validation_sets():
    """Load known human and AI validation samples."""
    human_docs = []
    ai_docs = []
    
    # Load from validation cache
    with open('data/processed/val.jsonl') as f:
        for line in f:
            doc = json.loads(line)
            if doc['label'] == 0:
                human_docs.append(doc)
            else:
                ai_docs.append(doc)
    
    return human_docs[:5000], ai_docs[:5000]

def compute_metrics(model, human_docs, ai_docs):
    """Score validation sets and compute FPR/TPR."""
    
    # Score human docs (should get low scores)
    human_scores = [model.predict(doc['text']).score for doc in human_docs]
    human_below_threshold = sum(1 for s in human_scores if s < 0.5)
    fpr = 1 - (human_below_threshold / len(human_scores))
    
    # Score AI docs (should get high scores)
    ai_scores = [model.predict(doc['text']).score for doc in ai_docs]
    ai_above_threshold = sum(1 for s in ai_scores if s >= 0.5)
    tpr = ai_above_threshold / len(ai_scores)
    
    return {
        'fpr': fpr,
        'tpr': tpr,
        'human_mean_score': np.mean(human_scores),
        'ai_mean_score': np.mean(ai_scores),
        'human_median_score': np.median(human_scores),
        'ai_median_score': np.median(ai_scores),
    }

def main():
    # Load model
    from ml.inference import ProvenanceModel
    model = ProvenanceModel.load('models/model.onnx', 'models/calibrator.pkl')
    
    # Load validation sets
    human_docs, ai_docs = load_validation_sets()
    
    # Compute metrics
    metrics = compute_metrics(model, human_docs, ai_docs)
    
    # Log results
    results_dir = Path('evaluation/daily_validation')
    results_dir.mkdir(parents=True, exist_ok=True)
    
    log_file = results_dir / f"{datetime.now().isoformat()}.json"
    with open(log_file, 'w') as f:
        json.dump({
            'timestamp': datetime.now().isoformat(),
            **metrics
        }, f, indent=2)
    
    # Check alerts
    alerts = []
    if metrics['fpr'] > 0.025:
        alerts.append(f"FPR {metrics['fpr']:.4f} > 2.5% THRESHOLD")
    if metrics['tpr'] < 0.92:
        alerts.append(f"TPR {metrics['tpr']:.4f} < 92% THRESHOLD")
    
    # Print summary
    print(f"[{datetime.now().isoformat()}] Daily Validation")
    print(f"  FPR: {metrics['fpr']:.4f}")
    print(f"  TPR: {metrics['tpr']:.4f}")
    
    if alerts:
        print(f"  ⚠️  ALERTS: {', '.join(alerts)}")
        # Send alert (email, Slack, etc.)
    else:
        print(f"  ✓ All metrics nominal")

if __name__ == '__main__':
    main()
```

---

**Templates prepared:** 2026-03-24  
**Status:** Ready for copy-paste integration

