//! Calibration configuration and solver selection.
//!
//! This module provides unified configuration for calibration processes including:
//! - Solver selection and numerical parameters
//! - Multi-curve framework configuration (post-2008 standard)
//! - Entity seniority mappings for credit calibration

use finstack_core::market_data::term_structures::Seniority;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// Solver type selection for calibration.
///
/// For 1D problems (most curve calibrations), Newton, Brent, or Hybrid are used directly.
/// For multi-dimensional problems, use `create_lm_solver()` or `create_de_solver()` methods
/// on `CalibrationConfig`. Note that LevenbergMarquardt and DifferentialEvolution variants
/// automatically fall back to Hybrid for 1D solve_1d() calls.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Solver Kind enumeration.
pub enum SolverKind {
    /// Newton-Raphson solver with automatic derivative estimation (1D only)
    Newton,
    /// Brent's method solver - robust bracketing method (1D only)
    Brent,
    /// Hybrid solver: Newton first, Brent fallback (1D only, recommended)
    #[default]
    Hybrid,
    /// Levenberg-Marquardt for non-linear least squares (multi-dimensional).
    /// Falls back to Hybrid for 1D problems. Use `create_lm_solver()` for multi-D.
    LevenbergMarquardt,
    /// Differential Evolution for global optimization (multi-dimensional).
    /// Falls back to Hybrid for 1D problems. Use `create_de_solver()` for multi-D.
    DifferentialEvolution,
}

/// Multi-curve calibration configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Multi Curve Config structure.
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
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Calibration Config structure.
pub struct CalibrationConfig {
    /// Solver tolerance
    pub tolerance: f64,
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

/// Market-standard acceptable calibration residual tolerances.
///
/// These tolerances represent typical market practice for liquid instruments in normal
/// market conditions. Adjust based on:
/// - Instrument liquidity (wider for illiquid instruments)
/// - Market volatility (wider during stressed periods)
/// - Data quality (wider for stale or indicative quotes)
/// - Use case (tighter for arbitrage strategies, wider for risk management)
///
/// Calibration Tolerances structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)] // Public API type intended for external consumers; not used internally yet
pub struct CalibrationTolerances {
    /// Maximum absolute residual for deposits/futures (basis points).
    ///
    /// Standard: 0.1 bp for highly liquid overnight and short-term money markets.
    /// Consider 0.5 bp for less liquid tenors or currencies.
    pub deposits_bp: f64,

    /// Maximum absolute residual for swaps (basis points).
    ///
    /// Standard: 0.5 bp for standard benchmark tenors (2Y, 5Y, 10Y, 30Y) in G10 currencies.
    /// Consider 1-2 bp for off-the-run tenors or emerging market currencies.
    pub swaps_bp: f64,

    /// Maximum relative residual for volatility surface calibration.
    ///
    /// Standard: 1% (0.01) for ATM volatilities, 2% (0.02) for wings and far strikes.
    /// Absolute tolerance for low-vol regimes may be more appropriate.
    pub vol_relative: f64,

    /// Maximum absolute residual for credit spreads (basis points).
    ///
    /// Standard: 1 bp for liquid single-name CDS, 2-3 bp for CDS indices.
    /// Consider 5-10 bp for illiquid names or during credit stress.
    pub credit_bp: f64,

    /// Maximum absolute residual for basis swaps (basis points).
    ///
    /// Standard: 0.5 bp for standard LIBOR basis swaps (3M vs 6M).
    /// Consider 1-2 bp for cross-currency basis or exotic bases.
    pub basis_bp: f64,
}

impl Default for CalibrationTolerances {
    /// Returns market-standard tolerances for liquid G10 instruments.
    fn default() -> Self {
        Self {
            deposits_bp: 0.1,
            swaps_bp: 0.5,
            vol_relative: 0.01,
            credit_bp: 1.0,
            basis_bp: 0.5,
        }
    }
}

impl CalibrationTolerances {}
