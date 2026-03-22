//! Enterprise Integrations — LMS plugins, publishing connectors, platform API,
//! and Google Workspace integration.
//!
//! Provides adapters for institutional deployment: LTI 1.3 for learning management
//! systems, webhook-based publishing connectors, high-throughput batch API with
//! usage billing, and Google Workspace integration with FERPA compliance.

pub mod lms;
pub mod publishing;
pub mod platform;
pub mod workspace;
