//! Calibration configuration types for WASM bindings.

use crate::utils::json::{from_js_value, to_js_value};
use finstack_valuations::calibration::{CalibrationConfig, SolverConfig};
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
