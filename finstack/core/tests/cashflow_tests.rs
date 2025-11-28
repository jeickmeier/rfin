//! Cashflow module integration tests.
//!
//! This test suite verifies market-standard correctness for:
//! - XIRR/IRR calculations with reference golden values
//! - CashFlow validation edge cases
//! - Day count conventions including leap year handling
//! - Numerical stability with large cashflow counts and extreme durations
//!
//! # Test Organization
//!
//! - `test_helpers`: Shared tolerance constants and test curves
//! - `cashflow_primitives`: Basic CashFlow struct tests
//! - `xirr_golden`: Reference golden values from financial textbooks
//! - `validation_edge_cases`: Input validation tests
//! - `irr_edge_cases`: Boundary condition tests for IRR/XIRR
//! - `daycount_leap_year`: Leap year handling tests
//! - `numerical_stability`: Large-scale and extreme value tests

#[path = "cashflow/test_helpers.rs"]
mod test_helpers;

#[path = "cashflow/cashflow_primitives.rs"]
mod cashflow_primitives;

#[path = "cashflow/xirr_golden.rs"]
mod xirr_golden;

#[path = "cashflow/validation_edge_cases.rs"]
mod validation_edge_cases;

#[path = "cashflow/irr_edge_cases.rs"]
mod irr_edge_cases;

#[path = "cashflow/daycount_leap_year.rs"]
mod daycount_leap_year;

#[path = "cashflow/numerical_stability.rs"]
mod numerical_stability;
