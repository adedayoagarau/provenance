//! Adversarial Robustness — evasion testing, humanizer resistance, bias audit,
//! and published accuracy reporting.
//!
//! This module provides tools to systematically evaluate Provenance's resilience
//! against adversarial attacks, measure detection accuracy across demographics
//! and registers, and produce transparent accuracy reports.

pub mod evasion;
pub mod humanizer;
pub mod bias;
pub mod accuracy;
