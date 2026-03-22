//! LMS Integration — LTI 1.3 protocol, Canvas, Blackboard, Moodle, Brightspace adapters.
//!
//! Implements the IMS Global LTI 1.3 / LTI Advantage protocol for deep integration
//! with learning management systems. Supports assignment submission hooks, grade
//! passback, and roster access for institution-wide deployment.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── LTI 1.3 Core Protocol ──────────────────────────────────────────

/// LTI 1.3 platform registration (one per LMS instance).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtiPlatformRegistration {
    /// Unique platform identifier (issuer URL).
    pub issuer: String,
    /// Platform's OAuth2 authorization endpoint.
    pub auth_endpoint: String,
    /// Platform's OAuth2 token endpoint.
    pub token_endpoint: String,
    /// Platform's JWKS endpoint for verifying ID tokens.
    pub jwks_uri: String,
    /// Client ID assigned to Provenance by the platform.
    pub client_id: String,
    /// Deployment ID for this specific integration.
    pub deployment_id: String,
    /// LMS type for adapter selection.
    pub lms_type: LmsType,
    /// Human-readable institution name.
    pub institution_name: String,
}

/// Supported LMS platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LmsType {
    Canvas,
    Blackboard,
    Moodle,
    Brightspace,
    Sakai,
    Generic,
}

impl LmsType {
    pub fn label(&self) -> &'static str {
        match self {
            LmsType::Canvas => "Canvas (Instructure)",
            LmsType::Blackboard => "Blackboard Learn",
            LmsType::Moodle => "Moodle",
            LmsType::Brightspace => "D2L Brightspace",
            LmsType::Sakai => "Sakai",
            LmsType::Generic => "Generic LTI 1.3",
        }
    }

    /// Whether this LMS supports LTI Assignment and Grade Services.
    pub fn supports_ags(&self) -> bool {
        matches!(self, LmsType::Canvas | LmsType::Blackboard | LmsType::Brightspace | LmsType::Moodle)
    }

    /// Whether this LMS supports LTI Names and Role Provisioning.
    pub fn supports_nrps(&self) -> bool {
        matches!(self, LmsType::Canvas | LmsType::Blackboard | LmsType::Brightspace)
    }
}

/// LTI 1.3 launch request (decoded from JWT ID token).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtiLaunchRequest {
    /// Subject identifier (user ID in the platform).
    pub sub: String,
    /// Issuer (matches platform registration).
    pub iss: String,
    /// Audience (our client ID).
    pub aud: String,
    /// LTI message type.
    pub message_type: LtiMessageType,
    /// LTI version (must be "1.3.0").
    pub version: String,
    /// Deployment ID.
    pub deployment_id: String,
    /// Target link URI (where the user should land).
    pub target_link_uri: String,
    /// Resource link (assignment/activity info).
    pub resource_link: Option<ResourceLink>,
    /// User's roles in the course.
    pub roles: Vec<String>,
    /// Context (course info).
    pub context: Option<LtiContext>,
    /// Custom parameters set by the instructor.
    pub custom: HashMap<String, String>,
    /// Assignment and Grade Services claim.
    pub ags: Option<AgsServiceClaim>,
    /// Names and Role Provisioning claim.
    pub nrps: Option<NrpsServiceClaim>,
}

/// LTI message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LtiMessageType {
    /// Student launching the tool from an assignment.
    LtiResourceLinkRequest,
    /// Deep linking (instructor adding the tool to a course).
    LtiDeepLinkingRequest,
    /// Submission review (instructor reviewing a submission).
    LtiSubmissionReviewRequest,
}

/// Resource link — the specific assignment or activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLink {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Course/context information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtiContext {
    pub id: String,
    pub label: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub context_type: Option<Vec<String>>,
}

/// Assignment and Grade Services claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgsServiceClaim {
    /// Scope: can create/read lineitems.
    pub scope: Vec<String>,
    /// Lineitems URL for grade passback.
    pub lineitems: String,
    /// Specific lineitem URL (if bound to an assignment).
    pub lineitem: Option<String>,
}

/// Names and Role Provisioning claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NrpsServiceClaim {
    /// Membership URL.
    pub context_memberships_url: String,
    /// Supported service versions.
    pub service_versions: Vec<String>,
}

// ─── LTI Roles ───────────────────────────────────────────────────────

/// Parse LTI roles into Provenance role categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvenanceRole {
    /// Student/learner — submitter of work.
    Student,
    /// Instructor — evaluator of work.
    Instructor,
    /// Teaching assistant — limited evaluator role.
    TeachingAssistant,
    /// Administrator — institutional configuration.
    Admin,
}

pub fn parse_role(lti_roles: &[String]) -> ProvenanceRole {
    for role in lti_roles {
        if role.contains("Instructor") || role.contains("Faculty") {
            return ProvenanceRole::Instructor;
        }
        if role.contains("TeachingAssistant") || role.contains("Mentor") {
            return ProvenanceRole::TeachingAssistant;
        }
        if role.contains("Administrator") || role.contains("SysAdmin") {
            return ProvenanceRole::Admin;
        }
    }
    ProvenanceRole::Student
}

// ─── Grade Passback ──────────────────────────────────────────────────

/// A grade/score to pass back to the LMS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradePassback {
    /// User ID (LTI sub claim).
    pub user_id: String,
    /// Activity progress.
    pub activity_progress: ActivityProgress,
    /// Grading progress.
    pub grading_progress: GradingProgress,
    /// Score (0.0-1.0, optional — may not assign a grade).
    pub score_given: Option<f64>,
    /// Maximum possible score.
    pub score_maximum: Option<f64>,
    /// Comment to display to the student.
    pub comment: Option<String>,
    /// Timestamp of the score.
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityProgress {
    Initialized,
    Started,
    InProgress,
    Submitted,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradingProgress {
    NotReady,
    Failed,
    Pending,
    PendingManual,
    FullyGraded,
}

// ─── Submission Flow ─────────────────────────────────────────────────

/// A student submission received via LTI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtiSubmission {
    /// Submission ID (generated by Provenance).
    pub submission_id: String,
    /// LTI user ID.
    pub user_id: String,
    /// Course context ID.
    pub context_id: String,
    /// Resource link ID (assignment).
    pub resource_link_id: String,
    /// Institution (issuer).
    pub institution: String,
    /// LMS type.
    pub lms_type: LmsType,
    /// Submitted file name.
    pub file_name: String,
    /// Submission timestamp.
    pub submitted_at: String,
    /// Analysis job ID (once submitted for analysis).
    pub analysis_job_id: Option<String>,
    /// Current status.
    pub status: SubmissionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionStatus {
    /// Received but not yet analyzed.
    Received,
    /// Analysis in progress.
    Analyzing,
    /// Analysis complete — ready for instructor review.
    Ready,
    /// Instructor has reviewed.
    Reviewed,
    /// Error during analysis.
    Error,
}

// ─── LMS-Specific Adapters ──────────────────────────────────────────

/// Canvas-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    pub base_url: String,
    pub developer_key_id: String,
    /// Whether to use Canvas's SpeedGrader integration.
    pub enable_speedgrader: bool,
    /// Whether to auto-submit analysis when assignment is submitted.
    pub auto_analyze_on_submit: bool,
}

/// Blackboard-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardConfig {
    pub base_url: String,
    pub application_id: String,
    /// Whether to integrate with Blackboard's SafeAssign.
    pub complement_safe_assign: bool,
}

/// Moodle-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodleConfig {
    pub base_url: String,
    /// Moodle plugin type.
    pub plugin_type: MoodlePluginType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoodlePluginType {
    /// External tool (LTI).
    ExternalTool,
    /// Native Moodle plugin (mod_provenance).
    NativePlugin,
}

/// Brightspace-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightspaceConfig {
    pub base_url: String,
    pub org_id: String,
    /// Whether to use Brightspace's TurnItIn replacement workflow.
    pub replace_turnitin: bool,
}

// ─── Institution Configuration ───────────────────────────────────────

/// Complete configuration for an institution's LMS integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionConfig {
    /// Institution identifier.
    pub institution_id: String,
    /// Display name.
    pub name: String,
    /// LTI platform registration.
    pub platform: LtiPlatformRegistration,
    /// FERPA compliance settings.
    pub ferpa: FerpaConfig,
    /// Analysis defaults for this institution.
    pub analysis_defaults: InstitutionAnalysisDefaults,
}

/// FERPA compliance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerpaConfig {
    /// Whether student data is stored (must be true for operation).
    pub data_storage_consent: bool,
    /// Data retention period in days (0 = delete after analysis).
    pub retention_days: u32,
    /// Whether to anonymize student identifiers in analysis results.
    pub anonymize_students: bool,
    /// Data sharing agreement reference.
    pub data_sharing_agreement_id: Option<String>,
    /// Whether instructor can see student names with results.
    pub instructor_sees_names: bool,
}

/// Default analysis settings per institution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionAnalysisDefaults {
    /// Whether to run DOCX forensics by default.
    pub enable_docx_forensics: bool,
    /// Whether to run humanizer detection.
    pub enable_humanizer_detection: bool,
    /// Whether to include process capture data when available.
    pub use_process_capture: bool,
    /// Custom report template.
    pub report_template: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lms_type_labels() {
        assert_eq!(LmsType::Canvas.label(), "Canvas (Instructure)");
        assert_eq!(LmsType::Moodle.label(), "Moodle");
        assert!(LmsType::Canvas.supports_ags());
        assert!(LmsType::Canvas.supports_nrps());
        assert!(!LmsType::Sakai.supports_nrps());
    }

    #[test]
    fn test_parse_role_instructor() {
        let roles = vec![
            "http://purl.imsglobal.org/vocab/lis/v2/membership#Instructor".into(),
        ];
        assert_eq!(parse_role(&roles), ProvenanceRole::Instructor);
    }

    #[test]
    fn test_parse_role_student() {
        let roles = vec![
            "http://purl.imsglobal.org/vocab/lis/v2/membership#Learner".into(),
        ];
        assert_eq!(parse_role(&roles), ProvenanceRole::Student);
    }

    #[test]
    fn test_parse_role_admin() {
        let roles = vec![
            "http://purl.imsglobal.org/vocab/lis/v2/institution/person#Administrator".into(),
        ];
        assert_eq!(parse_role(&roles), ProvenanceRole::Admin);
    }

    #[test]
    fn test_parse_role_ta() {
        let roles = vec![
            "http://purl.imsglobal.org/vocab/lis/v2/membership#TeachingAssistant".into(),
        ];
        assert_eq!(parse_role(&roles), ProvenanceRole::TeachingAssistant);
    }

    #[test]
    fn test_submission_serialization() {
        let sub = LtiSubmission {
            submission_id: "sub-001".into(),
            user_id: "user-123".into(),
            context_id: "course-456".into(),
            resource_link_id: "assign-789".into(),
            institution: "https://canvas.university.edu".into(),
            lms_type: LmsType::Canvas,
            file_name: "essay.docx".into(),
            submitted_at: "2026-03-22T10:00:00Z".into(),
            analysis_job_id: None,
            status: SubmissionStatus::Received,
        };

        let json = serde_json::to_string(&sub).unwrap();
        let recovered: LtiSubmission = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.submission_id, "sub-001");
        assert_eq!(recovered.lms_type, LmsType::Canvas);
    }

    #[test]
    fn test_ferpa_config() {
        let config = FerpaConfig {
            data_storage_consent: true,
            retention_days: 90,
            anonymize_students: false,
            data_sharing_agreement_id: Some("DSA-2026-001".into()),
            instructor_sees_names: true,
        };
        assert!(config.data_storage_consent);
        assert_eq!(config.retention_days, 90);
    }

    #[test]
    fn test_grade_passback_serialization() {
        let grade = GradePassback {
            user_id: "user-123".into(),
            activity_progress: ActivityProgress::Completed,
            grading_progress: GradingProgress::FullyGraded,
            score_given: None, // Provenance doesn't assign grades
            score_maximum: None,
            comment: Some("Analysis complete. View detailed report in Provenance.".into()),
            timestamp: "2026-03-22T10:30:00Z".into(),
        };

        let json = serde_json::to_string(&grade).unwrap();
        assert!(json.contains("Completed"));
        assert!(json.contains("FullyGraded"));
    }
}
