//! Property-based test suite entry point.
//!
//! Market Standards Review (Priority 4, Week 3)
//!
//! These tests verify mathematical invariants using proptest to explore
//! thousands of input combinations and catch edge cases.

#[path = "properties/mod.rs"]
mod properties_tests;
