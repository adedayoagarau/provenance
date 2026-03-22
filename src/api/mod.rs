//! API and Platform Layer — Axum-based HTTP service for Provenance.
//!
//! Provides REST endpoints for document analysis, forensics, profile management,
//! certificate operations, and humanizer detection. Supports async job submission
//! for long-running analyses, rate limiting, and API key authentication.

#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod handlers;
#[cfg(feature = "server")]
pub mod jobs;
#[cfg(feature = "server")]
pub mod auth;
#[cfg(feature = "server")]
pub mod errors;
