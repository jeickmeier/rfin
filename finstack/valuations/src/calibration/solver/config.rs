//! Solver configuration for calibration procedures.
//!
//! Provides serializable solver configuration that can be persisted
//! in calibration reports for full reproducibility.

use finstack_core::math::solver::{BrentSolver, NewtonSolver};
use serde::{Deserialize, Serialize};

/// Serializable solver configuration for calibration.
///
/// Captures the complete solver state including method choice and
/// all parameters. This enables full reproducibility of calibration
/// results.
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::calibration::SolverConfig;
/// use finstack_core::math::solver::NewtonSolver;
///
/// // Newton solver with custom tolerance
/// let config = SolverConfig::Newton {
///     solver: NewtonSolver {
///         tolerance: 1e-14,
///         max_iterations: 100,
///         fd_step: 1e-8,
///         min_derivative: 1e-14,
///     }
/// };
///
/// // Serialize to JSON for persistence
/// let json = serde_json::to_string(&config)?;
/// ```
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum SolverConfig {
    /// Newton-Raphson solver with finite difference derivatives.
    Newton {
        /// Full solver state from `finstack-core`.
        #[serde(flatten)]
        #[cfg_attr(feature = "ts_export", ts(skip))]
        solver: NewtonSolver,
    },

    /// Brent's method (bracketing solver).
    Brent {
        /// Full solver state from `finstack-core`.
        #[serde(flatten)]
        #[cfg_attr(feature = "ts_export", ts(skip))]
        solver: BrentSolver,
    },
    /// Global Newton-style solve (with optional LM damping for robustness).
    GlobalNewton {
        /// Convergence tolerance
        tolerance: f64,
        /// Maximum iterations
        max_iterations: usize,
        /// Apply LM-style damping to improve robustness
        use_lm_damping: bool,
    },
}

impl SolverConfig {
    /// Create a Newton solver config with default settings.
    pub fn newton_default() -> Self {
        Self::Newton {
            solver: NewtonSolver::default(),
        }
    }

    /// Create a Brent solver config with default settings.
    pub fn brent_default() -> Self {
        Self::Brent {
            solver: BrentSolver::default(),
        }
    }

    /// Create a Global Newton config with Newton defaults and LM damping enabled.
    pub fn global_newton_default() -> Self {
        let solver = NewtonSolver::default();
        Self::GlobalNewton {
            tolerance: solver.tolerance,
            max_iterations: solver.max_iterations,
            use_lm_damping: true,
        }
    }

    /// Create from a NewtonSolver instance.
    pub fn from_newton(solver: &NewtonSolver) -> Self {
        Self::Newton {
            solver: solver.clone(),
        }
    }

    /// Create from a BrentSolver instance.
    pub fn from_brent(solver: &BrentSolver) -> Self {
        Self::Brent {
            solver: solver.clone(),
        }
    }

    /// Get the convergence tolerance.
    pub fn tolerance(&self) -> f64 {
        match self {
            Self::Newton { solver } => solver.tolerance,
            Self::Brent { solver } => solver.tolerance,
            Self::GlobalNewton { tolerance, .. } => *tolerance,
        }
    }

    /// Get the maximum number of iterations.
    pub fn max_iterations(&self) -> usize {
        match self {
            Self::Newton { solver } => solver.max_iterations,
            Self::Brent { solver } => solver.max_iterations,
            Self::GlobalNewton { max_iterations, .. } => *max_iterations,
        }
    }

    /// Set the tolerance (builder pattern).
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        match &mut self {
            Self::Newton { solver } => solver.tolerance = tolerance,
            Self::Brent { solver } => solver.tolerance = tolerance,
            Self::GlobalNewton {
                tolerance: t, ..
            } => *t = tolerance,
        }
        self
    }

    /// Set the maximum iterations (builder pattern).
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        match &mut self {
            Self::Newton { solver } => solver.max_iterations = max_iterations,
            Self::Brent { solver } => solver.max_iterations = max_iterations,
            Self::GlobalNewton {
                max_iterations: m, ..
            } => *m = max_iterations,
        }
        self
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self::brent_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_config_defaults() {
        let newton = SolverConfig::newton_default();
        let brent = SolverConfig::brent_default();

        // Verify they're different types
        assert!(matches!(newton, SolverConfig::Newton { .. }));
        assert!(matches!(brent, SolverConfig::Brent { .. }));
    }

    #[test]
    fn test_solver_config_from_instances() {
        let newton = NewtonSolver::new().with_tolerance(1e-15);
        let config = SolverConfig::from_newton(&newton);

        if let SolverConfig::Newton { solver } = config {
            assert_eq!(solver.tolerance, 1e-15);
        } else {
            panic!("Expected Newton config");
        }
    }

    #[test]
    fn test_solver_config_serde_roundtrip() {
        let configs = vec![
            SolverConfig::newton_default(),
            SolverConfig::brent_default(),
            SolverConfig::Newton {
                solver: NewtonSolver {
                    tolerance: 1e-14,
                    max_iterations: 200,
                    fd_step: 1e-7,
                    min_derivative: 1e-16,
                    min_derivative_rel: 1e-6,
                },
            },
        ];

        for config in configs {
            let json =
                serde_json::to_string(&config).expect("Serialization should succeed in test");
            let deserialized: SolverConfig =
                serde_json::from_str(&json).expect("Deserialization should succeed in test");
            assert_eq!(config, deserialized);
        }
    }

    #[test]
    fn test_solver_config_json_format() {
        // Verify tagged enum serialization format
        let config = SolverConfig::Newton {
            solver: NewtonSolver {
                tolerance: 1e-12,
                max_iterations: 50,
                fd_step: 1e-8,
                min_derivative: 1e-14,
                min_derivative_rel: 1e-6,
            },
        };

        let json =
            serde_json::to_string_pretty(&config).expect("Serialization should succeed in test");
        assert!(json.contains(r#""method": "newton"#));
        assert!(json.contains(r#""tolerance"#));
    }
}
