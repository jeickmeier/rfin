#![cfg(feature = "golden")]

//! Golden test harness for finstack-core.
//!
//! This file includes the golden test modules that load expected values
//! from JSON fixtures and validate against computed results.

#[path = "golden/mod.rs"]
mod golden;
