//! Calibration helper macros.

/// Macro to apply calibration with the appropriate solver type.
/// This avoids the need for a SolverInstance enum while working around
/// the object-safety limitations of the Solver trait.
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
                let $solver = finstack_core::math::BrentSolver::new()
                    .with_tolerance($config.tolerance);
                $body
            }
            $crate::calibration::SolverKind::Hybrid => {
                let $solver = finstack_core::math::HybridSolver::new();
                $body
            }
        }
    };
}
