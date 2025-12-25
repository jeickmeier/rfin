//! Calibration configuration and solver selection.
//!
//! This module provides unified configuration for calibration processes including:
//! - Solver selection and numerical parameters
//! - Rate bounds for market-regime-aware calibration
//! - Validation and explainability settings

use crate::calibration::solver::SolverConfig;
use crate::calibration::validation::{RateBounds, RateBoundsPolicy, ValidationMode};
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::explain::ExplainOpts;

use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Calibration method selection (bootstrap vs global solve).
///
/// Defines the numerical approach used to solve for curve/surface parameters.
/// Bootstrap is the traditional sequential approach, while GlobalSolve
/// solves all parameters simultaneously.
///
/// # Variants
/// - [`Bootstrap`]: Traditional sequential bootstrap where each knot is solved
///   independently based on the previous knots.
/// - [`GlobalSolve`]: Simultaneous optimization of all knots using Levenberg-Marquardt
///   or Newton-Raphson.
///
/// # Examples
/// ```rust
/// use finstack_valuations::calibration::api::schema::CalibrationMethod;
///
/// let method = CalibrationMethod::GlobalSolve { use_analytical_jacobian: true };
/// ```
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

/// Policy for weighting residuals in global solve calibration.
///
/// Determines how the objective function weights individual instrument fitting
/// errors (residuals) during optimization.
///
/// # Variants
/// - [`Equal`]: Every instrument contributes equally to the objective.
/// - [`LinearTime`]: Weights increase linearly with time to maturity.
/// - [`SqrtTime`]: Weights increase with the square root of time (market-standard).
/// - [`InverseDuration`]: Weights based on inverse DV01 approximation.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ResidualWeightingScheme {
    /// Equal weighting (1.0 for all quotes).
    Equal,
    /// Weight by time to maturity (t).
    LinearTime,
    /// Weight by square root of time (sqrt(t)).
    #[default]
    SqrtTime,
    /// Weight by inverse duration (1/DV01 approximation).
    InverseDuration,
}

/// Discount-curve specific numerical solver configuration.
///
/// Controls the search space and numerical stability of the discount curve
/// bootstrapping or global solve process.
///
/// # Invariants
/// - `df_hard_min` > 0
/// - `scan_grid_points` > 0
///
/// # Examples
/// ```rust,no_run
/// use finstack_valuations::calibration::DiscountCurveSolveConfig;
///
/// let config = DiscountCurveSolveConfig {
///     scan_grid_points: 64,
///     df_hard_min: 1e-10,
///     ..Default::default()
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiscountCurveSolveConfig {
    /// Number of points in the initial geometric scan grid.
    pub scan_grid_points: usize,
    /// Minimum required success points in scan before attempting polish.
    pub min_scan_grid_points: usize,
    /// Initial step size for geometric scan grid.
    pub scan_grid_step: f64,
    /// Absolute minimum allowed discount factor (prevents singularity).
    pub df_hard_min: f64,
    /// Absolute maximum allowed discount factor (prevents divergence).
    pub df_hard_max: f64,
    /// Minimum time threshold for considering a knot at spot (t=0).
    pub min_t_spot: f64,
    /// Interpolation style for the constructed curve.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(skip))]
    pub interp_style: InterpStyle,
    /// Extrapolation policy for the constructed curve.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(skip))]
    pub extrapolation_policy: ExtrapolationPolicy,
    /// Whether to use a sequential bootstrap to seed a global solve.
    pub bootstrap_seed_global_solve: bool,
    /// Override final-curve monotonicity enforcement (None = policy-driven).
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "boolean | null"))]
    pub allow_non_monotonic_final: Option<bool>,
    /// Weighting scheme for global solve residuals.
    #[serde(default)]
    pub weighting_scheme: ResidualWeightingScheme,
    /// Step size (h) for finite-difference Jacobian calculation.
    #[serde(default)]
    pub jacobian_step_size: f64,
    /// Tolerance for determining calibration success (applied to residuals).
    pub validation_tolerance: f64,
}

impl Default for DiscountCurveSolveConfig {
    fn default() -> Self {
        Self {
            scan_grid_points: 48,
            min_scan_grid_points: 20,
            scan_grid_step: 1e-4,
            interp_style: InterpStyle::default(),
            extrapolation_policy: ExtrapolationPolicy::default(),
            df_hard_min: 1e-12,
            df_hard_max: 1e6,
            min_t_spot: 1e-6,
            bootstrap_seed_global_solve: true,
            allow_non_monotonic_final: None,
            weighting_scheme: ResidualWeightingScheme::default(),
            // `jacobian_step_size` is used as a *relative* bump size (h = max(eps, |p|*eps)).
            // For typical discount-curve zero rates (~1-5%), `1e-8` can make PV differences
            // fall into numerical noise and cause GlobalSolve to stall (e.g. StepTooSmall).
            jacobian_step_size: 1e-6,
            // Validation success tolerance for residuals (per notional).
            validation_tolerance: 1e-8,
        }
    }
}

/// Global configuration for the calibration subsystem.
///
/// This struct consolidates all settings for solvers, validation, and market-regime
/// specific bounds. It is typically derived from a `FinstackConfig` extension section.
///
/// # Examples
/// ```rust
/// use finstack_valuations::calibration::CalibrationConfig;
///
/// // Create a default config
/// let config = CalibrationConfig::default();
///
/// // Create a conservative config for high-precision risk systems
/// let conservative = CalibrationConfig::conservative();
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CalibrationConfig {
    /// Solver configuration including numerical method (e.g., Brent) and parameters (tolerance, iterations).
    pub solver: SolverConfig,
    /// Use parallel processing when available (e.g., for independent curves).
    pub use_parallel: bool,
    /// Enable verbose logging of the calibration process.
    pub verbose: bool,
    /// Explanation options (opt-in detailed trace for debugging).
    #[serde(skip)]
    #[cfg_attr(feature = "ts_export", ts(skip))]
    pub explain: ExplainOpts,
    /// Runtime validation mode (warnings vs errors).
    pub validation_mode: ValidationMode,
    /// Validation configuration with thresholds and quality checks.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    pub validation: crate::calibration::validation::ValidationConfig,
    /// Policy for selecting rate bounds (explicit vs currency-derived).
    #[serde(default = "crate::calibration::validation::default_rate_bounds_policy_for_serde")]
    pub rate_bounds_policy: RateBoundsPolicy,
    /// Rate bounds for forward/zero rate calibration (when policy is `Explicit`).
    #[serde(default)]
    pub rate_bounds: RateBounds,
    /// High-level calibration method (bootstrap vs global solve).
    #[serde(default)]
    pub calibration_method: CalibrationMethod,

    /// Discount-curve specific solver configuration.
    #[serde(default)]
    pub discount_curve: DiscountCurveSolveConfig,
}

/// Extension section key for calibration overrides.
pub const CALIBRATION_CONFIG_KEY: &str = "valuations.calibration";

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            solver: SolverConfig::default(),
            use_parallel: false, // Deterministic by default
            verbose: false,
            explain: ExplainOpts::default(), // Disabled by default (zero overhead)
            validation_mode: ValidationMode::Error,
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::AutoCurrency,
            rate_bounds: RateBounds::default(),
            calibration_method: CalibrationMethod::default(),
            discount_curve: DiscountCurveSolveConfig::default(),
        }
    }
}

impl CalibrationConfig {
    /// Build a calibration config from a `FinstackConfig` extension section.
    ///
    /// If the extension section `valuations.calibration` is present, its
    /// fields override the defaults; otherwise defaults are used.
    ///
    /// # Errors
    ///
    /// Returns an error if the extension section is present but malformed
    /// (e.g., unknown fields when `deny_unknown_fields` is enforced).
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::config::FinstackConfig;
    /// use finstack_valuations::calibration::CalibrationConfig;
    ///
    /// let cfg = FinstackConfig::default();
    /// let calib_cfg = CalibrationConfig::from_finstack_config_or_default(&cfg)
    ///     .expect("valid config");
    /// assert_eq!(calib_cfg.solver.tolerance(), 1e-12); // default
    /// ```
    #[cfg(feature = "serde")]
    pub fn from_finstack_config_or_default(cfg: &FinstackConfig) -> finstack_core::Result<Self> {
        if let Some(raw) = cfg.extensions.get(CALIBRATION_CONFIG_KEY) {
            // Deserialize directly into CalibrationConfig; missing fields use defaults via #[serde(default)]
            serde_json::from_value(raw.clone()).map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Failed to parse extension '{}': {}",
                    CALIBRATION_CONFIG_KEY, e
                ),
                category: "config".to_string(),
            })
        } else {
            Ok(Self::default())
        }
    }

    /// Build a calibration config from a `FinstackConfig` (non-serde fallback).
    ///
    /// When the `serde` feature is disabled, extensions are not available and
    /// this method always returns `CalibrationConfig::default()`.
    #[cfg(not(feature = "serde"))]
    pub fn from_finstack_config_or_default(_cfg: &FinstackConfig) -> finstack_core::Result<Self> {
        Ok(Self::default())
    }

    /// Resolve effective rate bounds for a given currency based on `rate_bounds_policy`.
    pub fn effective_rate_bounds(&self, currency: Currency) -> RateBounds {
        match self.rate_bounds_policy {
            RateBoundsPolicy::AutoCurrency => RateBounds::for_currency(currency),
            RateBoundsPolicy::Explicit => self.rate_bounds.clone(),
        }
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
        self.solver = self.solver.with_tolerance(tolerance);
        self
    }

    /// Set the maximum number of iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.solver = self.solver.with_max_iterations(max_iterations);
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
    /// assert_eq!(config.solver.tolerance(), 1e-12);
    /// ```
    pub fn conservative() -> Self {
        Self {
            solver: SolverConfig::brent_default()
                .with_tolerance(1e-12)
                .with_max_iterations(100),
            use_parallel: false, // Deterministic
            verbose: false,
            explain: ExplainOpts::default(),
            validation_mode: ValidationMode::Error,
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds {
                min_rate: -0.05,
                max_rate: 1.00, // Allow up to 100% for conservative edge cases
            },
            calibration_method: CalibrationMethod::default(),
            discount_curve: DiscountCurveSolveConfig::default(),
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
    /// assert_eq!(config.solver.max_iterations(), 1000);
    /// ```
    pub fn aggressive() -> Self {
        Self {
            solver: SolverConfig::brent_default()
                .with_tolerance(1e-6)
                .with_max_iterations(1000),
            use_parallel: false, // Keep deterministic by default
            verbose: false,
            explain: ExplainOpts::default(),
            validation_mode: ValidationMode::Warn,
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds::default(),
            calibration_method: CalibrationMethod::default(),
            discount_curve: DiscountCurveSolveConfig::default(),
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
    /// assert_eq!(config.solver.max_iterations(), 50);
    /// ```
    pub fn fast() -> Self {
        Self {
            solver: SolverConfig::brent_default()
                .with_tolerance(1e-4)
                .with_max_iterations(50),
            use_parallel: false,
            verbose: false,
            explain: ExplainOpts::default(),
            validation_mode: ValidationMode::Warn,
            validation: crate::calibration::validation::ValidationConfig::default(),
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds::default(),
            calibration_method: CalibrationMethod::default(),
            discount_curve: DiscountCurveSolveConfig::default(),
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
            .with_tolerance(self.solver.tolerance())
            .with_max_iterations(self.solver.max_iterations())
    }
}

/// Step-level conventions for rates calibration (discount and forward curves).
///
/// This is a Bloomberg/FinCad-style design: curve construction uses a small set of
/// *step-level* conventions (e.g., curve time-axis day count).
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatesStepConventions {
    /// Day count used to map dates to year fractions for curve knot times.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub curve_day_count: Option<finstack_core::dates::DayCount>,
}
