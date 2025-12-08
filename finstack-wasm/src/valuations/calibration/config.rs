//! Calibration configuration types for WASM bindings.

use finstack_valuations::calibration::{CalibrationConfig, MultiCurveConfig, SolverKind};
use crate::utils::json::{from_js_value, to_js_value};
use wasm_bindgen::prelude::*;

/// Solver strategy enumeration for calibration routines.
#[wasm_bindgen(js_name = SolverKind)]
#[derive(Clone, Debug)]
pub struct JsSolverKind {
    inner: SolverKind,
}

impl JsSolverKind {
    pub(crate) fn from_inner(inner: SolverKind) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> SolverKind {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = SolverKind)]
impl JsSolverKind {
    /// Newton-Raphson solver.
    #[wasm_bindgen(js_name = Newton)]
    pub fn newton() -> Self {
        Self {
            inner: SolverKind::Newton,
        }
    }

    /// Brent's method (bracketing).
    #[wasm_bindgen(js_name = Brent)]
    pub fn brent() -> Self {
        Self {
            inner: SolverKind::Brent,
        }
    }

    /// Levenberg-Marquardt optimizer.
    #[wasm_bindgen(js_name = LevenbergMarquardt)]
    pub fn levenberg_marquardt() -> Self {
        Self {
            inner: SolverKind::LevenbergMarquardt,
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsSolverKind, JsValue> {
        let normalized = name.to_ascii_lowercase().replace('-', "_");
        let kind = match normalized.as_str() {
            "newton" => SolverKind::Newton,
            "brent" => SolverKind::Brent,
            "levenberg_marquardt" => SolverKind::LevenbergMarquardt,
            _ => return Err(JsValue::from_str(&format!("Unknown solver kind: {}", name))),
        };
        Ok(Self { inner: kind })
    }

    /// Get the string name of the solver kind.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        match self.inner {
            SolverKind::Newton => "newton".to_string(),
            SolverKind::Brent => "brent".to_string(),
            SolverKind::LevenbergMarquardt => "levenberg_marquardt".to_string(),
        }
    }
}

/// Multi-curve calibration configuration.
#[wasm_bindgen(js_name = MultiCurveConfig)]
#[derive(Clone, Debug)]
pub struct JsMultiCurveConfig {
    inner: MultiCurveConfig,
}

impl JsMultiCurveConfig {
    pub(crate) fn from_inner(inner: MultiCurveConfig) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> MultiCurveConfig {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = MultiCurveConfig)]
impl JsMultiCurveConfig {
    /// Create a new multi-curve configuration.
    #[wasm_bindgen(constructor)]
    pub fn new(calibrate_basis: bool, enforce_separation: bool) -> Self {
        let config = MultiCurveConfig {
            calibrate_basis,
            enforce_separation,
        };
        Self { inner: config }
    }

    /// Standard multi-curve configuration.
    #[wasm_bindgen(js_name = standard)]
    pub fn standard() -> Self {
        Self {
            inner: MultiCurveConfig::default(),
        }
    }

    /// Whether to calibrate basis swaps.
    #[wasm_bindgen(getter, js_name = calibrateBasis)]
    pub fn calibrate_basis(&self) -> bool {
        self.inner.calibrate_basis
    }

    /// Whether to enforce curve separation.
    #[wasm_bindgen(getter, js_name = enforceSeparation)]
    pub fn enforce_separation(&self) -> bool {
        self.inner.enforce_separation
    }

    /// Set calibrate basis flag.
    #[wasm_bindgen(js_name = withCalibrateBasis)]
    pub fn with_calibrate_basis(&self, value: bool) -> JsMultiCurveConfig {
        let mut next = self.inner.clone();
        next.calibrate_basis = value;
        Self::from_inner(next)
    }

    /// Set enforce separation flag.
    #[wasm_bindgen(js_name = withEnforceSeparation)]
    pub fn with_enforce_separation(&self, value: bool) -> JsMultiCurveConfig {
        let mut next = self.inner.clone();
        next.enforce_separation = value;
        Self::from_inner(next)
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMultiCurveConfig, JsValue> {
        from_js_value(value).map(JsMultiCurveConfig::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

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

    /// Multi-curve preset configuration.
    #[wasm_bindgen(js_name = multiCurve)]
    pub fn multi_curve() -> Self {
        Self {
            inner: CalibrationConfig::multi_curve(),
        }
    }

    /// Get tolerance.
    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Get max iterations.
    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
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
        JsSolverKind::from_inner(self.inner.solver_kind.clone())
    }

    /// Set tolerance.
    #[wasm_bindgen(js_name = withTolerance)]
    pub fn with_tolerance(&self, tolerance: f64) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.tolerance = tolerance;
        Self::from_inner(next)
    }

    /// Set max iterations.
    #[wasm_bindgen(js_name = withMaxIterations)]
    pub fn with_max_iterations(&self, max_iterations: usize) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.max_iterations = max_iterations;
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
        next.solver_kind = kind.inner();
        Self::from_inner(next)
    }

    /// Set multi-curve configuration.
    #[wasm_bindgen(js_name = withMultiCurveConfig)]
    pub fn with_multi_curve_config(&self, config: &JsMultiCurveConfig) -> JsCalibrationConfig {
        let mut next = self.inner.clone();
        next.multi_curve = config.inner();
        Self::from_inner(next)
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
