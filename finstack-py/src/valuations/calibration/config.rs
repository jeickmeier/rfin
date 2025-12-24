use super::validation::PyValidationConfig;
use crate::core::common::{labels::normalize_label, pycmp::richcmp_eq_ne};
use finstack_core::explain::ExplainOpts;
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationSolveMethod, RateBounds, SolverConfig, ValidationMode,
};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::Bound;

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SolverKind",
    frozen
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySolverKind {
    pub(crate) inner: SolverConfig,
}

impl PySolverKind {
    pub(crate) const fn new(inner: SolverConfig) -> Self {
        Self { inner }
    }

    fn discriminant(&self) -> isize {
        match self.inner {
            SolverConfig::Newton { .. } => 0,
            SolverConfig::Brent { .. } => 1,
        }
    }
}

#[pymethods]
impl PySolverKind {
    #[classattr]
    #[pyo3(name = "NEWTON")]
    fn newton_attr() -> Self {
        Self::new(SolverConfig::newton_default())
    }
    #[classattr]
    #[pyo3(name = "BRENT")]
    fn brent_attr() -> Self {
        Self::new(SolverConfig::brent_default())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "newton" => Ok(Self::new(SolverConfig::newton_default())),
            "brent" => Ok(Self::new(SolverConfig::brent_default())),
            other => Err(PyKeyError::new_err(format!("Unknown solver kind: {other}"))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            SolverConfig::Newton { .. } => "newton",
            SolverConfig::Brent { .. } => "brent",
        }
    }

    fn __repr__(&self) -> String {
        format!("SolverKind('{}')", self.name())
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = other.extract::<PySolverKind>().ok();
        richcmp_eq_ne(
            py,
            &self.discriminant(),
            rhs.as_ref().map(PySolverKind::discriminant),
            op,
        )
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CalibrationMethod",
    frozen
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyCalibrationMethod {
    pub(crate) inner: CalibrationSolveMethod,
}

impl PyCalibrationMethod {
    pub(crate) const fn new(inner: CalibrationSolveMethod) -> Self {
        Self { inner }
    }

    const fn discriminant(&self) -> (isize, bool) {
        match self.inner {
            CalibrationSolveMethod::Bootstrap => (0, false),
            CalibrationSolveMethod::GlobalSolve {
                use_analytical_jacobian,
            } => (1, use_analytical_jacobian),
        }
    }
}

#[pymethods]
impl PyCalibrationMethod {
    #[classattr]
    const BOOTSTRAP: Self = Self::new(CalibrationSolveMethod::Bootstrap);
    #[classattr]
    const GLOBAL_SOLVE: Self = Self::new(CalibrationSolveMethod::GlobalSolve {
        use_analytical_jacobian: false,
    });

    #[classmethod]
    #[pyo3(signature = (name, use_analytical_jacobian = false))]
    #[pyo3(text_signature = "(cls, name, use_analytical_jacobian=False)")]
    fn from_name(
        _cls: &Bound<'_, PyType>,
        name: &str,
        use_analytical_jacobian: bool,
    ) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "bootstrap" => Ok(Self::new(CalibrationSolveMethod::Bootstrap)),
            "global_solve" => Ok(Self::new(CalibrationSolveMethod::GlobalSolve {
                use_analytical_jacobian,
            })),
            other => Err(PyKeyError::new_err(format!(
                "Unknown calibration method: {other}"
            ))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            CalibrationSolveMethod::Bootstrap => "bootstrap",
            CalibrationSolveMethod::GlobalSolve { .. } => "global_solve",
        }
    }

    #[getter]
    fn use_analytical_jacobian(&self) -> bool {
        match self.inner {
            CalibrationSolveMethod::Bootstrap => false,
            CalibrationSolveMethod::GlobalSolve {
                use_analytical_jacobian,
            } => use_analytical_jacobian,
        }
    }

    fn with_use_analytical_jacobian(&self, value: bool) -> Self {
        match self.inner {
            CalibrationSolveMethod::Bootstrap => Self::new(CalibrationSolveMethod::Bootstrap),
            CalibrationSolveMethod::GlobalSolve { .. } => {
                Self::new(CalibrationSolveMethod::GlobalSolve {
                    use_analytical_jacobian: value,
                })
            }
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            CalibrationSolveMethod::Bootstrap => "CalibrationMethod('bootstrap')".to_string(),
            CalibrationSolveMethod::GlobalSolve {
                use_analytical_jacobian,
            } => format!(
                "CalibrationMethod('global_solve', use_analytical_jacobian={})",
                use_analytical_jacobian
            ),
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }

    fn __hash__(&self) -> isize {
        let (tag, flag) = self.discriminant();
        (tag << 1) ^ (flag as isize)
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = other.extract::<PyCalibrationMethod>().ok();
        richcmp_eq_ne(
            py,
            &self.discriminant(),
            rhs.as_ref().map(PyCalibrationMethod::discriminant),
            op,
        )
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "ValidationMode",
    frozen
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyValidationMode {
    pub(crate) inner: ValidationMode,
}

impl PyValidationMode {
    pub(crate) const fn new(inner: ValidationMode) -> Self {
        Self { inner }
    }

    const fn discriminant(&self) -> isize {
        match self.inner {
            ValidationMode::Warn => 0,
            ValidationMode::Error => 1,
        }
    }
}

#[pymethods]
impl PyValidationMode {
    #[classattr]
    const WARN: Self = Self::new(ValidationMode::Warn);
    #[classattr]
    const ERROR: Self = Self::new(ValidationMode::Error);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "warn" | "warning" => Ok(Self::new(ValidationMode::Warn)),
            "error" | "strict" => Ok(Self::new(ValidationMode::Error)),
            other => Err(PyKeyError::new_err(format!(
                "Unknown validation mode: {other}"
            ))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            ValidationMode::Warn => "warn",
            ValidationMode::Error => "error",
        }
    }

    fn __repr__(&self) -> String {
        format!("ValidationMode('{}')", self.name())
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = other.extract::<PyValidationMode>().ok();
        richcmp_eq_ne(
            py,
            &self.discriminant(),
            rhs.as_ref().map(PyValidationMode::discriminant),
            op,
        )
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "RateBounds",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRateBounds {
    pub(crate) inner: RateBounds,
}

impl PyRateBounds {
    pub(crate) fn new(inner: RateBounds) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRateBounds {
    #[new]
    #[pyo3(text_signature = "(min_rate=-0.02, max_rate=0.50)")]
    fn ctor(min_rate: Option<f64>, max_rate: Option<f64>) -> PyResult<Self> {
        use crate::errors::core_to_py;
        let mut inner = RateBounds::default();
        if let Some(v) = min_rate {
            inner.min_rate = v;
        }
        if let Some(v) = max_rate {
            inner.max_rate = v;
        }
        inner.validate().map_err(core_to_py)?;
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn emerging_markets(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(RateBounds::emerging_markets())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn negative_rate_environment(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(RateBounds::negative_rate_environment())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn stress_test(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(RateBounds::stress_test())
    }

    #[getter]
    fn min_rate(&self) -> f64 {
        self.inner.min_rate
    }

    #[getter]
    fn max_rate(&self) -> f64 {
        self.inner.max_rate
    }

    fn __repr__(&self) -> String {
        format!(
            "RateBounds(min_rate={:.6}, max_rate={:.6})",
            self.inner.min_rate, self.inner.max_rate
        )
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CalibrationConfig",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCalibrationConfig {
    pub(crate) inner: CalibrationConfig,
}

impl PyCalibrationConfig {
    pub(crate) fn new(inner: CalibrationConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCalibrationConfig {
    #[new]
    #[pyo3(
        signature = (
            tolerance=None,
            max_iterations=None,
            use_parallel=None,
            verbose=None,
            solver_kind=None,
            calibration_method=None,
            validation_mode=None,
            validation=None,
            rate_bounds=None,
            explain=None
        ),
        text_signature = "(tolerance=None, max_iterations=None, use_parallel=None, verbose=None, solver_kind=None, calibration_method=None, validation_mode=None, validation=None, rate_bounds=None, explain=None)"
    )]
    fn ctor(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        use_parallel: Option<bool>,
        verbose: Option<bool>,
        solver_kind: Option<PyRef<PySolverKind>>,
        calibration_method: Option<PyRef<PyCalibrationMethod>>,
        validation_mode: Option<PyRef<PyValidationMode>>,
        validation: Option<PyRef<PyValidationConfig>>,
        rate_bounds: Option<PyRef<PyRateBounds>>,
        explain: Option<bool>,
    ) -> PyResult<Self> {
        let mut inner = CalibrationConfig::default();
        // Apply solver kind first, then apply numeric overrides (tolerance / max_iterations).
        //
        // Rationale: `SolverKind` wraps a full `SolverConfig` with its own defaults.
        // Users expect explicit `tolerance`/`max_iterations` args to win over those defaults.
        if let Some(kind) = solver_kind {
            inner.solver = kind.inner.clone();
        }
        if let Some(val) = tolerance {
            inner.solver = inner.solver.with_tolerance(val);
        }
        if let Some(val) = max_iterations {
            inner.solver = inner.solver.with_max_iterations(val);
        }
        if let Some(flag) = use_parallel {
            inner.use_parallel = flag;
        }
        if let Some(flag) = verbose {
            inner.verbose = flag;
        }
        if let Some(method) = calibration_method {
            inner.calibration_method = method.inner.clone();
        }
        if let Some(mode) = validation_mode {
            inner.validation_mode = mode.inner.clone();
        }
        if let Some(cfg) = validation {
            inner.validation = cfg.inner.clone();
        }
        if let Some(bounds) = rate_bounds {
            inner.rate_bounds = bounds.inner.clone();
        }
        if explain.unwrap_or(false) {
            inner.explain = ExplainOpts::enabled();
        }
        Ok(Self::new(inner))
    }

    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.solver.tolerance()
    }

    #[getter]
    fn max_iterations(&self) -> usize {
        self.inner.solver.max_iterations()
    }

    #[getter]
    fn use_parallel(&self) -> bool {
        self.inner.use_parallel
    }

    #[getter]
    fn verbose(&self) -> bool {
        self.inner.verbose
    }

    #[getter]
    fn solver_kind(&self) -> PySolverKind {
        PySolverKind::new(self.inner.solver.clone())
    }

    #[getter]
    fn calibration_method(&self) -> PyCalibrationMethod {
        PyCalibrationMethod::new(self.inner.calibration_method.clone())
    }

    #[getter]
    fn validation_mode(&self) -> PyValidationMode {
        PyValidationMode::new(self.inner.validation_mode.clone())
    }

    #[getter]
    fn validation_config(&self) -> PyValidationConfig {
        PyValidationConfig::new(self.inner.validation.clone())
    }

    #[getter]
    fn rate_bounds(&self) -> PyRateBounds {
        PyRateBounds::new(self.inner.rate_bounds.clone())
    }

    #[getter]
    fn explain_enabled(&self) -> bool {
        self.inner.explain.enabled
    }

    fn with_tolerance(&self, tolerance: f64) -> Self {
        let mut next = self.inner.clone();
        next.solver = next.solver.with_tolerance(tolerance);
        Self::new(next)
    }

    fn with_max_iterations(&self, max_iterations: usize) -> Self {
        let mut next = self.inner.clone();
        next.solver = next.solver.with_max_iterations(max_iterations);
        Self::new(next)
    }

    fn with_parallel(&self, flag: bool) -> Self {
        let mut next = self.inner.clone();
        next.use_parallel = flag;
        Self::new(next)
    }

    fn with_verbose(&self, verbose: bool) -> Self {
        let mut next = self.inner.clone();
        next.verbose = verbose;
        Self::new(next)
    }

    fn with_solver_kind(&self, kind: PyRef<PySolverKind>) -> Self {
        let mut next = self.inner.clone();
        // Preserve any explicit numeric overrides currently stored in `next.solver`.
        let tol = next.solver.tolerance();
        let max_it = next.solver.max_iterations();
        next.solver = kind
            .inner
            .clone()
            .with_tolerance(tol)
            .with_max_iterations(max_it);
        Self::new(next)
    }

    fn with_calibration_method(&self, method: PyRef<PyCalibrationMethod>) -> Self {
        let mut next = self.inner.clone();
        next.calibration_method = method.inner.clone();
        Self::new(next)
    }

    fn with_validation_mode(&self, mode: PyRef<PyValidationMode>) -> Self {
        let mut next = self.inner.clone();
        next.validation_mode = mode.inner.clone();
        Self::new(next)
    }

    fn with_validation_config(&self, config: PyRef<PyValidationConfig>) -> Self {
        let mut next = self.inner.clone();
        next.validation = config.inner.clone();
        Self::new(next)
    }

    fn with_rate_bounds(&self, bounds: PyRef<PyRateBounds>) -> Self {
        let mut next = self.inner.clone();
        next.rate_bounds = bounds.inner.clone();
        Self::new(next)
    }

    fn with_explain(&self) -> Self {
        let mut next = self.inner.clone();
        next.explain = ExplainOpts::enabled();
        Self::new(next)
    }

    fn __repr__(&self) -> PyResult<String> {
        let parts = vec![
            format!("tolerance={}", self.inner.solver.tolerance()),
            format!("max_iterations={}", self.inner.solver.max_iterations()),
            format!("use_parallel={}", self.inner.use_parallel),
            format!("verbose={}", self.inner.verbose),
            format!(
                "solver_kind='{}'",
                PySolverKind::new(self.inner.solver.clone()).name()
            ),
            format!(
                "calibration_method='{}'",
                PyCalibrationMethod::new(self.inner.calibration_method.clone()).name()
            ),
            format!(
                "validation_mode='{}'",
                PyValidationMode::new(self.inner.validation_mode.clone()).name()
            ),
            format!(
                "rate_bounds=RateBounds(min_rate={:.6}, max_rate={:.6})",
                self.inner.rate_bounds.min_rate, self.inner.rate_bounds.max_rate
            ),
            format!("explain={}", self.inner.explain.enabled),
        ];
        Ok(format!("CalibrationConfig({})", parts.join(", ")))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PySolverKind>()?;
    module.add_class::<PyCalibrationMethod>()?;
    module.add_class::<PyValidationMode>()?;
    module.add_class::<PyRateBounds>()?;
    module.add_class::<PyCalibrationConfig>()?;
    Ok(vec![
        "SolverKind",
        "CalibrationMethod",
        "ValidationMode",
        "RateBounds",
        "CalibrationConfig",
    ])
}
