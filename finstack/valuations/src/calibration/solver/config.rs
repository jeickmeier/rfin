//! Solver configuration for calibration procedures.
//!
//! Provides serializable solver configuration that can be persisted
//! in calibration reports for full reproducibility.

use finstack_core::math::solver::BrentSolver;
use serde::{Deserialize, Serialize};

/// Serializable solver configuration for calibration.
///
/// Wraps the Brent 1D solver used by the bootstrap path. The enum shape is retained for
/// schema stability — adding a new variant in the future (e.g. a Newton-Raphson path
/// when one is actually wired) does not require a breaking change.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::calibration::SolverConfig;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SolverConfig::brent_default().with_tolerance(1e-12).with_max_iterations(200);
/// let json = serde_json::to_string(&config)?;
/// # let _ = json;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum SolverConfig {
    /// Brent's method (bracketing solver). Guaranteed to converge if a sign-change
    /// bracket is found. Used by the sequential bootstrap path.
    Brent {
        /// Full solver state from `finstack-core`.
        #[serde(flatten)]
        #[cfg_attr(feature = "ts_export", ts(skip))]
        solver: BrentSolver,
    },
}

impl SolverConfig {
    /// Create a Brent solver config with default settings.
    pub fn brent_default() -> Self {
        Self::Brent {
            solver: BrentSolver::default(),
        }
    }

    /// Get the convergence tolerance.
    pub fn tolerance(&self) -> f64 {
        match self {
            Self::Brent { solver } => solver.tolerance,
        }
    }

    /// Get the maximum number of iterations.
    pub fn max_iterations(&self) -> usize {
        match self {
            Self::Brent { solver } => solver.max_iterations,
        }
    }

    /// Set the tolerance (builder pattern).
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        match &mut self {
            Self::Brent { solver } => solver.tolerance = tolerance,
        }
        self
    }

    /// Set the maximum iterations (builder pattern).
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        match &mut self {
            Self::Brent { solver } => solver.max_iterations = max_iterations,
        }
        self
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self::brent_default()
    }
}

impl From<BrentSolver> for SolverConfig {
    fn from(solver: BrentSolver) -> Self {
        Self::Brent { solver }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_config_default_is_brent() {
        assert!(matches!(
            SolverConfig::default(),
            SolverConfig::Brent { .. }
        ));
    }

    #[test]
    fn test_solver_config_from_brent() {
        let brent = BrentSolver::default();
        let config: SolverConfig = brent.into();
        let SolverConfig::Brent { solver } = config;
        assert_eq!(solver.tolerance, BrentSolver::default().tolerance);
    }

    #[test]
    fn test_solver_config_serde_roundtrip() {
        let config = SolverConfig::brent_default()
            .with_tolerance(1e-14)
            .with_max_iterations(200);
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: SolverConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_solver_config_json_format() {
        let config = SolverConfig::brent_default().with_tolerance(1e-12);
        let json = serde_json::to_string(&config).expect("serialize");
        assert!(json.contains(r#""method":"brent""#));
        assert!(json.contains(r#""tolerance""#));
    }
}
