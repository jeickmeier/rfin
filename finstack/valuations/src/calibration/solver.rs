//! Solver utilities and implementations for calibration (v2).
//!
//! This module is the single solver surface for calibration:
//! - **Algorithms**: sequential bootstrapping and global least-squares optimization
//! - **Utilities**: bracketing/root-finding helpers and solver configuration

pub mod bootstrap;
mod config;
pub mod global;
mod helpers;
/// Solver traits for bootstrap and global optimization.
pub mod traits;

pub use crate::calibration::constants::{
    OBJECTIVE_VALID_ABS_MAX, PENALTY, RESIDUAL_PENALTY_ABS_MIN,
};
pub use config::SolverConfig;
pub(crate) use helpers::bracket_solve_1d_with_diagnostics;
pub use helpers::{create_simple_solver, solve_1d, BracketDiagnostics};

pub use bootstrap::SequentialBootstrapper;
pub use global::GlobalFitOptimizer;
pub use traits::{BootstrapTarget, GlobalSolveTarget};
