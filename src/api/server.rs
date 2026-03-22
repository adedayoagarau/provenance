//! Axum HTTP server setup — routes, middleware, and application state.

#[cfg(feature = "server")]
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::auth::{ApiKeyConfig, RateLimiter};
use super::jobs::JobStore;

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Maximum concurrent analysis jobs.
    pub max_concurrent_jobs: usize,
    /// Default rate limit (requests per minute, 0 = unlimited).
    pub default_rate_limit_rpm: u32,
    /// Authentication configuration.
    pub auth: ApiKeyConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            max_body_size: 100 * 1024 * 1024, // 100 MB
            max_concurrent_jobs: 10,
            default_rate_limit_rpm: 60,
            auth: ApiKeyConfig::default(),
        }
    }
}

/// Shared application state.
pub struct AppState {
    pub config: ServerConfig,
    pub job_store: JobStore,
    pub rate_limiter: RateLimiter,
    pub auth_config: ApiKeyConfig,
    pub start_time: std::time::Instant,
}

/// Build the Axum router with all routes.
#[cfg(feature = "server")]
pub fn build_router(config: ServerConfig) -> Router {
    let rate_limiter = RateLimiter::new(config.default_rate_limit_rpm);
    let job_store = JobStore::new(config.max_concurrent_jobs * 100);

    let state = Arc::new(AppState {
        auth_config: config.auth.clone(),
        config: config.clone(),
        job_store,
        rate_limiter,
        start_time: std::time::Instant::now(),
    });

    let api_routes = Router::new()
        // Health
        .route("/health", get(super::handlers::health))
        // Analysis
        .route("/analyze", post(super::handlers::analyze))
        .route("/analyze/async", post(super::handlers::analyze_async))
        // Forensics
        .route("/forensics", post(super::handlers::forensics))
        // Jobs
        .route("/jobs", get(super::handlers::list_jobs))
        .route("/jobs/{id}", get(super::handlers::get_job))
        // Profiles
        .route("/profile/build", post(super::handlers::profile_build))
        .route("/profile/compare", post(super::handlers::profile_compare))
        // Certificates
        .route(
            "/certificate/generate",
            post(super::handlers::certificate_generate),
        )
        .route(
            "/certificate/verify",
            post(super::handlers::certificate_verify),
        )
        // Humanizer detection
        .route(
            "/detect-humanizer",
            post(super::handlers::detect_humanizer),
        )
        // Adversarial / transparency
        .route(
            "/adversarial/robustness",
            get(super::handlers::robustness_matrix),
        )
        .route("/adversarial/bias", get(super::handlers::bias_audit));

    let app = Router::new()
        .nest("/api/v1", api_routes)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            super::auth::auth_middleware,
        ))
        .layer(
            tower_http::cors::CorsLayer::permissive(),
        )
        .layer(
            tower_http::trace::TraceLayer::new_for_http(),
        )
        .layer(
            tower_http::limit::RequestBodyLimitLayer::new(config.max_body_size),
        )
        .with_state(state);

    app
}

/// Start the server.
#[cfg(feature = "server")]
pub async fn start(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);

    tracing::info!("Provenance API server starting on {addr}");
    tracing::info!(
        "Auth: {}",
        if config.auth.require_auth {
            "API key required"
        } else {
            "open access"
        }
    );
    tracing::info!("Rate limit: {} RPM", config.default_rate_limit_rpm);

    let app = build_router(config);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {addr}");
    tracing::info!("API docs: http://{addr}/api/v1/health");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.default_rate_limit_rpm, 60);
        assert!(!config.auth.require_auth);
    }

    #[test]
    fn test_build_router() {
        let config = ServerConfig::default();
        let _router = build_router(config);
        // Router builds without panic
    }
}
