//! Calibration solver submodule (exports-only).
//!
//! This module groups:
//! - Serializable solver configuration (`SolverConfig`) for reproducibility/reporting.
//! - Runtime solver utilities (`create_simple_solver`, bracketing helpers, etc.).

mod config;
mod helpers;

pub use config::SolverConfig;

pub(crate) use helpers::bracket_solve_1d_with_diagnostics;
pub use helpers::{
    create_simple_solver, solve_1d, BracketDiagnostics, OBJECTIVE_VALID_ABS_MAX, PENALTY,
    RESIDUAL_PENALTY_ABS_MIN,
};
