//! Calibration configuration and solver selection.
//!
//! This module provides unified configuration for calibration processes including:
//! - Solver selection and numerical parameters
//! - Multi-curve framework configuration (post-2008 standard)
//! - Entity seniority mappings for credit calibration
//! - Rate bounds for market-regime-aware calibration

use finstack_core::currency::Currency;
use finstack_core::explain::ExplainOpts;
use finstack_core::market_data::term_structures::Seniority;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

fn default_rate_bounds_policy_for_serde() -> RateBoundsPolicy {
    // Backward compatible: older serialized configs only have `rate_bounds`.
    RateBoundsPolicy::Explicit
}

/// Configurable bounds for forward/zero rates during calibration.
///
/// Different market regimes require different rate bounds:
/// - Developed markets (USD, EUR, GBP): typically [-2%, 50%]
/// - Negative rate environments (EUR, JPY, CHF): [-5%, 20%]
/// - Emerging markets (TRY, ARS, BRL): [-5%, 200%]
///
/// # Examples
///
/// ```
/// use finstack_valuations::calibration::RateBounds;
/// use finstack_core::currency::Currency;
///
/// // Use currency-specific defaults
/// let usd_bounds = RateBounds::for_currency(Currency::USD);
/// assert!(usd_bounds.min_rate < 0.0);
///
/// // Or customize for specific scenarios
/// let em_bounds = RateBounds::emerging_markets();
/// assert!(em_bounds.max_rate > 1.0);
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RateBounds {
    /// Minimum allowed rate (decimal, e.g., -0.02 for -2%)
    pub min_rate: f64,
    /// Maximum allowed rate (decimal, e.g., 0.50 for 50%)
    pub max_rate: f64,
}

impl Default for RateBounds {
    fn default() -> Self {
        Self {
            min_rate: -0.02,
            max_rate: 0.50,
        }
    }
}

impl RateBounds {
    /// Create rate bounds for a specific currency based on market conventions.
    ///
    /// - USD/CAD/AUD: Standard developed market bounds [-2%, 50%]
    /// - EUR/JPY/CHF: Extended negative rate support [-5%, 30%]
    /// - GBP: Standard with slightly wider negative [-3%, 50%]
    /// - TRY/ARS/BRL/ZAR: Emerging market bounds [-5%, 200%]
    /// - Other: Conservative developed market defaults
    pub fn for_currency(currency: Currency) -> Self {
        match currency {
            // Deep negative rate environments
            Currency::EUR | Currency::JPY | Currency::CHF => Self {
                min_rate: -0.05,
                max_rate: 0.30,
            },
            // Standard developed markets
            Currency::USD | Currency::CAD | Currency::AUD | Currency::NZD => Self {
                min_rate: -0.02,
                max_rate: 0.50,
            },
            // GBP slightly wider negative
            Currency::GBP => Self {
                min_rate: -0.03,
                max_rate: 0.50,
            },
            // Emerging markets with potential for high rates
            Currency::TRY | Currency::ARS | Currency::BRL | Currency::ZAR | Currency::MXN => {
                Self::emerging_markets()
            }
            // Default: conservative developed market
            _ => Self::default(),
        }
    }

    /// Rate bounds for emerging markets with potential hyperinflation.
    ///
    /// Allows rates up to 200% to accommodate countries like Turkey and Argentina.
    pub fn emerging_markets() -> Self {
        Self {
            min_rate: -0.05,
            max_rate: 2.00, // 200%
        }
    }

    /// Rate bounds for negative rate environments.
    ///
    /// Optimized for EUR/JPY/CHF where deeply negative rates are common.
    pub fn negative_rate_environment() -> Self {
        Self {
            min_rate: -0.10, // -10%
            max_rate: 0.20,  // 20%
        }
    }

    /// Rate bounds for stress testing scenarios.
    ///
    /// Very wide bounds to allow extreme scenarios.
    pub fn stress_test() -> Self {
        Self {
            min_rate: -0.20, // -20%
            max_rate: 5.00,  // 500%
        }
    }

    /// Check if a rate is within bounds.
    #[inline]
    pub fn contains(&self, rate: f64) -> bool {
        rate >= self.min_rate && rate <= self.max_rate
    }

    /// Clamp a rate to be within bounds.
    #[inline]
    pub fn clamp(&self, rate: f64) -> f64 {
        rate.clamp(self.min_rate, self.max_rate)
    }
}

/// How `CalibrationConfig` obtains rate bounds.
///
/// Market-standard bounds depend on currency/market regime. `AutoCurrency` makes this choice
/// explicit and avoids relying on `RateBounds::default()` as an implicit assumption.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateBoundsPolicy {
    /// Pick currency-specific bounds via `RateBounds::for_currency(currency)`.
    #[default]
    AutoCurrency,
    /// Use the explicit `CalibrationConfig.rate_bounds` values.
    Explicit,
}

/// Runtime validation behavior for arbitrage/consistency checks.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
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
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
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

/// Calibration method selection (bootstrap vs global solve).
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CalibrationMethod {
    /// Traditional sequential bootstrap (default).
    #[default]
    Bootstrap,
    /// Global solve of all knots simultaneously (Newton/LM).
    GlobalSolve {
        /// Use analytical Jacobian if available (otherwise finite-difference).
        #[serde(default)]
        use_analytical_jacobian: bool,
    },
}

/// Multi-curve calibration configuration
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
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
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationConfig {
    /// Solver tolerance for convergence checks
    pub tolerance: f64,
    /// Maximum iterations for solver
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
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, string>"))]
    pub entity_seniority: HashMap<String, Seniority>,
    /// Multi-curve framework configuration
    pub multi_curve: MultiCurveConfig,
    /// Use finite-difference gradients for SABR calibration instead of analytical approximations.
    /// Analytical gradients are faster but use approximations (treating x(z) as ~constant).
    /// FD gradients are slower but more accurate, especially for far-from-ATM strikes.
    pub use_fd_sabr_gradients: bool,
    /// Explanation options (opt-in detailed trace)
    #[serde(skip)]
    #[cfg_attr(feature = "ts_export", ts(skip))]
    pub explain: ExplainOpts,
    /// Runtime validation mode (warnings vs errors). Feature `strict_validation` may still harden checks.
    pub validation_mode: ValidationMode,
    /// Validation configuration with thresholds and checks
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub validation: crate::calibration::validation::ValidationConfig,
    /// Policy for selecting rate bounds (explicit vs currency-derived).
    ///
    /// - `auto_currency` is market-standard: bounds depend on currency/market regime.
    /// - `explicit` uses `rate_bounds` as provided.
    #[serde(default = "default_rate_bounds_policy_for_serde")]
    pub rate_bounds_policy: RateBoundsPolicy,
    /// Rate bounds for forward/zero rate calibration.
    /// Currency-specific defaults can be set via `with_rate_bounds_for_currency()`.
    #[serde(default)]
    pub rate_bounds: RateBounds,
    /// Calibration method (bootstrap vs global solve).
    #[serde(default)]
    pub calibration_method: CalibrationMethod,
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
            validation_mode: ValidationMode::Error,
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::AutoCurrency,
            rate_bounds: RateBounds::default(),
            calibration_method: CalibrationMethod::default(),
        }
    }
}

impl CalibrationConfig {
    /// Resolve effective rate bounds for a given currency based on `rate_bounds_policy`.
    pub fn effective_rate_bounds(&self, currency: Currency) -> RateBounds {
        match self.rate_bounds_policy {
            RateBoundsPolicy::AutoCurrency => RateBounds::for_currency(currency),
            RateBoundsPolicy::Explicit => self.rate_bounds.clone(),
        }
    }

    /// Set multi-curve framework configuration.
    /// This is a convenience method for backward compatibility.
    #[must_use]
    pub fn with_multi_curve_config(mut self, multi_curve_config: MultiCurveConfig) -> Self {
        self.multi_curve = multi_curve_config;
        self
    }

    /// Create a configuration for multi-curve mode (standard).
    pub fn multi_curve() -> Self {
        Self::default()
    }

    /// Enable explanation tracing with default settings.
    #[must_use]
    pub fn with_explain(mut self) -> Self {
        self.explain = ExplainOpts::enabled();
        self
    }

    /// Set custom explanation options.
    #[must_use]
    pub fn with_explain_opts(mut self, opts: ExplainOpts) -> Self {
        self.explain = opts;
        self
    }

    /// Set custom rate bounds for calibration.
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_valuations::calibration::{CalibrationConfig, RateBounds};
    ///
    /// let config = CalibrationConfig::default()
    ///     .with_rate_bounds(RateBounds::emerging_markets());
    /// ```
    #[must_use]
    pub fn with_rate_bounds(mut self, bounds: RateBounds) -> Self {
        self.rate_bounds_policy = RateBoundsPolicy::Explicit;
        self.rate_bounds = bounds;
        self
    }

    /// Set the calibration method (bootstrap vs global solve).
    #[must_use]
    pub fn with_calibration_method(mut self, method: CalibrationMethod) -> Self {
        self.calibration_method = method;
        self
    }

    /// Set rate bounds appropriate for the given currency.
    ///
    /// Automatically selects bounds based on market conventions:
    /// - EUR/JPY/CHF: Extended negative rate support
    /// - TRY/ARS/BRL: Emerging market bounds (up to 200%)
    /// - USD/GBP/etc: Standard developed market bounds
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_valuations::calibration::CalibrationConfig;
    /// use finstack_core::currency::Currency;
    ///
    /// let config = CalibrationConfig::default()
    ///     .with_rate_bounds_for_currency(Currency::TRY);
    /// ```
    #[must_use]
    pub fn with_rate_bounds_for_currency(mut self, currency: Currency) -> Self {
        self.rate_bounds_policy = RateBoundsPolicy::Explicit;
        self.rate_bounds = RateBounds::for_currency(currency);
        self
    }

    /// Set the solver tolerance.
    #[must_use]
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set the maximum number of iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Enable or disable verbose logging.
    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Create a conservative calibration configuration.
    ///
    /// Conservative settings prioritize stability and robustness over speed:
    /// - Higher tolerance for better accuracy (1e-12)
    /// - Moderate iteration limit (100)
    /// - Wide rate bounds to handle edge cases
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
            validation_mode: ValidationMode::Error,
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds {
                min_rate: -0.05,
                max_rate: 1.00, // Allow up to 100% for conservative edge cases
            },
            calibration_method: CalibrationMethod::default(),
        }
    }

    /// Create an aggressive calibration configuration.
    ///
    /// Aggressive settings prioritize speed and will tolerate looser fits:
    /// - Relaxed tolerance for faster convergence (1e-6)
    /// - Higher iteration limit (1000) for difficult problems
    /// - Standard rate bounds
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
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds::default(),
            calibration_method: CalibrationMethod::default(),
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
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds::default(),
            calibration_method: CalibrationMethod::default(),
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
