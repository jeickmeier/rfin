//! Math module integration tests.
//!
//! This test suite verifies mathematical correctness for:
//! - Interpolation (linear, log-linear, cubic spline, etc.)
//! - Root-finding algorithms (Brent, Newton)
//! - Numerical integration (quadrature)
//! - Statistics (mean, variance, realized volatility)
//! - Compensated summation
//!
//! # Test Organization
//!
//! - `common`: Shared test helpers (approx_eq, standard test data)
//! - `interp`: Interpolation tests (all types, extrapolation, derivatives)
//! - `solver`: Root-finding tests (Brent, Newton)
//! - `integration`: Numerical quadrature tests
//! - `stats`: Statistics tests (mean, variance, realized vol)
//! - `summation`: Compensated summation tests

mod math;
