//! API request handlers for all Provenance endpoints.

#[cfg(feature = "server")]
use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "server")]
use super::errors::ApiError;
#[cfg(feature = "server")]
use super::server::AppState;

// ─── Request/Response Types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    /// Base64-encoded file content.
    pub file_content: Option<String>,
    /// Plain text to analyze directly.
    pub text: Option<String>,
    /// File name (for format detection).
    pub file_name: Option<String>,
    /// Profile path for comparison (optional).
    pub profile_path: Option<String>,
    /// Output format: json (default), text, html.
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub status: String,
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JobSubmitResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct FormatQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CertificateGenerateRequest {
    /// Path to file to certify.
    pub file_path: String,
    /// Path to signing key.
    pub key_path: String,
    /// Path to author profile (optional).
    pub profile_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CertificateVerifyRequest {
    /// Path to certificate file.
    pub certificate_path: String,
    /// Path to original document (optional).
    pub document_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileBuildRequest {
    /// Directory containing writing samples.
    pub samples_dir: String,
    /// Author name.
    pub author_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ProfileCompareRequest {
    /// Path to document file.
    pub file_path: String,
    /// Path to author profile.
    pub profile_path: String,
}

#[derive(Debug, Deserialize)]
pub struct HumanizerRequest {
    /// Plain text to analyze.
    pub text: Option<String>,
    /// Path to file to analyze.
    pub file_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

// ─── Handlers ────────────────────────────────────────────────────────

/// GET /health — Health check endpoint.
#[cfg(feature = "server")]
pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: uptime,
    })
}

/// POST /analyze — Analyze a document for authorship verification.
#[cfg(feature = "server")]
pub async fn analyze(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Write text to temp file if provided directly
    let file_path = if let Some(ref text) = req.text {
        let temp_path = write_temp_file(text.as_bytes(), "analysis.txt")?;
        temp_path
    } else if let Some(ref b64) = req.file_content {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| ApiError::bad_request(format!("Invalid base64: {e}")))?;
        let name = req.file_name.as_deref().unwrap_or("document.txt");
        write_temp_file(&bytes, name)?
    } else {
        return Err(ApiError::bad_request(
            "Either 'text' or 'file_content' is required",
        ));
    };

    let format = crate::scoring::engine::OutputFormat::Json;
    let result_str = crate::analyze_with_format(
        &file_path,
        req.profile_path.as_deref(),
        format,
    )
    .map_err(ApiError::from)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&file_path);

    let result: serde_json::Value = serde_json::from_str(&result_str)
        .unwrap_or_else(|_| serde_json::json!({"raw": result_str}));

    Ok(Json(result))
}

/// POST /analyze/async — Submit analysis as async job.
#[cfg(feature = "server")]
pub async fn analyze_async(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<JobSubmitResponse>, ApiError> {
    let job_id = state.job_store.create("analyze");

    let store = state.job_store.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        store.mark_running(&job_id_clone);

        let file_path = if let Some(ref text) = req.text {
            match write_temp_file(text.as_bytes(), "analysis.txt") {
                Ok(p) => p,
                Err(e) => {
                    store.fail(&job_id_clone, e.error);
                    return;
                }
            }
        } else {
            store.fail(&job_id_clone, "No input provided".into());
            return;
        };

        let format = crate::scoring::engine::OutputFormat::Json;
        match crate::analyze_with_format(&file_path, req.profile_path.as_deref(), format) {
            Ok(result) => {
                let _ = std::fs::remove_file(&file_path);
                store.complete(&job_id_clone, result);
            }
            Err(e) => {
                let _ = std::fs::remove_file(&file_path);
                store.fail(&job_id_clone, e.to_string());
            }
        }
    });

    Ok(Json(JobSubmitResponse {
        job_id,
        status: "pending".into(),
        message: "Analysis job submitted. Poll GET /jobs/{id} for status.".into(),
    }))
}

/// GET /jobs/:id — Get job status and result.
#[cfg(feature = "server")]
pub async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<super::jobs::Job>, ApiError> {
    state
        .job_store
        .get(&job_id)
        .map(Json)
        .ok_or_else(|| ApiError {
            error: format!("Job '{job_id}' not found"),
            code: "job_not_found".into(),
            detail: None,
        })
}

/// GET /jobs — List all jobs.
#[cfg(feature = "server")]
pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<super::jobs::Job>> {
    Json(state.job_store.list())
}

/// POST /forensics — Run forensic analysis on a file.
#[cfg(feature = "server")]
pub async fn forensics(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let file_path = if let Some(ref text) = req.text {
        write_temp_file(text.as_bytes(), "forensics.txt")?
    } else {
        return Err(ApiError::bad_request("'text' field is required"));
    };

    let format = crate::scoring::engine::OutputFormat::Json;
    let result_str =
        crate::run_forensics_with_format(&file_path, format).map_err(ApiError::from)?;

    let _ = std::fs::remove_file(&file_path);

    let result: serde_json::Value = serde_json::from_str(&result_str)
        .unwrap_or_else(|_| serde_json::json!({"raw": result_str}));

    Ok(Json(result))
}

/// POST /profile/build — Build an author profile from samples.
#[cfg(feature = "server")]
pub async fn profile_build(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ProfileBuildRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = crate::build_profile(&req.samples_dir, &req.author_name).map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "profile_path": path,
        "author_name": req.author_name,
    })))
}

/// POST /profile/compare — Compare a document against an author profile.
#[cfg(feature = "server")]
pub async fn profile_compare(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ProfileCompareRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let format = crate::scoring::engine::OutputFormat::Json;
    let result_str = crate::analyze_with_format(
        &req.file_path,
        Some(&req.profile_path),
        format,
    )
    .map_err(ApiError::from)?;

    let result: serde_json::Value = serde_json::from_str(&result_str)
        .unwrap_or_else(|_| serde_json::json!({"raw": result_str}));

    Ok(Json(result))
}

/// POST /certificate/generate — Generate a signed Provenance certificate.
#[cfg(feature = "server")]
pub async fn certificate_generate(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CertificateGenerateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cert_path = crate::generate_certificate(
        &req.file_path,
        &req.key_path,
        req.profile_path.as_deref(),
        None,
    )
    .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "certificate_path": cert_path,
    })))
}

/// POST /certificate/verify — Verify a Provenance certificate.
#[cfg(feature = "server")]
pub async fn certificate_verify(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CertificateVerifyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let format = crate::scoring::engine::OutputFormat::Json;
    let result_str = crate::verify_certificate(
        &req.certificate_path,
        req.document_path.as_deref(),
        format,
    )
    .map_err(ApiError::from)?;

    let result: serde_json::Value = serde_json::from_str(&result_str)
        .unwrap_or_else(|_| serde_json::json!({"raw": result_str}));

    Ok(Json(result))
}

/// POST /detect-humanizer — Detect humanizer tool artifacts.
#[cfg(feature = "server")]
pub async fn detect_humanizer(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<HumanizerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let text = if let Some(text) = req.text {
        text
    } else if let Some(ref path) = req.file_path {
        crate::extraction::extract_text(path).map_err(ApiError::from)?
    } else {
        return Err(ApiError::bad_request(
            "Either 'text' or 'file_path' is required",
        ));
    };

    let result = crate::adversarial::humanizer::detect_humanizer(&text);
    let json = serde_json::to_value(&result)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(json))
}

/// GET /adversarial/robustness — Get the robustness matrix.
#[cfg(feature = "server")]
pub async fn robustness_matrix() -> Json<serde_json::Value> {
    let matrix = crate::adversarial::evasion::robustness_matrix();
    let json = serde_json::to_value(&matrix).unwrap_or_default();
    Json(json)
}

/// GET /adversarial/bias — Get the bias audit template.
#[cfg(feature = "server")]
pub async fn bias_audit() -> Json<serde_json::Value> {
    let report = crate::adversarial::bias::generate_audit_template();
    let json = serde_json::to_value(&report).unwrap_or_default();
    Json(json)
}

// ─── Helpers ─────────────────────────────────────────────────────────

#[cfg(feature = "server")]
fn write_temp_file(content: &[u8], name: &str) -> Result<String, ApiError> {
    let temp_dir = std::env::temp_dir().join("provenance_api");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| ApiError::internal(format!("Failed to create temp dir: {e}")))?;

    let unique = format!(
        "{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        name
    );
    let path = temp_dir.join(unique);

    std::fs::write(&path, content)
        .map_err(|e| ApiError::internal(format!("Failed to write temp file: {e}")))?;

    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::internal("Invalid temp path"))
}
