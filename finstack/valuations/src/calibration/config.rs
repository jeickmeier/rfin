//! Calibration configuration and solver selection.
//!
//! This module provides unified configuration for calibration processes including:
//! - Solver selection and numerical parameters
//! - Multi-curve framework mode (post-2008 vs legacy single-curve)
//! - Entity seniority mappings for credit calibration

use finstack_core::F;
use finstack_core::market_data::term_structures::Seniority;
use serde::{Deserialize, Serialize};
use hashbrown::HashMap;

/// Solver type selection for calibration.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum SolverKind {
    /// Newton-Raphson solver with automatic derivative estimation (1D)
    Newton,
    /// Brent's method solver (robust, bracketing required) (1D)
    Brent,
    /// Hybrid solver that tries Newton first, falls back to Brent (1D)
    #[default]
    Hybrid,
    /// Levenberg-Marquardt for non-linear least squares (multi-dimensional)
    LevenbergMarquardt,
    /// Differential Evolution for global optimization (multi-dimensional)
    DifferentialEvolution,
}

/// Multi-curve calibration configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiCurveConfig {
    /// Whether to calibrate basis spreads
    pub calibrate_basis: bool,

    /// Whether to enforce strict separation (fail if trying to derive forward from discount)
    pub enforce_separation: bool,
}

impl Default for MultiCurveConfig {
    fn default() -> Self {
        Self {
            calibrate_basis: true,
            enforce_separation: true,
        }
    }
}

impl MultiCurveConfig {
    /// Create a multi-curve configuration (post-2008 standard)
    pub fn new() -> Self {
        Self::default()
    }
}

/// Configuration for calibration processes.
#[derive(Clone, Debug)]
pub struct CalibrationConfig {
    /// Solver tolerance
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Use parallel processing when available
    pub use_parallel: bool,
    /// Random seed for reproducible results
    pub random_seed: Option<u64>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Solver type selection
    pub solver_kind: SolverKind,
    /// Entity-specific seniority mappings for credit calibration
    pub entity_seniority: HashMap<String, Seniority>,
    /// Multi-curve framework configuration
    pub multi_curve: MultiCurveConfig,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            use_parallel: false, // Deterministic by default
            random_seed: Some(42),
            verbose: false,
            solver_kind: SolverKind::default(),
            entity_seniority: HashMap::new(),
            multi_curve: MultiCurveConfig::default(),
        }
    }
}

impl CalibrationConfig {
    /// Set multi-curve framework configuration.
    /// This is a convenience method for backward compatibility.
    pub fn with_multi_curve_config(mut self, multi_curve_config: MultiCurveConfig) -> Self {
        self.multi_curve = multi_curve_config;
        self
    }

    /// Create a configuration for multi-curve mode (standard).
    pub fn multi_curve() -> Self {
        Self::default()
    }

    /// Create a Levenberg-Marquardt solver with current config settings.
    ///
    /// Returns a configured LevenbergMarquardtSolver for multi-dimensional optimization.
    /// We return the concrete type instead of trait object since MultiSolver
    /// is not object-safe due to generic parameters.
    pub fn create_lm_solver(&self) -> finstack_core::math::solver_multi::LevenbergMarquardtSolver {
        use finstack_core::math::solver_multi::LevenbergMarquardtSolver;

        LevenbergMarquardtSolver::new()
            .with_tolerance(self.tolerance)
            .with_max_iterations(self.max_iterations)
    }

    /// Create a Differential Evolution solver with current config settings.
    ///
    /// Returns a configured DifferentialEvolutionSolver for global optimization.
    /// Useful when the objective function has multiple local minima.
    pub fn create_de_solver(
        &self,
    ) -> finstack_core::math::solver_multi::DifferentialEvolutionSolver {
        use finstack_core::math::solver_multi::DifferentialEvolutionSolver;

        let mut solver = DifferentialEvolutionSolver::new()
            .with_tolerance(self.tolerance)
            .with_max_generations(self.max_iterations);

        if let Some(seed) = self.random_seed {
            solver = solver.with_seed(seed);
        }

        solver
    }

    /// Check if configured for multi-dimensional solving
    pub fn is_multi_dimensional(&self) -> bool {
        matches!(
            self.solver_kind,
            SolverKind::LevenbergMarquardt | SolverKind::DifferentialEvolution
        )
    }
}
