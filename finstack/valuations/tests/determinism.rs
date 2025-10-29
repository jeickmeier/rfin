//! Determinism test suite entry point.
//!
//! Market Standards Review (Priority 2, Week 2)
//!
//! These tests ensure that all pricing operations are deterministic:
//! same inputs → bitwise-identical outputs across multiple runs.

#[path = "determinism/mod.rs"]
mod determinism_tests;
