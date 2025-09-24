//! Tiny factory for 1D solvers to replace the `with_solver!` macro.
//!
//! Provides a concrete `SolverInstance` enum that implements the `Solver` trait,
//! allowing call sites to work with a single concrete type without macros.

use finstack_core::math::{BrentSolver, HybridSolver, NewtonSolver, Solver};

use super::config::{CalibrationConfig, SolverKind};

/// Concrete solver wrapper that dispatches to specific 1D algorithms.
#[derive(Clone, Debug)]
pub enum SolverInstance {
    Newton(NewtonSolver),
    Brent(BrentSolver),
    Hybrid(HybridSolver),
}

impl Solver for SolverInstance {
    fn solve<Func>(&self, f: Func, initial_guess: finstack_core::F) -> finstack_core::Result<finstack_core::F>
    where
        Func: Fn(finstack_core::F) -> finstack_core::F,
    {
        match self {
            SolverInstance::Newton(s) => s.solve(f, initial_guess),
            SolverInstance::Brent(s) => s.solve(f, initial_guess),
            SolverInstance::Hybrid(s) => s.solve(f, initial_guess),
        }
    }
}

/// Create a configured 1D solver instance from calibration settings.
///
/// Multi-dimensional solver kinds (LM/DE) are not object-safe in the 1D `Solver`
/// trait, so we conservatively fall back to `Hybrid` for 1D problems.
pub fn make_solver(cfg: &CalibrationConfig) -> SolverInstance {
    match cfg.solver_kind {
        SolverKind::Newton => SolverInstance::Newton(
            NewtonSolver::new()
                .with_tolerance(cfg.tolerance)
                .with_max_iterations(cfg.max_iterations),
        ),
        SolverKind::Brent => SolverInstance::Brent(
            BrentSolver::new().with_tolerance(cfg.tolerance),
        ),
        SolverKind::Hybrid
        | SolverKind::LevenbergMarquardt
        | SolverKind::DifferentialEvolution => SolverInstance::Hybrid(
            HybridSolver::new()
                .with_tolerance(cfg.tolerance)
                .with_max_iterations(cfg.max_iterations),
        ),
    }
}

