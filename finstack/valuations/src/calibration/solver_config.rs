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
///
/// // Newton solver with custom tolerance
/// let config = SolverConfig::Newton {
///     tolerance: 1e-14,
///     max_iterations: 100,
///     fd_step: 1e-8,
///     min_derivative: 1e-14,
/// };
///
/// // Serialize to JSON for persistence
/// let json = serde_json::to_string(&config)?;
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum SolverConfig {
    /// Newton-Raphson solver with finite difference derivatives.
    Newton {
        /// Convergence tolerance
        tolerance: f64,
        /// Maximum iterations
        max_iterations: usize,
        /// Base finite difference step (scaled adaptively)
        fd_step: f64,
        /// Minimum derivative threshold
        min_derivative: f64,
    },

    /// Brent's method (bracketing solver).
    Brent {
        /// Convergence tolerance
        tolerance: f64,
        /// Maximum iterations
        max_iterations: usize,
        /// Bracket expansion factor
        bracket_expansion: f64,
        /// Initial bracket size (None = adaptive)
        initial_bracket_size: Option<f64>,
    },
}

impl SolverConfig {
    /// Create a Newton solver config with default settings.
    pub fn newton_default() -> Self {
        let solver = NewtonSolver::default();
        Self::Newton {
            tolerance: solver.tolerance,
            max_iterations: solver.max_iterations,
            fd_step: solver.fd_step,
            min_derivative: solver.min_derivative,
        }
    }

    /// Create a Brent solver config with default settings.
    pub fn brent_default() -> Self {
        let solver = BrentSolver::default();
        Self::Brent {
            tolerance: solver.tolerance,
            max_iterations: solver.max_iterations,
            bracket_expansion: solver.bracket_expansion,
            initial_bracket_size: solver.initial_bracket_size,
        }
    }

    /// Create from a NewtonSolver instance.
    pub fn from_newton(solver: &NewtonSolver) -> Self {
        Self::Newton {
            tolerance: solver.tolerance,
            max_iterations: solver.max_iterations,
            fd_step: solver.fd_step,
            min_derivative: solver.min_derivative,
        }
    }

    /// Create from a BrentSolver instance.
    pub fn from_brent(solver: &BrentSolver) -> Self {
        Self::Brent {
            tolerance: solver.tolerance,
            max_iterations: solver.max_iterations,
            bracket_expansion: solver.bracket_expansion,
            initial_bracket_size: solver.initial_bracket_size,
        }
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

        if let SolverConfig::Newton { tolerance, .. } = config {
            assert_eq!(tolerance, 1e-15);
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
                tolerance: 1e-14,
                max_iterations: 200,
                fd_step: 1e-7,
                min_derivative: 1e-16,
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
            tolerance: 1e-12,
            max_iterations: 50,
            fd_step: 1e-8,
            min_derivative: 1e-14,
        };

        let json =
            serde_json::to_string_pretty(&config).expect("Serialization should succeed in test");
        assert!(json.contains(r#""method": "newton"#));
        assert!(json.contains(r#""tolerance"#));
    }
}
