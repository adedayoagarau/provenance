//! API error types — maps internal errors to HTTP responses.

#[cfg(feature = "server")]
use axum::http::StatusCode;
#[cfg(feature = "server")]
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// API error response body.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[cfg(feature = "server")]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "bad_request" | "validation_error" => StatusCode::BAD_REQUEST,
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
            "payload_too_large" => StatusCode::PAYLOAD_TOO_LARGE,
            "job_not_found" => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = serde_json::to_string(&self).unwrap_or_else(|_| {
            r#"{"error":"Internal server error","code":"internal"}"#.to_string()
        });

        (status, [("content-type", "application/json")], body).into_response()
    }
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "bad_request".into(),
            detail: None,
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "not_found".into(),
            detail: None,
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "internal".into(),
            detail: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            error: "Invalid or missing API key".into(),
            code: "unauthorized".into(),
            detail: None,
        }
    }

    pub fn rate_limited() -> Self {
        Self {
            error: "Rate limit exceeded. Please try again later.".into(),
            code: "rate_limited".into(),
            detail: None,
        }
    }

    pub fn payload_too_large(max_bytes: usize) -> Self {
        Self {
            error: format!("Request body exceeds maximum size of {} bytes", max_bytes),
            code: "payload_too_large".into(),
            detail: None,
        }
    }
}

impl From<crate::utils::errors::ProvenanceError> for ApiError {
    fn from(err: crate::utils::errors::ProvenanceError) -> Self {
        use crate::utils::errors::ProvenanceError;
        match &err {
            ProvenanceError::FileNotFound { .. } => ApiError::not_found(err.to_string()),
            ProvenanceError::UnsupportedFormat { .. } => ApiError::bad_request(err.to_string()),
            ProvenanceError::InsufficientText { .. } => ApiError::bad_request(err.to_string()),
            ProvenanceError::ConfigError { .. } => ApiError::bad_request(err.to_string()),
            _ => ApiError::internal(err.to_string()),
        }
    }
}
