//! Calibration configuration and solver selection.
//!
//! This module provides unified configuration for calibration processes including:
//! - Solver selection and numerical parameters
//! - Multi-curve framework configuration (post-2008 standard)
//! - Entity seniority mappings for credit calibration

use finstack_core::explain::ExplainOpts;
use finstack_core::market_data::term_structures::Seniority;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
/// Runtime validation behavior for arbitrage/consistency checks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationMode {
    /// Emit warnings (non-fatal) when validations fail
    Warn,
    /// Treat validation failures as hard errors
    Error,
}

/// Solver type selection for calibration.
///
/// For 1D problems (most curve calibrations), Newton or Brent are used directly.
/// For multi-dimensional problems, use `create_lm_solver()` method on `CalibrationConfig`.
/// Note that LevenbergMarquardt variant automatically falls back to Brent for 1D solve_1d() calls.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Solver Kind enumeration.
pub enum SolverKind {
    /// Newton-Raphson solver with automatic derivative estimation (1D only)
    Newton,
    /// Brent's method solver - robust bracketing method (1D only, recommended)
    #[default]
    Brent,
    /// Levenberg-Marquardt for non-linear least squares (multi-dimensional).
    /// Falls back to Brent for 1D problems. Use `create_lm_solver()` for multi-D.
    LevenbergMarquardt,
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
    /// Use finite-difference gradients for SABR calibration instead of analytical approximations.
    /// Analytical gradients are faster but use approximations (treating x(z) as ~constant).
    /// FD gradients are slower but more accurate, especially for far-from-ATM strikes.
    pub use_fd_sabr_gradients: bool,
    /// Explanation options (opt-in detailed trace)
    #[serde(skip)]
    pub explain: ExplainOpts,
    /// Runtime validation mode (warnings vs errors). Feature `strict_validation` may still harden checks.
    pub validation_mode: ValidationMode,
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
            use_fd_sabr_gradients: false, // Use fast analytical approximations by default
            explain: ExplainOpts::default(), // Disabled by default (zero overhead)
            validation_mode: ValidationMode::Warn,
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

    /// Enable explanation tracing with default settings.
    pub fn with_explain(mut self) -> Self {
        self.explain = ExplainOpts::enabled();
        self
    }

    /// Set custom explanation options.
    pub fn with_explain_opts(mut self, opts: ExplainOpts) -> Self {
        self.explain = opts;
        self
    }

    /// Create a conservative calibration configuration.
    ///
    /// Conservative settings prioritize stability and robustness over speed:
    /// - Higher tolerance for better accuracy (1e-12)
    /// - Moderate iteration limit (100)
    /// - Smaller step size for cautious progression (0.5x)
    /// - Regularization enabled to prevent overfitting
    ///
    /// Use this for:
    /// - Production risk systems requiring high precision
    /// - Illiquid instruments with sparse quotes
    /// - When calibration stability is more important than speed
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_valuations::calibration::CalibrationConfig;
    ///
    /// let config = CalibrationConfig::conservative();
    /// assert_eq!(config.tolerance, 1e-12);
    /// ```
    pub fn conservative() -> Self {
        Self {
            tolerance: 1e-12,
            max_iterations: 100,
            use_parallel: false, // Deterministic
            random_seed: Some(42),
            verbose: false,
            solver_kind: SolverKind::Brent, // Robust bracketing solver
            entity_seniority: HashMap::new(),
            multi_curve: MultiCurveConfig::default(),
            use_fd_sabr_gradients: true, // Use more accurate FD gradients for conservative mode
            explain: ExplainOpts::default(),
            validation_mode: ValidationMode::Warn,
        }
    }

    /// Create an aggressive calibration configuration.
    ///
    /// Aggressive settings prioritize speed and will tolerate looser fits:
    /// - Relaxed tolerance for faster convergence (1e-6)
    /// - Higher iteration limit (1000) for difficult problems
    /// - Normal step size (1.0x)
    /// - No regularization for exact fit to quotes
    ///
    /// Use this for:
    /// - Real-time pricing applications
    /// - Liquid instruments with tight bid/ask spreads
    /// - When speed is critical and small fitting errors are acceptable
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_valuations::calibration::CalibrationConfig;
    ///
    /// let config = CalibrationConfig::aggressive();
    /// assert_eq!(config.max_iterations, 1000);
    /// ```
    pub fn aggressive() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 1000,
            use_parallel: false, // Keep deterministic by default
            random_seed: Some(42),
            verbose: false,
            solver_kind: SolverKind::Brent,
            entity_seniority: HashMap::new(),
            multi_curve: MultiCurveConfig::default(),
            use_fd_sabr_gradients: false, // Use fast analytical approximations for speed
            explain: ExplainOpts::default(),
            validation_mode: ValidationMode::Warn,
        }
    }

    /// Create a fast calibration configuration for interactive use.
    ///
    /// Fast settings sacrifice some accuracy for speed:
    /// - Very relaxed tolerance (1e-4)
    /// - Low iteration limit (50)
    /// - Brent solver (faster for simple curves)
    /// - No regularization
    ///
    /// Use this for:
    /// - Interactive exploration and what-if scenarios
    /// - Approximate pricing where precision isn't critical
    /// - Quick sanity checks during development
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_valuations::calibration::CalibrationConfig;
    ///
    /// let config = CalibrationConfig::fast();
    /// assert_eq!(config.max_iterations, 50);
    /// ```
    pub fn fast() -> Self {
        Self {
            tolerance: 1e-4,
            max_iterations: 50,
            use_parallel: false,
            random_seed: Some(42),
            verbose: false,
            solver_kind: SolverKind::Brent, // Fast bracketing method
            entity_seniority: HashMap::new(),
            multi_curve: MultiCurveConfig::default(),
            use_fd_sabr_gradients: false, // Use fast analytical approximations
            explain: ExplainOpts::default(),
            validation_mode: ValidationMode::Warn,
        }
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

    /// Check if configured for multi-dimensional solving
    pub fn is_multi_dimensional(&self) -> bool {
        matches!(self.solver_kind, SolverKind::LevenbergMarquardt)
    }
}
