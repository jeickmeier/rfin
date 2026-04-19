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
/// - `Bootstrap`: Traditional sequential bootstrap where each knot is solved
///   independently based on the previous knots.
/// - `GlobalSolve`: Simultaneous optimization of all knots using Levenberg-Marquardt
///   or Newton-Raphson.
///
/// # Examples
/// ```rust
/// use finstack_valuations::calibration::CalibrationMethod;
///
/// let method = CalibrationMethod::GlobalSolve { use_analytical_jacobian: true };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
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

impl std::fmt::Display for CalibrationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bootstrap => write!(f, "bootstrap"),
            Self::GlobalSolve { .. } => write!(f, "global_solve"),
        }
    }
}

impl std::str::FromStr for CalibrationMethod {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase().replace(['-', '/', ' '], "_");
        match normalized.as_str() {
            "bootstrap" => Ok(Self::Bootstrap),
            "global_solve" | "globalsolve" => Ok(Self::GlobalSolve {
                use_analytical_jacobian: false,
            }),
            other => Err(format!(
                "Unknown calibration method: '{}'. Valid: bootstrap, global_solve",
                other
            )),
        }
    }
}

/// Policy for weighting residuals in global solve calibration.
///
/// Determines how the objective function weights individual instrument fitting
/// errors (residuals) during optimization.
///
/// # Variants
/// - `Equal`: Every instrument contributes equally to the objective.
/// - `LinearTime`: Weights increase linearly with time to maturity.
/// - `SqrtTime`: Weights increase with the square root of time (market-standard).
/// - `InverseDuration`: Weights based on inverse DV01 approximation.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

impl std::fmt::Display for ResidualWeightingScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal => write!(f, "equal"),
            Self::LinearTime => write!(f, "linear_time"),
            Self::SqrtTime => write!(f, "sqrt_time"),
            Self::InverseDuration => write!(f, "inverse_duration"),
        }
    }
}

impl std::str::FromStr for ResidualWeightingScheme {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase().replace(['-', '/', ' '], "_");
        match normalized.as_str() {
            "equal" => Ok(Self::Equal),
            "linear_time" | "lineartime" => Ok(Self::LinearTime),
            "sqrt_time" | "sqrttime" => Ok(Self::SqrtTime),
            "inverse_duration" | "inverseduration" => Ok(Self::InverseDuration),
            other => Err(format!(
                "Unknown residual weighting scheme: '{}'. Valid: equal, linear_time, sqrt_time, inverse_duration",
                other
            )),
        }
    }
}

/// Hazard-curve specific numerical solver configuration.
///
/// Controls the search space and numerical stability of hazard curve
/// bootstrapping or global solve for credit curve calibration.
///
/// # Invariants
/// - `hazard_hard_min` >= 0 (hazard rates must be non-negative)
/// - `hazard_hard_max` > `hazard_hard_min`
///
/// # Examples
/// ```rust,no_run
/// use finstack_valuations::calibration::HazardCurveSolveConfig;
///
/// // For distressed debt scenarios, increase the max hazard rate
/// let config = HazardCurveSolveConfig {
///     hazard_hard_max: 100.0,  // Allow up to ~100% default probability per year
///     ..Default::default()
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HazardCurveSolveConfig {
    /// Minimum allowed hazard rate (must be non-negative for survival monotonicity).
    pub hazard_hard_min: f64,
    /// Maximum allowed hazard rate.
    ///
    /// The default of 10.0 corresponds to roughly 99.995% 1Y default probability.
    /// For distressed/distressed sovereign scenarios, increase to 50.0 or 100.0.
    pub hazard_hard_max: f64,
    /// Weighting scheme for global solve residuals.
    #[serde(default)]
    pub weighting_scheme: ResidualWeightingScheme,
    /// Tolerance for determining calibration *success* (applied to residuals).
    ///
    /// After the solver converges, the final residuals are compared against this
    /// tolerance. If `max_residual > validation_tolerance`, the calibration report
    /// will have `success = false` even if the solver converged.
    ///
    /// This is distinct from `solver.tolerance()` which controls when the numerical
    /// solver terminates. See [`CalibrationConfig`] for a full explanation.
    ///
    /// Default: `1e-8` (suitable for per-unit-notional residuals).
    pub validation_tolerance: f64,
}

impl Default for HazardCurveSolveConfig {
    fn default() -> Self {
        Self {
            hazard_hard_min: 0.0,
            hazard_hard_max: 10.0, // ~99.995% 1Y default probability
            weighting_scheme: ResidualWeightingScheme::default(),
            validation_tolerance: 1e-8,
        }
    }
}

impl HazardCurveSolveConfig {
    /// Create a configuration suitable for distressed debt scenarios.
    ///
    /// Increases the hazard rate cap to 100.0, allowing calibration of
    /// credit curves for entities with very high default probabilities
    /// (e.g., distressed sovereigns, near-default corporates).
    pub fn distressed() -> Self {
        Self {
            hazard_hard_max: 100.0,
            validation_tolerance: 1e-6, // Slightly relaxed for distressed
            ..Self::default()
        }
    }
}

/// Inflation-curve specific numerical solver configuration.
///
/// Controls the numerical stability and success criteria of inflation curve
/// bootstrapping or global solve for CPI curve calibration.
///
/// # Examples
/// ```rust,no_run
/// use finstack_valuations::calibration::InflationCurveSolveConfig;
///
/// let config = InflationCurveSolveConfig {
///     validation_tolerance: 1e-6,  // Relax for noisy inflation markets
///     ..Default::default()
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InflationCurveSolveConfig {
    /// Weighting scheme for global solve residuals.
    #[serde(default)]
    pub weighting_scheme: ResidualWeightingScheme,
    /// Tolerance for determining calibration *success* (applied to residuals).
    ///
    /// After the solver converges, the final residuals are compared against this
    /// tolerance. If `max_residual > validation_tolerance`, the calibration report
    /// will have `success = false` even if the solver converged.
    ///
    /// This is distinct from `solver.tolerance()` which controls when the numerical
    /// solver terminates. See [`CalibrationConfig`] for a full explanation.
    ///
    /// Default: `1e-8` (suitable for per-unit-notional residuals).
    pub validation_tolerance: f64,
    /// Minimum allowed CPI level during calibration.
    ///
    /// Default: `1.0`. For indices with different base conventions or rebased
    /// indices, adjust accordingly.
    pub cpi_hard_min: f64,
    /// Maximum allowed CPI level during calibration.
    ///
    /// Default: `10_000.0`. For hyperinflation currencies (TRY, ARS, VES),
    /// increase this bound significantly (e.g. `1e9`).
    pub cpi_hard_max: f64,
}

impl Default for InflationCurveSolveConfig {
    fn default() -> Self {
        Self {
            weighting_scheme: ResidualWeightingScheme::default(),
            validation_tolerance: 1e-8,
            cpi_hard_min: 1.0,
            cpi_hard_max: 10_000.0,
        }
    }
}

impl InflationCurveSolveConfig {
    /// Configuration suitable for hyperinflation currencies (TRY, ARS, VES, etc.).
    ///
    /// Widens CPI bounds to accommodate very high index levels and relaxes
    /// the validation tolerance for noisier inflation markets.
    pub fn hyperinflation() -> Self {
        Self {
            validation_tolerance: 1e-6,
            cpi_hard_min: 0.01,
            cpi_hard_max: 1e9,
            ..Self::default()
        }
    }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Tolerance for determining calibration *success* (applied to residuals).
    ///
    /// After the solver converges, the final residuals are compared against this
    /// tolerance. If `max_residual > validation_tolerance`, the calibration report
    /// will have `success = false` even if the solver converged.
    ///
    /// This is distinct from `solver.tolerance()` which controls when the numerical
    /// solver terminates. See [`CalibrationConfig`] for a full explanation.
    ///
    /// Default: `1e-8` (suitable for per-unit-notional residuals).
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
/// Public callers should treat it as the behavioral contract for calibration
/// execution policy: solver choice, convergence settings, validation thresholds,
/// rate-bound policy, and curve-specific numerical guardrails.
///
/// # Tolerance Semantics
///
/// Calibration involves two distinct tolerance concepts:
///
/// 1. **Solver Tolerance** ([`solver.tolerance()`](SolverConfig::tolerance)):
///    Controls when the numerical solver (Brent/Newton) terminates. This is an
///    algorithmic convergence criterion in x-space (parameter space). The solver
///    stops when successive parameter estimates differ by less than this tolerance.
///
/// 2. **Validation Tolerance** (e.g., [`discount_curve.validation_tolerance`](DiscountCurveSolveConfig::validation_tolerance)):
///    Controls whether calibration is considered *successful*. After the solver
///    converges, the final residuals are compared against this tolerance. If any
///    residual exceeds `validation_tolerance`, the calibration is marked as failed
///    even if the solver converged.
///
/// **Why two tolerances?**
/// - Solver tolerance ensures numerical convergence but doesn't guarantee economic fit.
/// - Validation tolerance ensures the calibrated curve actually prices instruments correctly.
/// - For well-behaved problems, solver tolerance of `1e-12` with validation tolerance of
///   `1e-8` works well: the solver finds a precise root, and we verify it prices accurately.
///
/// # Configuration Hierarchy
///
/// Settings can be specified at multiple levels with the following precedence:
///
/// 1. **Step-level** (`CalibrationStep.params.method`): Per-instrument-type overrides
/// 2. **Plan-level** (`CalibrationPlan.settings`): Plan-wide defaults
/// 3. **Global defaults** (`CalibrationConfig::default()`): Fallback values
///
/// Step-level settings always take precedence over plan-level settings.
/// In other words, this struct provides default policy, but explicit plan steps
/// remain authoritative when both are supplied.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::calibration::CalibrationConfig;
///
/// // Create a default config
/// let config = CalibrationConfig::default();
///
/// // Create a conservative config for high-precision risk systems
/// let conservative = CalibrationConfig::conservative();
///
/// // Customize tolerance settings
/// let custom = CalibrationConfig::default()
///     .with_tolerance(1e-14)  // Solver convergence tolerance
///     .with_max_iterations(200);
/// ```
///
/// # References
///
/// - Multi-curve construction context: `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`
/// - Curve interpolation context: `docs/REFERENCES.md#hagan-west-monotone-convex`
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct CalibrationConfig {
    /// Solver configuration including numerical method (e.g., Brent) and parameters (tolerance, iterations).
    #[schemars(with = "serde_json::Value")]
    pub solver: SolverConfig,
    /// Use parallel processing when available (e.g., for independent curves).
    pub use_parallel: bool,
    /// Enable verbose logging of the calibration process.
    pub verbose: bool,
    /// Explanation options (opt-in detailed trace for debugging).
    #[serde(skip)]
    #[cfg_attr(feature = "ts_export", ts(skip))]
    #[schemars(with = "serde_json::Value")]
    pub explain: ExplainOpts,
    /// Runtime validation mode (warnings vs errors).
    #[schemars(with = "serde_json::Value")]
    pub validation_mode: ValidationMode,
    /// Validation configuration with thresholds and quality checks.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown>"))]
    #[schemars(with = "serde_json::Value")]
    pub validation: crate::calibration::validation::ValidationConfig,
    /// Policy for selecting rate bounds (explicit vs currency-derived).
    #[serde(default = "crate::calibration::validation::default_rate_bounds_policy_for_serde")]
    #[schemars(with = "serde_json::Value")]
    pub rate_bounds_policy: RateBoundsPolicy,
    /// Rate bounds for forward/zero rate calibration (when policy is `Explicit`).
    #[serde(default)]
    #[schemars(with = "serde_json::Value")]
    pub rate_bounds: RateBounds,
    /// High-level calibration method (bootstrap vs global solve).
    ///
    /// **Note**: When using the plan-driven API, this field is typically overwritten
    /// by the step-level `params.method` for each calibration step. The step-level
    /// method always takes precedence. This field serves as runtime state passed
    /// from calibration targets to the underlying solvers.
    #[serde(default)]
    pub calibration_method: CalibrationMethod,

    /// Whether to compute detailed calibration diagnostics (condition number,
    /// per-quote quality metrics, singular values, R-squared, etc.).
    ///
    /// When `true`, the solver will perform additional post-solve analysis
    /// including Jacobian-based condition number estimation. This adds
    /// computational overhead and should typically be disabled in production
    /// hot paths but enabled for calibration debugging, auditing, or
    /// quality monitoring.
    ///
    /// Default: `false`.
    #[serde(default)]
    pub compute_diagnostics: bool,

    /// Discount-curve specific solver configuration.
    #[serde(default)]
    #[schemars(with = "serde_json::Value")]
    pub discount_curve: DiscountCurveSolveConfig,

    /// Hazard-curve specific solver configuration.
    #[serde(default)]
    #[schemars(with = "serde_json::Value")]
    pub hazard_curve: HazardCurveSolveConfig,

    /// Inflation-curve specific solver configuration.
    #[serde(default)]
    #[schemars(with = "serde_json::Value")]
    pub inflation_curve: InflationCurveSolveConfig,
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
            compute_diagnostics: false,
            discount_curve: DiscountCurveSolveConfig::default(),
            hazard_curve: HazardCurveSolveConfig::default(),
            inflation_curve: InflationCurveSolveConfig::default(),
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

    /// Enable or disable detailed calibration diagnostics.
    ///
    /// When enabled, the solver will compute condition number, per-quote
    /// quality metrics, and other diagnostic information after calibration.
    #[must_use]
    pub fn with_compute_diagnostics(mut self, enabled: bool) -> Self {
        self.compute_diagnostics = enabled;
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
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            rate_bounds: RateBounds {
                min_rate: -0.05,
                max_rate: 1.00, // Allow up to 100% for conservative edge cases
            },
            ..Self::default()
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
            validation_mode: ValidationMode::Warn,
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            ..Self::default()
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
            validation_mode: ValidationMode::Warn,
            rate_bounds_policy: RateBoundsPolicy::Explicit,
            ..Self::default()
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatesStepConventions {
    /// Day count used to map dates to year fractions for curve knot times.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub curve_day_count: Option<finstack_core::dates::DayCount>,
}

// =============================================================================
// End of configuration module

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn residual_weighting_scheme_fromstr_display_roundtrip() {
        fn assert_residual_weighting_scheme(label: &str, expected: ResidualWeightingScheme) {
            assert!(
                matches!(ResidualWeightingScheme::from_str(label), Ok(value) if value == expected)
            );
        }

        let variants = [
            ResidualWeightingScheme::Equal,
            ResidualWeightingScheme::LinearTime,
            ResidualWeightingScheme::SqrtTime,
            ResidualWeightingScheme::InverseDuration,
        ];
        for v in variants {
            let s = v.to_string();
            let parsed =
                ResidualWeightingScheme::from_str(&s).expect("roundtrip parse should succeed");
            assert_eq!(v, parsed, "roundtrip failed for {s}");
        }
        // Test aliases
        assert_residual_weighting_scheme("lineartime", ResidualWeightingScheme::LinearTime);
        assert_residual_weighting_scheme("sqrttime", ResidualWeightingScheme::SqrtTime);
        assert!(ResidualWeightingScheme::from_str("invalid").is_err());
    }

    #[test]
    fn calibration_method_fromstr_display_roundtrip() {
        fn assert_calibration_method(label: &str, matcher: fn(&CalibrationMethod) -> bool) {
            let parsed = CalibrationMethod::from_str(label);
            assert!(matches!(parsed.as_ref(), Ok(value) if matcher(value)));
        }

        // Bootstrap roundtrips exactly
        let bootstrap = CalibrationMethod::Bootstrap;
        let s = bootstrap.to_string();
        let parsed = CalibrationMethod::from_str(&s).expect("roundtrip parse should succeed");
        assert_eq!(bootstrap, parsed);

        // GlobalSolve parses to default (use_analytical_jacobian = false)
        assert_calibration_method("global_solve", |value| {
            matches!(
                value,
                CalibrationMethod::GlobalSolve {
                    use_analytical_jacobian: false
                }
            )
        });

        // Alias
        assert_calibration_method("globalsolve", |value| {
            matches!(value, CalibrationMethod::GlobalSolve { .. })
        });

        assert!(CalibrationMethod::from_str("invalid").is_err());
    }
}
