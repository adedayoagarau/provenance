//! Platform API — high-throughput batch processing, webhooks, and usage billing.
//!
//! Extends the base API (Phase 24) with enterprise features: bulk document
//! submission, webhook notifications, usage tracking, and billing integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A batch submission of multiple documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSubmission {
    /// Unique batch identifier.
    pub batch_id: String,
    /// Client/tenant who submitted the batch.
    pub tenant_id: String,
    /// Individual document entries.
    pub entries: Vec<BatchEntry>,
    /// Submission timestamp.
    pub submitted_at: String,
    /// Webhook URL for completion notification.
    pub webhook_url: Option<String>,
    /// Batch-level analysis options.
    pub options: BatchOptions,
}

/// A single entry in a batch submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEntry {
    /// Client's identifier for this document.
    pub external_id: String,
    /// File name.
    pub file_name: String,
    /// Analysis job ID (assigned after submission).
    pub job_id: Option<String>,
    /// Status.
    pub status: BatchEntryStatus,
    /// Result summary (populated after analysis).
    pub result_summary: Option<BatchEntryResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchEntryStatus {
    Pending,
    Queued,
    Analyzing,
    Completed,
    Failed,
}

/// Summary result for a batch entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEntryResult {
    pub acs_score: f64,
    pub acs_margin: f64,
    pub pii_score: f64,
    pub anomaly_count: usize,
    pub register: String,
    pub word_count: usize,
}

/// Options for batch processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptions {
    /// Maximum concurrent analyses.
    pub concurrency: usize,
    /// Whether to continue on individual failures.
    pub continue_on_error: bool,
    /// Profile path for comparison (applied to all entries).
    pub profile_path: Option<String>,
    /// Result format for export.
    pub export_format: ExportFormat,
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            concurrency: 4,
            continue_on_error: true,
            profile_path: None,
            export_format: ExportFormat::JsonArray,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    JsonArray,
    Csv,
    Ndjson,
}

/// Batch progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    pub batch_id: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub progress_pct: f64,
    pub estimated_remaining_secs: Option<u64>,
}

impl BatchProgress {
    pub fn from_submission(batch: &BatchSubmission) -> Self {
        let total = batch.entries.len();
        let completed = batch
            .entries
            .iter()
            .filter(|e| e.status == BatchEntryStatus::Completed)
            .count();
        let failed = batch
            .entries
            .iter()
            .filter(|e| e.status == BatchEntryStatus::Failed)
            .count();
        let pending = total - completed - failed;
        let progress = if total > 0 {
            (completed + failed) as f64 / total as f64 * 100.0
        } else {
            0.0
        };

        Self {
            batch_id: batch.batch_id.clone(),
            total,
            completed,
            failed,
            pending,
            progress_pct: progress,
            estimated_remaining_secs: None,
        }
    }
}

// ─── Webhook Notifications ───────────────────────────────────────────

/// Webhook event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEventType {
    /// Single analysis completed.
    AnalysisCompleted,
    /// Single analysis failed.
    AnalysisFailed,
    /// Batch processing completed.
    BatchCompleted,
    /// High-severity anomaly detected.
    AnomalyDetected,
    /// Usage quota approaching limit.
    QuotaWarning,
}

/// Webhook notification payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookNotification {
    pub event: WebhookEventType,
    pub timestamp: String,
    pub tenant_id: String,
    pub data: serde_json::Value,
}

/// Webhook endpoint configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    pub url: String,
    pub secret: String,
    pub events: Vec<WebhookEventType>,
    pub active: bool,
}

// ─── Usage Billing ───────────────────────────────────────────────────

/// Usage record for billing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Tenant/customer identifier.
    pub tenant_id: String,
    /// Billing period (YYYY-MM).
    pub period: String,
    /// Number of analyses performed.
    pub analysis_count: u64,
    /// Number of documents processed.
    pub document_count: u64,
    /// Total bytes processed.
    pub bytes_processed: u64,
    /// Number of certificate generations.
    pub certificates_generated: u64,
    /// Number of batch submissions.
    pub batch_count: u64,
    /// API calls made.
    pub api_calls: u64,
    /// Per-feature usage.
    pub feature_usage: HashMap<String, u64>,
}

/// Usage tracker (in-memory, replace with database in production).
#[derive(Debug, Clone)]
pub struct UsageTracker {
    records: Arc<Mutex<HashMap<String, UsageRecord>>>,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record an API call for a tenant.
    pub fn record_api_call(&self, tenant_id: &str) {
        let period = current_period();
        let key = format!("{tenant_id}:{period}");
        let mut records = self.records.lock().unwrap();
        let record = records.entry(key).or_insert_with(|| UsageRecord {
            tenant_id: tenant_id.to_string(),
            period: period.clone(),
            analysis_count: 0,
            document_count: 0,
            bytes_processed: 0,
            certificates_generated: 0,
            batch_count: 0,
            api_calls: 0,
            feature_usage: HashMap::new(),
        });
        record.api_calls += 1;
    }

    /// Record a document analysis for a tenant.
    pub fn record_analysis(&self, tenant_id: &str, bytes: u64) {
        let period = current_period();
        let key = format!("{tenant_id}:{period}");
        let mut records = self.records.lock().unwrap();
        let record = records.entry(key).or_insert_with(|| UsageRecord {
            tenant_id: tenant_id.to_string(),
            period: period.clone(),
            analysis_count: 0,
            document_count: 0,
            bytes_processed: 0,
            certificates_generated: 0,
            batch_count: 0,
            api_calls: 0,
            feature_usage: HashMap::new(),
        });
        record.analysis_count += 1;
        record.document_count += 1;
        record.bytes_processed += bytes;
    }

    /// Get usage for a tenant in the current period.
    pub fn get_current(&self, tenant_id: &str) -> Option<UsageRecord> {
        let period = current_period();
        let key = format!("{tenant_id}:{period}");
        let records = self.records.lock().unwrap();
        records.get(&key).cloned()
    }

    /// Check if a tenant is within their quota.
    pub fn check_quota(&self, tenant_id: &str, max_analyses: u64) -> bool {
        match self.get_current(tenant_id) {
            Some(record) => record.analysis_count < max_analyses,
            None => true,
        }
    }
}

fn current_period() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ts = crate::scoring::engine::format_epoch_public(secs);
    // Extract YYYY-MM from ISO 8601
    ts[..7].to_string()
}

/// Tenant configuration for multi-tenant platform deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Tenant identifier.
    pub tenant_id: String,
    /// Display name.
    pub name: String,
    /// API key(s) for this tenant.
    pub api_keys: Vec<String>,
    /// Monthly analysis quota (0 = unlimited).
    pub monthly_quota: u64,
    /// Billing plan.
    pub plan: BillingPlan,
    /// Webhook endpoints.
    pub webhooks: Vec<WebhookEndpoint>,
    /// Custom analysis defaults.
    pub analysis_defaults: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingPlan {
    Free,
    Starter,
    Professional,
    Enterprise,
    Custom,
}

impl BillingPlan {
    pub fn default_quota(&self) -> u64 {
        match self {
            BillingPlan::Free => 50,
            BillingPlan::Starter => 500,
            BillingPlan::Professional => 5000,
            BillingPlan::Enterprise => 50000,
            BillingPlan::Custom => 0, // Configured per tenant
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_progress() {
        let batch = BatchSubmission {
            batch_id: "batch-001".into(),
            tenant_id: "tenant-1".into(),
            entries: vec![
                BatchEntry {
                    external_id: "doc1".into(),
                    file_name: "a.docx".into(),
                    job_id: None,
                    status: BatchEntryStatus::Completed,
                    result_summary: None,
                },
                BatchEntry {
                    external_id: "doc2".into(),
                    file_name: "b.docx".into(),
                    job_id: None,
                    status: BatchEntryStatus::Completed,
                    result_summary: None,
                },
                BatchEntry {
                    external_id: "doc3".into(),
                    file_name: "c.docx".into(),
                    job_id: None,
                    status: BatchEntryStatus::Pending,
                    result_summary: None,
                },
            ],
            submitted_at: "2026-03-22T10:00:00Z".into(),
            webhook_url: None,
            options: BatchOptions::default(),
        };

        let progress = BatchProgress::from_submission(&batch);
        assert_eq!(progress.total, 3);
        assert_eq!(progress.completed, 2);
        assert_eq!(progress.pending, 1);
        assert!((progress.progress_pct - 66.7).abs() < 1.0);
    }

    #[test]
    fn test_usage_tracker() {
        let tracker = UsageTracker::new();
        tracker.record_api_call("tenant-1");
        tracker.record_api_call("tenant-1");
        tracker.record_analysis("tenant-1", 50000);

        let usage = tracker.get_current("tenant-1").unwrap();
        assert_eq!(usage.api_calls, 2);
        assert_eq!(usage.analysis_count, 1);
        assert_eq!(usage.bytes_processed, 50000);
    }

    #[test]
    fn test_quota_check() {
        let tracker = UsageTracker::new();
        assert!(tracker.check_quota("tenant-1", 100));

        for _ in 0..5 {
            tracker.record_analysis("tenant-1", 1000);
        }
        assert!(tracker.check_quota("tenant-1", 100)); // 5 < 100
        assert!(!tracker.check_quota("tenant-1", 5)); // 5 >= 5
    }

    #[test]
    fn test_billing_plan_quotas() {
        assert_eq!(BillingPlan::Free.default_quota(), 50);
        assert_eq!(BillingPlan::Enterprise.default_quota(), 50000);
    }

    #[test]
    fn test_batch_options_default() {
        let opts = BatchOptions::default();
        assert_eq!(opts.concurrency, 4);
        assert!(opts.continue_on_error);
        assert_eq!(opts.export_format, ExportFormat::JsonArray);
    }
}
