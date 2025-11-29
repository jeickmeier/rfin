//! Math module tests.
//!
//! This module contains tests for:
//! - Interpolation (all types, extrapolation, derivatives)
//! - Root-finding (Brent, Newton)
//! - Numerical integration
//! - Statistics (mean, variance, realized vol)
//! - Compensated summation

mod common;

mod integration;
mod interp;
mod solver;
mod stats;
mod summation;
