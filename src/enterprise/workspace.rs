//! Google Workspace integration — institution-wide OAuth, Drive API, FERPA-compliant.
//!
//! Enables institutions using Google Workspace for Education to integrate Provenance
//! across their Google Docs workflow. Supports domain-wide delegation for admin-level
//! access, individual OAuth for student consent, and FERPA-compliant data handling.

use serde::{Deserialize, Serialize};

/// Google Workspace integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Google Cloud project ID.
    pub project_id: String,
    /// OAuth2 client ID.
    pub client_id: String,
    /// OAuth2 client secret (stored encrypted in production).
    pub client_secret_encrypted: String,
    /// Google Workspace domain.
    pub domain: String,
    /// Institution name.
    pub institution_name: String,
    /// Authorization mode.
    pub auth_mode: WorkspaceAuthMode,
    /// FERPA compliance settings.
    pub ferpa: WorkspaceFerpaConfig,
    /// Scopes requested.
    pub scopes: Vec<GoogleScope>,
}

/// Authorization modes for Google Workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceAuthMode {
    /// Domain-wide delegation — admin consents for all users.
    /// Requires Google Workspace admin and service account.
    DomainWide,
    /// Individual consent — each user authorizes separately.
    /// More privacy-friendly, less admin overhead.
    IndividualConsent,
    /// Hybrid — domain-wide for metadata, individual for content.
    Hybrid,
}

/// Google OAuth2 scopes needed for integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoogleScope {
    /// Read Google Docs content.
    DocsReadonly,
    /// Read Google Drive file metadata.
    DriveMetadataReadonly,
    /// Read Google Drive files.
    DriveReadonly,
    /// Access Google Classroom courses and submissions.
    ClassroomCoursesReadonly,
    /// Access Google Classroom student submissions.
    ClassroomStudentSubmissionsReadonly,
    /// User profile information.
    UserInfoProfile,
    /// User email.
    UserInfoEmail,
}

impl GoogleScope {
    pub fn scope_url(&self) -> &'static str {
        match self {
            GoogleScope::DocsReadonly => "https://www.googleapis.com/auth/documents.readonly",
            GoogleScope::DriveMetadataReadonly => {
                "https://www.googleapis.com/auth/drive.metadata.readonly"
            }
            GoogleScope::DriveReadonly => "https://www.googleapis.com/auth/drive.readonly",
            GoogleScope::ClassroomCoursesReadonly => {
                "https://www.googleapis.com/auth/classroom.courses.readonly"
            }
            GoogleScope::ClassroomStudentSubmissionsReadonly => {
                "https://www.googleapis.com/auth/classroom.student-submissions.students.readonly"
            }
            GoogleScope::UserInfoProfile => "https://www.googleapis.com/auth/userinfo.profile",
            GoogleScope::UserInfoEmail => "https://www.googleapis.com/auth/userinfo.email",
        }
    }
}

/// FERPA-specific configuration for Google Workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFerpaConfig {
    /// Data processing agreement with institution.
    pub dpa_signed: bool,
    /// Data processing agreement reference.
    pub dpa_reference: Option<String>,
    /// Whether student PII is stored.
    pub stores_student_pii: bool,
    /// If PII stored, retention period in days.
    pub pii_retention_days: Option<u32>,
    /// Whether content is processed in-memory only (no disk persistence).
    pub memory_only_processing: bool,
    /// Data residency requirement.
    pub data_residency: Option<DataResidency>,
    /// Audit log enabled.
    pub audit_logging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataResidency {
    US,
    EU,
    Canada,
    Australia,
    Any,
}

/// A Google Classroom course linked to Provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedCourse {
    /// Google Classroom course ID.
    pub course_id: String,
    /// Course name.
    pub name: String,
    /// Instructor email.
    pub instructor_email: String,
    /// Whether auto-analysis is enabled for submissions.
    pub auto_analyze: bool,
    /// Assignments configured for Provenance analysis.
    pub linked_assignments: Vec<LinkedAssignment>,
}

/// A Google Classroom assignment linked to Provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedAssignment {
    /// Coursework ID in Google Classroom.
    pub coursework_id: String,
    /// Assignment title.
    pub title: String,
    /// Whether to analyze all submissions or only flagged ones.
    pub analyze_all: bool,
    /// Author profile to compare against (optional).
    pub comparison_profile: Option<String>,
}

/// OAuth2 token for a user session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Access token.
    pub access_token: String,
    /// Refresh token (for long-lived sessions).
    pub refresh_token: Option<String>,
    /// Token expiry (epoch seconds).
    pub expires_at: u64,
    /// Scopes granted.
    pub scopes: Vec<String>,
    /// User identifier.
    pub user_email: String,
}

impl OAuthToken {
    /// Check if the token is expired.
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }

    /// Check if the token has a specific scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Submission from Google Classroom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassroomSubmission {
    /// Submission ID.
    pub submission_id: String,
    /// Student email (may be anonymized per FERPA config).
    pub student_identifier: String,
    /// Course ID.
    pub course_id: String,
    /// Coursework ID.
    pub coursework_id: String,
    /// Google Drive file IDs attached to the submission.
    pub drive_file_ids: Vec<String>,
    /// Submission state in Classroom.
    pub state: ClassroomSubmissionState,
    /// When the student submitted.
    pub submitted_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassroomSubmissionState {
    New,
    Created,
    TurnedIn,
    Returned,
    ReclaimedByStudent,
}

/// Result of processing a Classroom submission through Provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassroomAnalysisResult {
    /// Original submission ID.
    pub submission_id: String,
    /// Provenance job ID.
    pub job_id: String,
    /// Whether analysis completed successfully.
    pub success: bool,
    /// ACS score (if available).
    pub acs_score: Option<f64>,
    /// Summary for instructor view.
    pub instructor_summary: Option<String>,
    /// Summary for student view (more protective framing).
    pub student_summary: Option<String>,
    /// Link to full report.
    pub report_url: Option<String>,
}

/// Build the default scopes for Provenance Classroom integration.
pub fn default_classroom_scopes() -> Vec<GoogleScope> {
    vec![
        GoogleScope::DocsReadonly,
        GoogleScope::DriveReadonly,
        GoogleScope::ClassroomCoursesReadonly,
        GoogleScope::ClassroomStudentSubmissionsReadonly,
        GoogleScope::UserInfoEmail,
    ]
}

/// Build the minimal scopes for individual user consent.
pub fn minimal_user_scopes() -> Vec<GoogleScope> {
    vec![
        GoogleScope::DocsReadonly,
        GoogleScope::DriveMetadataReadonly,
        GoogleScope::UserInfoEmail,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_urls() {
        assert!(GoogleScope::DocsReadonly
            .scope_url()
            .contains("documents.readonly"));
        assert!(GoogleScope::ClassroomCoursesReadonly
            .scope_url()
            .contains("classroom.courses"));
    }

    #[test]
    fn test_default_classroom_scopes() {
        let scopes = default_classroom_scopes();
        assert!(scopes.len() >= 4);
        assert!(scopes.contains(&GoogleScope::DocsReadonly));
        assert!(scopes.contains(&GoogleScope::ClassroomStudentSubmissionsReadonly));
    }

    #[test]
    fn test_oauth_token_expiry() {
        let expired = OAuthToken {
            access_token: "tok".into(),
            refresh_token: None,
            expires_at: 0, // Epoch = always expired
            scopes: vec!["scope1".into()],
            user_email: "user@example.com".into(),
        };
        assert!(expired.is_expired());

        let valid = OAuthToken {
            access_token: "tok".into(),
            refresh_token: None,
            expires_at: u64::MAX,
            scopes: vec!["scope1".into()],
            user_email: "user@example.com".into(),
        };
        assert!(!valid.is_expired());
        assert!(valid.has_scope("scope1"));
        assert!(!valid.has_scope("scope2"));
    }

    #[test]
    fn test_workspace_auth_modes() {
        assert_eq!(
            serde_json::to_string(&WorkspaceAuthMode::DomainWide).unwrap(),
            "\"DomainWide\""
        );
        assert_eq!(
            serde_json::to_string(&WorkspaceAuthMode::IndividualConsent).unwrap(),
            "\"IndividualConsent\""
        );
    }

    #[test]
    fn test_classroom_submission_serialization() {
        let sub = ClassroomSubmission {
            submission_id: "sub-001".into(),
            student_identifier: "student@school.edu".into(),
            course_id: "course-123".into(),
            coursework_id: "cw-456".into(),
            drive_file_ids: vec!["file-abc".into()],
            state: ClassroomSubmissionState::TurnedIn,
            submitted_at: Some("2026-03-22T10:00:00Z".into()),
        };

        let json = serde_json::to_string(&sub).unwrap();
        let recovered: ClassroomSubmission = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.state, ClassroomSubmissionState::TurnedIn);
    }

    #[test]
    fn test_ferpa_config() {
        let config = WorkspaceFerpaConfig {
            dpa_signed: true,
            dpa_reference: Some("DPA-2026-EDU-001".into()),
            stores_student_pii: false,
            pii_retention_days: None,
            memory_only_processing: true,
            data_residency: Some(DataResidency::US),
            audit_logging: true,
        };
        assert!(config.dpa_signed);
        assert!(config.memory_only_processing);
    }
}
