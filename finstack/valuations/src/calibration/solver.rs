//! Solver utilities and implementations for calibration (v2).
//!
//! This module is the single solver surface for calibration:
//! - **Algorithms**: sequential bootstrapping and global least-squares optimization
//! - **Utilities**: bracketing/root-finding helpers and solver configuration

mod config;
mod helpers;
pub mod bootstrap;
pub mod global;
/// Solver traits for bootstrap and global optimization.
pub mod traits;

pub use config::SolverConfig;
pub(crate) use helpers::bracket_solve_1d_with_diagnostics;
pub use helpers::{
    create_simple_solver, solve_1d, BracketDiagnostics, OBJECTIVE_VALID_ABS_MAX, PENALTY,
    RESIDUAL_PENALTY_ABS_MIN,
};

pub use bootstrap::SequentialBootstrapper;
pub use global::GlobalFitOptimizer;
pub use traits::{BootstrapTarget, GlobalSolveTarget};
