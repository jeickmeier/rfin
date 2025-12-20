//! Integration tests module for market convention refactors.
//!
//! This module contains end-to-end integration tests that verify the
//! complete workflows affected by market convention refactors across all phases.
//!
//! # Phase 1: Critical Safety Fixes
//! - [`metrics_strict_mode`]: Metrics framework with strict error handling
//!
//! # Phase 2: Market Convention Alignment
//! - [`fx_settlement`]: FX spot date calculations and joint business day counting
//!
//! # Phase 3: API Safety & Reporting
//! - [`full_regression`]: Comprehensive end-to-end regression test covering all phases

pub mod full_regression;
pub mod fx_settlement;
pub mod metrics_strict_mode;
