//! Calibration configuration types for WASM bindings.

use crate::core::currency::JsCurrency;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationMethod, RateBounds, RateBoundsPolicy, ResidualWeightingScheme,
    SolverConfig, ValidationMode,
};
use wasm_bindgen::prelude::*;

/// Solver strategy enumeration for calibration routines.
#[wasm_bindgen(js_name = SolverKind)]
#[derive(Clone, Debug)]
pub struct JsSolverKind {
    inner: SolverConfig,
}

impl JsSolverKind {
    pub(crate) fn from_inner(inner: SolverConfig) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> SolverConfig {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = SolverKind)]
impl JsSolverKind {
    /// Newton-Raphson solver.
    #[wasm_bindgen(js_name = Newton)]
    pub fn newton() -> Self {
        Self {
            inner: SolverConfig::newton_default(),
        }
    }

    /// Brent's method (bracketing).
    #[wasm_bindgen(js_name = Brent)]
    pub fn brent() -> Self {
        Self {
            inner: SolverConfig::brent_default(),
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsSolverKind, JsValue> {
        let normalized = name.to_ascii_lowercase().replace('-', "_");
        let kind = match normalized.as_str() {
            "newton" => SolverConfig::newton_default(),
            "brent" => SolverConfig::brent_default(),
            _ => return Err(JsValue::from_str(&format!("Unknown solver kind: {}", name))),
        };
        Ok(Self { inner: kind })
    }

    /// Get the string name of the solver kind.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        match self.inner {
            SolverConfig::Newton { .. } => "newton".to_string(),
            SolverConfig::Brent { .. } => "brent".to_string(),
        }
    }
}

// =============================================================================
// CalibrationMethod
// =============================================================================

/// Calibration method selection (bootstrap vs global solve).
#[wasm_bindgen(js_name = CalibrationMethod)]
#[derive(Clone, Debug)]
pub struct JsCalibrationMethod {
    inner: CalibrationMethod,
}

impl JsCalibrationMethod {
    pub(crate) fn from_inner(inner: CalibrationMethod) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> CalibrationMethod {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CalibrationMethod)]
impl JsCalibrationMethod {
    /// Traditional sequential bootstrap (default).
    #[wasm_bindgen(js_name = Bootstrap)]
    pub fn bootstrap() -> JsCalibrationMethod {
        Self {
            inner: CalibrationMethod::Bootstrap,
        }
    }

    /// Global solve of all knots simultaneously using Levenberg-Marquardt.
    #[wasm_bindgen(js_name = GlobalSolve)]
    pub fn global_solve(use_analytical_jacobian: bool) -> JsCalibrationMethod {
        Self {
            inner: CalibrationMethod::GlobalSolve {
                use_analytical_jacobian,
            },
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsCalibrationMethod, JsValue> {
        let normalized = name.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "bootstrap" => Ok(Self::bootstrap()),
            "global_solve" | "globalsolve" | "global" => Ok(Self::global_solve(false)),
            _ => Err(JsValue::from_str(&format!(
                "Unknown calibration method: {}",
                name
            ))),
        }
    }

    /// Get the string name of the method.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        match &self.inner {
            CalibrationMethod::Bootstrap => "bootstrap".to_string(),
            CalibrationMethod::GlobalSolve { .. } => "global_solve".to_string(),
        }
    }

    /// Whether this is a global solve method.
    #[wasm_bindgen(getter, js_name = isGlobalSolve)]
    pub fn is_global_solve(&self) -> bool {
        matches!(self.inner, CalibrationMethod::GlobalSolve { .. })
    }
}

// =============================================================================
// ResidualWeightingScheme
// =============================================================================

/// Policy for weighting residuals in global solve calibration.
#[wasm_bindgen(js_name = ResidualWeightingScheme)]
#[derive(Clone, Debug)]
pub struct JsResidualWeightingScheme {
    inner: ResidualWeightingScheme,
}

impl JsResidualWeightingScheme {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: ResidualWeightingScheme) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> ResidualWeightingScheme {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ResidualWeightingScheme)]
impl JsResidualWeightingScheme {
    /// Equal weighting (1.0 for all quotes).
    #[wasm_bindgen(js_name = Equal)]
    pub fn equal() -> JsResidualWeightingScheme {
        Self {
            inner: ResidualWeightingScheme::Equal,
        }
    }

    /// Weight by time to maturity (t).
    #[wasm_bindgen(js_name = LinearTime)]
    pub fn linear_time() -> JsResidualWeightingScheme {
        Self {
            inner: ResidualWeightingScheme::LinearTime,
        }
    }

    /// Weight by square root of time (sqrt(t)) - market standard.
    #[wasm_bindgen(js_name = SqrtTime)]
    pub fn sqrt_time() -> JsResidualWeightingScheme {
        Self {
            inner: ResidualWeightingScheme::SqrtTime,
        }
    }

    /// Weight by inverse duration (1/DV01 approximation).
    #[wasm_bindgen(js_name = InverseDuration)]
    pub fn inverse_duration() -> JsResidualWeightingScheme {
        Self {
            inner: ResidualWeightingScheme::InverseDuration,
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsResidualWeightingScheme, JsValue> {
        let normalized = name.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "equal" => Ok(Self::equal()),
            "linear_time" | "lineartime" => Ok(Self::linear_time()),
            "sqrt_time" | "sqrttime" => Ok(Self::sqrt_time()),
            "inverse_duration" | "inverseduration" => Ok(Self::inverse_duration()),
            _ => Err(JsValue::from_str(&format!(
                "Unknown weighting scheme: {}",
                name
            ))),
        }
    }

    /// Get the string name.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        match &self.inner {
            ResidualWeightingScheme::Equal => "equal".to_string(),
            ResidualWeightingScheme::LinearTime => "linear_time".to_string(),
            ResidualWeightingScheme::SqrtTime => "sqrt_time".to_string(),
            ResidualWeightingScheme::InverseDuration => "inverse_duration".to_string(),
        }
    }
}

// =============================================================================
// RateBounds
// =============================================================================

/// Configurable bounds for forward/zero rates during calibration.
#[wasm_bindgen(js_name = RateBounds)]
#[derive(Clone, Debug)]
pub struct JsRateBounds {
    inner: RateBounds,
}

impl JsRateBounds {
    pub(crate) fn from_inner(inner: RateBounds) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> RateBounds {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RateBounds)]
impl JsRateBounds {
    /// Create new rate bounds.
    #[wasm_bindgen(constructor)]
    pub fn new(min_rate: f64, max_rate: f64) -> Result<JsRateBounds, JsValue> {
        RateBounds::new(min_rate, max_rate)
            .map(JsRateBounds::from_inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Default bounds for developed markets: [-2%, 50%].
    #[wasm_bindgen(js_name = default)]
    pub fn default_bounds() -> JsRateBounds {
        Self {
            inner: RateBounds::default(),
        }
    }

    /// Bounds for a specific currency based on market conventions.
    #[wasm_bindgen(js_name = forCurrency)]
    pub fn for_currency(currency: &JsCurrency) -> JsRateBounds {
        Self {
            inner: RateBounds::for_currency(currency.inner()),
        }
    }

    /// Bounds for emerging markets with potential hyperinflation: [-5%, 200%].
    #[wasm_bindgen(js_name = emergingMarkets)]
    pub fn emerging_markets() -> JsRateBounds {
        Self {
            inner: RateBounds::emerging_markets(),
        }
    }

    /// Bounds for negative rate environments (EUR/JPY/CHF): [-10%, 20%].
    #[wasm_bindgen(js_name = negativeRateEnvironment)]
    pub fn negative_rate_environment() -> JsRateBounds {
        Self {
            inner: RateBounds::negative_rate_environment(),
        }
    }

    /// Very wide bounds for stress testing: [-20%, 500%].
    #[wasm_bindgen(js_name = stressTest)]
    pub fn stress_test() -> JsRateBounds {
        Self {
            inner: RateBounds::stress_test(),
        }
    }

    /// Minimum allowed rate.
    #[wasm_bindgen(getter, js_name = minRate)]
    pub fn min_rate(&self) -> f64 {
        self.inner.min_rate
    }

    /// Maximum allowed rate.
    #[wasm_bindgen(getter, js_name = maxRate)]
    pub fn max_rate(&self) -> f64 {
        self.inner.max_rate
    }

    /// Check if a rate is within bounds.
    pub fn contains(&self, rate: f64) -> bool {
        self.inner.contains(rate)
    }

    /// Clamp a rate to be within bounds.
    pub fn clamp(&self, rate: f64) -> f64 {
        self.inner.clamp(rate)
    }
}

// =============================================================================
// RateBoundsPolicy
// =============================================================================

/// How CalibrationConfig obtains rate bounds.
#[wasm_bindgen(js_name = RateBoundsPolicy)]
#[derive(Clone, Debug)]
pub struct JsRateBoundsPolicy {
    inner: RateBoundsPolicy,
}

impl JsRateBoundsPolicy {
    pub(crate) fn from_inner(inner: RateBoundsPolicy) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> RateBoundsPolicy {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RateBoundsPolicy)]
impl JsRateBoundsPolicy {
    /// Pick currency-specific bounds automatically.
    #[wasm_bindgen(js_name = AutoCurrency)]
    pub fn auto_currency() -> JsRateBoundsPolicy {
        Self {
            inner: RateBoundsPolicy::AutoCurrency,
        }
    }

    /// Use explicitly provided rate bounds.
    #[wasm_bindgen(js_name = Explicit)]
    pub fn explicit() -> JsRateBoundsPolicy {
        Self {
            inner: RateBoundsPolicy::Explicit,
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsRateBoundsPolicy, JsValue> {
        let normalized = name.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "auto_currency" | "autocurrency" | "auto" => Ok(Self::auto_currency()),
            "explicit" => Ok(Self::explicit()),
            _ => Err(JsValue::from_str(&format!(
                "Unknown rate bounds policy: {}",
                name
            ))),
        }
    }

    /// Get the string name.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        match &self.inner {
            RateBoundsPolicy::AutoCurrency => "auto_currency".to_string(),
            RateBoundsPolicy::Explicit => "explicit".to_string(),
        }
    }
}

// =============================================================================
// ValidationMode
// =============================================================================

/// Runtime validation behavior for arbitrage/consistency checks.
#[wasm_bindgen(js_name = ValidationMode)]
#[derive(Clone, Debug)]
pub struct JsValidationMode {
    inner: ValidationMode,
}

impl JsValidationMode {
    pub(crate) fn from_inner(inner: ValidationMode) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> ValidationMode {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ValidationMode)]
impl JsValidationMode {
    /// Emit warnings (non-fatal) when validations fail.
    #[wasm_bindgen(js_name = Warn)]
    pub fn warn() -> JsValidationMode {
        Self {
            inner: ValidationMode::Warn,
        }
    }

    /// Treat validation failures as hard errors.
    #[wasm_bindgen(js_name = Error)]
    pub fn error() -> JsValidationMode {
        Self {
            inner: ValidationMode::Error,
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsValidationMode, JsValue> {
        let normalized = name.to_ascii_lowercase();
        match normalized.as_str() {
            "warn" | "warning" => Ok(Self::warn()),
            "error" | "strict" => Ok(Self::error()),
            _ => Err(JsValue::from_str(&format!(
                "Unknown validation mode: {}",
                name
            ))),
        }
    }

    /// Get the string name.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        match &self.inner {
            ValidationMode::Warn => "warn".to_string(),
            ValidationMode::Error => "error".to_string(),
        }
    }
}

// =============================================================================
// CalibrationConfig
// =============================================================================

/// Calibration configuration with solver settings and tolerances.
#[wasm_bindgen(js_name = CalibrationConfig)]
#[derive(Clone, Debug, Default)]
pub struct JsCalibrationConfig {
    inner: CalibrationConfig,
}

impl JsCalibrationConfig {
    pub(crate) fn from_inner(inner: CalibrationConfig) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CalibrationConfig {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CalibrationConfig)]
impl JsCalibrationConfig {
    /// Create a new calibration configuration.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CalibrationConfig::default(),
        }
    }

    // =========================================================================
    // Factory Methods
    // =========================================================================

    /// Create a conservative configuration for high-precision risk systems.
    ///
    /// - Higher tolerance for better accuracy (1e-12)
    /// - Moderate iteration limit (100)
    /// - Wide rate bounds to handle edge cases
    #[wasm_bindgen(js_name = conservative)]
    pub fn conservative() -> JsCalibrationConfig {
        Self {
            inner: CalibrationConfig::conservative(),
        }
    }

    /// Create an aggressive configuration for speed over precision.
    ///
    /// - Relaxed tolerance for faster convergence (1e-6)
    /// - Higher iteration limit (1000)
    /// - Standard rate bounds
    #[wasm_bindgen(js_name = aggressive)]
    pub fn aggressive() -> JsCalibrationConfig {
        Self {
            inner: CalibrationConfig::aggressive(),
        }
    }

    /// Create a fast configuration for interactive use.
    ///
    /// - Very relaxed tolerance (1e-4)
    /// - Low iteration limit (50)
    /// - For quick sanity checks and exploration
    #[wasm_bindgen(js_name = fast)]
    pub fn fast() -> JsCalibrationConfig {
        Self {
            inner: CalibrationConfig::fast(),
        }
    }

    // =========================================================================
    // Getters
    // =========================================================================

    /// Get tolerance.
    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.solver.tolerance()
    }

    /// Get max iterations.
    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.solver.max_iterations()
    }

    /// Get parallel flag.
    #[wasm_bindgen(getter, js_name = useParallel)]
    pub fn use_parallel(&self) -> bool {
        self.inner.use_parallel
    }

    /// Get verbose flag.
    #[wasm_bindgen(getter)]
    pub fn verbose(&self) -> bool {
        self.inner.verbose
    }

    /// Get solver kind.
    #[wasm_bindgen(getter, js_name = solverKind)]
    pub fn solver_kind(&self) -> JsSolverKind {
        JsSolverKind::from_inner(self.inner.solver.clone())
    }

    /// Set tolerance.
    #[wasm_bindgen(js_name = withTolerance)]
    pub fn with_tolerance(&self, tolerance: f64) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.solver = next.solver.with_tolerance(tolerance);
        Self::from_inner(next)
    }

    /// Set max iterations.
    #[wasm_bindgen(js_name = withMaxIterations)]
    pub fn with_max_iterations(&self, max_iterations: usize) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.solver = next.solver.with_max_iterations(max_iterations);
        Self::from_inner(next)
    }

    /// Set parallel flag.
    #[wasm_bindgen(js_name = withParallel)]
    pub fn with_parallel(&self, use_parallel: bool) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.use_parallel = use_parallel;
        Self::from_inner(next)
    }

    /// Set verbose flag.
    #[wasm_bindgen(js_name = withVerbose)]
    pub fn with_verbose(&self, verbose: bool) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.verbose = verbose;
        Self::from_inner(next)
    }

    /// Set solver kind.
    #[wasm_bindgen(js_name = withSolverKind)]
    pub fn with_solver_kind(&self, kind: &JsSolverKind) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.solver = kind.inner();
        Self::from_inner(next)
    }

    /// Set calibration method (bootstrap vs global solve).
    #[wasm_bindgen(js_name = withCalibrationMethod)]
    pub fn with_calibration_method(&self, method: &JsCalibrationMethod) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.calibration_method = method.inner();
        Self::from_inner(next)
    }

    /// Set explicit rate bounds for calibration.
    #[wasm_bindgen(js_name = withRateBounds)]
    pub fn with_rate_bounds(&self, bounds: &JsRateBounds) -> JsCalibrationConfig {
        Self::from_inner(self.inner.clone().with_rate_bounds(bounds.inner()))
    }

    /// Set rate bounds appropriate for a specific currency.
    #[wasm_bindgen(js_name = withRateBoundsForCurrency)]
    pub fn with_rate_bounds_for_currency(&self, currency: &JsCurrency) -> JsCalibrationConfig {
        Self::from_inner(
            self.inner
                .clone()
                .with_rate_bounds_for_currency(currency.inner()),
        )
    }

    /// Set validation mode (warn vs error).
    #[wasm_bindgen(js_name = withValidationMode)]
    pub fn with_validation_mode(&self, mode: &JsValidationMode) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.validation_mode = mode.inner();
        Self::from_inner(next)
    }

    /// Get the current calibration method.
    #[wasm_bindgen(getter, js_name = calibrationMethod)]
    pub fn calibration_method(&self) -> JsCalibrationMethod {
        JsCalibrationMethod::from_inner(self.inner.calibration_method.clone())
    }

    /// Get the current validation mode.
    #[wasm_bindgen(getter, js_name = validationMode)]
    pub fn validation_mode(&self) -> JsValidationMode {
        JsValidationMode::from_inner(self.inner.validation_mode.clone())
    }

    /// Get the current rate bounds.
    #[wasm_bindgen(getter, js_name = rateBounds)]
    pub fn rate_bounds(&self) -> JsRateBounds {
        JsRateBounds::from_inner(self.inner.rate_bounds.clone())
    }

    /// Get the rate bounds policy.
    #[wasm_bindgen(getter, js_name = rateBoundsPolicy)]
    pub fn rate_bounds_policy(&self) -> JsRateBoundsPolicy {
        JsRateBoundsPolicy::from_inner(self.inner.rate_bounds_policy.clone())
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCalibrationConfig, JsValue> {
        from_js_value(value).map(JsCalibrationConfig::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
