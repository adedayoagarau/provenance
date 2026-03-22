//! Publishing system connectors — Editorial Manager, ManuscriptManager, CMS webhooks.
//!
//! Integrates Provenance into scholarly publishing and content management workflows.
//! Publishers can submit manuscripts for analysis via webhooks and receive results
//! through callbacks or polling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported publishing systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PublishingSystem {
    /// Aries Systems Editorial Manager (journals).
    EditorialManager,
    /// ScholarOne Manuscripts (Clarivate).
    ScholarOne,
    /// Open Journal Systems (PKP).
    OJS,
    /// ManuscriptManager.
    ManuscriptManager,
    /// WordPress (with Provenance plugin).
    WordPress,
    /// Custom CMS via webhook.
    CustomWebhook,
}

impl PublishingSystem {
    pub fn label(&self) -> &'static str {
        match self {
            PublishingSystem::EditorialManager => "Editorial Manager (Aries)",
            PublishingSystem::ScholarOne => "ScholarOne Manuscripts",
            PublishingSystem::OJS => "Open Journal Systems",
            PublishingSystem::ManuscriptManager => "ManuscriptManager",
            PublishingSystem::WordPress => "WordPress",
            PublishingSystem::CustomWebhook => "Custom Webhook",
        }
    }
}

/// Webhook configuration for a publishing system integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Unique integration identifier.
    pub integration_id: String,
    /// Publishing system type.
    pub system: PublishingSystem,
    /// Display name for this integration.
    pub name: String,
    /// Webhook secret for HMAC signature verification.
    pub webhook_secret: String,
    /// URL to POST results back to.
    pub callback_url: String,
    /// HTTP headers to include in callbacks.
    pub callback_headers: HashMap<String, String>,
    /// Events that trigger analysis.
    pub trigger_events: Vec<PublishingEvent>,
    /// Analysis options.
    pub analysis_options: PublishingAnalysisOptions,
}

/// Events from publishing systems that can trigger analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublishingEvent {
    /// New manuscript submitted.
    ManuscriptSubmitted,
    /// Revised manuscript submitted.
    RevisionSubmitted,
    /// Manuscript assigned to reviewer.
    AssignedToReviewer,
    /// Article published (post-publication check).
    ArticlePublished,
    /// Manual analysis request by editor.
    EditorRequest,
}

/// Analysis options specific to publishing workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishingAnalysisOptions {
    /// Run full forensic analysis.
    pub enable_forensics: bool,
    /// Run humanizer detection.
    pub enable_humanizer_detection: bool,
    /// Compare against author's previous submissions (if profile exists).
    pub enable_author_comparison: bool,
    /// Include authorship confidence score in results.
    pub include_acs: bool,
    /// Result format for callbacks.
    pub result_format: ResultFormat,
    /// Priority level (affects queue position).
    pub priority: SubmissionPriority,
}

impl Default for PublishingAnalysisOptions {
    fn default() -> Self {
        Self {
            enable_forensics: true,
            enable_humanizer_detection: true,
            enable_author_comparison: false,
            include_acs: true,
            result_format: ResultFormat::Json,
            priority: SubmissionPriority::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultFormat {
    Json,
    Pdf,
    Html,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Inbound webhook payload from a publishing system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Event type.
    pub event: PublishingEvent,
    /// Manuscript/article identifier in the publishing system.
    pub manuscript_id: String,
    /// Title of the work.
    pub title: Option<String>,
    /// Author identifier(s).
    pub author_ids: Vec<String>,
    /// Author name(s).
    pub author_names: Vec<String>,
    /// Journal/publication identifier.
    pub journal_id: Option<String>,
    /// File to analyze (URL or base64-encoded content).
    pub file_url: Option<String>,
    pub file_content_base64: Option<String>,
    pub file_name: Option<String>,
    /// Custom metadata from the publishing system.
    pub metadata: HashMap<String, String>,
}

/// Result callback payload sent back to the publishing system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackPayload {
    /// Original manuscript ID.
    pub manuscript_id: String,
    /// Provenance analysis job ID.
    pub job_id: String,
    /// Analysis status.
    pub status: CallbackStatus,
    /// ACS score (if available).
    pub acs_score: Option<f64>,
    /// ACS confidence interval.
    pub acs_margin: Option<f64>,
    /// Process Integrity Index.
    pub pii_score: Option<f64>,
    /// Number of anomaly flags.
    pub anomaly_count: Option<usize>,
    /// Humanizer detection result.
    pub humanizer_detected: Option<bool>,
    /// Link to full report (if hosted).
    pub report_url: Option<String>,
    /// Full result JSON (if requested).
    pub full_result: Option<serde_json::Value>,
    /// Timestamp.
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallbackStatus {
    Completed,
    Failed,
    InsufficientData,
}

/// Verify a webhook signature (HMAC-SHA256).
pub fn verify_webhook_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    use sha2::{Digest, Sha256};

    // Compute HMAC-SHA256(secret, payload)
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(payload);
    let computed = format!("sha256={:x}", hasher.finalize());

    // Constant-time comparison
    if computed.len() != signature.len() {
        return false;
    }
    computed
        .bytes()
        .zip(signature.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Validate an inbound webhook payload.
pub fn validate_payload(payload: &WebhookPayload) -> Result<(), String> {
    if payload.manuscript_id.is_empty() {
        return Err("manuscript_id is required".into());
    }

    if payload.file_url.is_none() && payload.file_content_base64.is_none() {
        return Err("Either file_url or file_content_base64 is required".into());
    }

    if payload.author_ids.is_empty() && payload.author_names.is_empty() {
        return Err("At least one author identifier or name is required".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publishing_system_labels() {
        assert_eq!(
            PublishingSystem::EditorialManager.label(),
            "Editorial Manager (Aries)"
        );
        assert_eq!(PublishingSystem::OJS.label(), "Open Journal Systems");
    }

    #[test]
    fn test_validate_payload_valid() {
        let payload = WebhookPayload {
            event: PublishingEvent::ManuscriptSubmitted,
            manuscript_id: "MS-2026-001".into(),
            title: Some("A Novel Approach".into()),
            author_ids: vec!["auth-123".into()],
            author_names: vec!["Jane Smith".into()],
            journal_id: Some("JNL-001".into()),
            file_url: Some("https://example.com/manuscript.docx".into()),
            file_content_base64: None,
            file_name: Some("manuscript.docx".into()),
            metadata: HashMap::new(),
        };
        assert!(validate_payload(&payload).is_ok());
    }

    #[test]
    fn test_validate_payload_missing_file() {
        let payload = WebhookPayload {
            event: PublishingEvent::ManuscriptSubmitted,
            manuscript_id: "MS-001".into(),
            title: None,
            author_ids: vec!["auth-1".into()],
            author_names: Vec::new(),
            journal_id: None,
            file_url: None,
            file_content_base64: None,
            file_name: None,
            metadata: HashMap::new(),
        };
        assert!(validate_payload(&payload).is_err());
    }

    #[test]
    fn test_validate_payload_missing_authors() {
        let payload = WebhookPayload {
            event: PublishingEvent::ManuscriptSubmitted,
            manuscript_id: "MS-001".into(),
            title: None,
            author_ids: Vec::new(),
            author_names: Vec::new(),
            journal_id: None,
            file_url: Some("https://example.com/doc.docx".into()),
            file_content_base64: None,
            file_name: None,
            metadata: HashMap::new(),
        };
        assert!(validate_payload(&payload).is_err());
    }

    #[test]
    fn test_webhook_signature_verification() {
        let payload = b"test payload";
        let secret = "my-secret";

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(payload);
        let expected = format!("sha256={:x}", hasher.finalize());

        assert!(verify_webhook_signature(payload, &expected, secret));
        assert!(!verify_webhook_signature(payload, "sha256=invalid", secret));
    }

    #[test]
    fn test_callback_serialization() {
        let callback = CallbackPayload {
            manuscript_id: "MS-001".into(),
            job_id: "job-abc".into(),
            status: CallbackStatus::Completed,
            acs_score: Some(72.5),
            acs_margin: Some(12.0),
            pii_score: Some(65.0),
            anomaly_count: Some(2),
            humanizer_detected: Some(false),
            report_url: None,
            full_result: None,
            timestamp: "2026-03-22T10:00:00Z".into(),
        };

        let json = serde_json::to_string(&callback).unwrap();
        let recovered: CallbackPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.acs_score, Some(72.5));
    }

    #[test]
    fn test_default_analysis_options() {
        let opts = PublishingAnalysisOptions::default();
        assert!(opts.enable_forensics);
        assert!(opts.enable_humanizer_detection);
        assert!(!opts.enable_author_comparison);
        assert_eq!(opts.priority, SubmissionPriority::Normal);
    }
}
