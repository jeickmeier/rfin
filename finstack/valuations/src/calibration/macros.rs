//! Calibration helper macros.

/// Macro to apply calibration with the appropriate solver type.
/// This avoids the need for a SolverInstance enum while working around
/// the object-safety limitations of the Solver trait.
///
/// For multi-dimensional solvers (LevenbergMarquardt, DifferentialEvolution),
/// this falls back to HybridSolver for 1D problems. Use create_lm_solver()
/// directly for multi-dimensional optimization.
#[macro_export]
macro_rules! with_solver {
    ($config:expr, |$solver:ident| $body:expr) => {
        match $config.solver_kind {
            $crate::calibration::SolverKind::Newton => {
                let $solver = finstack_core::math::NewtonSolver::new()
                    .with_tolerance($config.tolerance)
                    .with_max_iterations($config.max_iterations);
                $body
            }
            $crate::calibration::SolverKind::Brent => {
                let $solver =
                    finstack_core::math::BrentSolver::new().with_tolerance($config.tolerance);
                $body
            }
            $crate::calibration::SolverKind::Hybrid => {
                let $solver = finstack_core::math::HybridSolver::new()
                    .with_tolerance($config.tolerance)
                    .with_max_iterations($config.max_iterations);
                $body
            }
            $crate::calibration::SolverKind::LevenbergMarquardt
            | $crate::calibration::SolverKind::DifferentialEvolution => {
                // For 1D problems, fall back to Hybrid solver
                // Multi-dimensional solvers should use create_lm_solver() directly
                let $solver = finstack_core::math::HybridSolver::new()
                    .with_tolerance($config.tolerance)
                    .with_max_iterations($config.max_iterations);
                $body
            }
        }
    };
}
