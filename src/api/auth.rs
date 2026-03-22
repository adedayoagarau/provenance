//! API key authentication and rate limiting middleware.

#[cfg(feature = "server")]
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// API key configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Valid API keys mapped to their metadata.
    pub keys: HashMap<String, ApiKeyMeta>,
    /// Whether authentication is required (false = open access).
    pub require_auth: bool,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            keys: HashMap::new(),
            require_auth: false,
        }
    }
}

/// Metadata about an API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyMeta {
    /// Human-readable name for the key owner.
    pub name: String,
    /// Maximum requests per minute (0 = unlimited).
    pub rate_limit_rpm: u32,
    /// Maximum file size in bytes (0 = server default).
    pub max_file_size: usize,
}

/// Rate limiter state.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// key → (request_count, window_start_epoch_secs)
    state: Arc<Mutex<HashMap<String, (u32, u64)>>>,
    /// Default requests per minute for unauthenticated access.
    pub default_rpm: u32,
}

impl RateLimiter {
    pub fn new(default_rpm: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            default_rpm,
        }
    }

    /// Check if a request is allowed. Returns true if within rate limit.
    pub fn check(&self, key: &str, limit_rpm: u32) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut state = self.state.lock().unwrap();
        let entry = state.entry(key.to_string()).or_insert((0, now));

        // Reset window if more than 60 seconds have passed
        if now - entry.1 >= 60 {
            entry.0 = 0;
            entry.1 = now;
        }

        if limit_rpm == 0 || entry.0 < limit_rpm {
            entry.0 += 1;
            true
        } else {
            false
        }
    }

    /// Clean up expired entries (call periodically).
    pub fn cleanup(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut state = self.state.lock().unwrap();
        state.retain(|_, (_, window_start)| now - *window_start < 120);
    }
}

/// Authentication middleware.
#[cfg(feature = "server")]
pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<super::server::AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let config = &state.auth_config;

    if !config.require_auth {
        // Check default rate limit for unauthenticated access
        let ip = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string();

        if !state.rate_limiter.check(&ip, state.rate_limiter.default_rpm) {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        return Ok(next.run(request).await);
    }

    // Extract API key from header
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
        });

    let Some(key) = api_key else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Some(meta) = config.keys.get(key) else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    // Check rate limit
    let rpm = if meta.rate_limit_rpm > 0 {
        meta.rate_limit_rpm
    } else {
        state.rate_limiter.default_rpm
    };

    if !state.rate_limiter.check(key, rpm) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(10);
        for _ in 0..10 {
            assert!(limiter.check("test-key", 10));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(limiter.check("test-key", 5));
        }
        assert!(!limiter.check("test-key", 5));
    }

    #[test]
    fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("key-a", 2));
        assert!(limiter.check("key-a", 2));
        assert!(!limiter.check("key-a", 2));
        // Different key should still be allowed
        assert!(limiter.check("key-b", 2));
    }

    #[test]
    fn test_rate_limiter_unlimited() {
        let limiter = RateLimiter::new(0);
        for _ in 0..1000 {
            assert!(limiter.check("test-key", 0));
        }
    }

    #[test]
    fn test_api_key_config_default() {
        let config = ApiKeyConfig::default();
        assert!(!config.require_auth);
        assert!(config.keys.is_empty());
    }
}
