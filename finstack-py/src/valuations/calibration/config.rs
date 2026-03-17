use super::validation::PyValidationConfig;
use crate::core::common::args::{CurrencyArg, DayCountArg, ExtrapolationPolicyArg, InterpStyleArg};
use crate::core::common::{labels::normalize_label, pycmp::richcmp_eq_ne};
use crate::core::dates::PyDayCount;
use crate::core::math::interp::{PyExtrapolationPolicy, PyInterpStyle};
use finstack_core::explain::ExplainOpts;
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationMethod, DiscountCurveSolveConfig, HazardCurveSolveConfig,
    InflationCurveSolveConfig, RateBounds, RateBoundsPolicy, RatesStepConventions,
    ResidualWeightingScheme, SolverConfig, ValidationMode,
};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::Bound;

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SolverKind",
    frozen,
    from_py_object
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

// =============================================================================
// ResidualWeightingScheme
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "ResidualWeightingScheme",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyResidualWeightingScheme {
    pub(crate) inner: ResidualWeightingScheme,
}

impl PyResidualWeightingScheme {
    pub(crate) const fn new(inner: ResidualWeightingScheme) -> Self {
        Self { inner }
    }

    const fn discriminant(&self) -> isize {
        match self.inner {
            ResidualWeightingScheme::Equal => 0,
            ResidualWeightingScheme::LinearTime => 1,
            ResidualWeightingScheme::SqrtTime => 2,
            ResidualWeightingScheme::InverseDuration => 3,
        }
    }
}

#[pymethods]
impl PyResidualWeightingScheme {
    #[classattr]
    const EQUAL: Self = Self::new(ResidualWeightingScheme::Equal);
    #[classattr]
    const LINEAR_TIME: Self = Self::new(ResidualWeightingScheme::LinearTime);
    #[classattr]
    const SQRT_TIME: Self = Self::new(ResidualWeightingScheme::SqrtTime);
    #[classattr]
    const INVERSE_DURATION: Self = Self::new(ResidualWeightingScheme::InverseDuration);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "equal" => Ok(Self::EQUAL),
            "linear_time" => Ok(Self::LINEAR_TIME),
            "sqrt_time" => Ok(Self::SQRT_TIME),
            "inverse_duration" => Ok(Self::INVERSE_DURATION),
            other => Err(PyKeyError::new_err(format!(
                "Unknown weighting scheme: {other}"
            ))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            ResidualWeightingScheme::Equal => "equal",
            ResidualWeightingScheme::LinearTime => "linear_time",
            ResidualWeightingScheme::SqrtTime => "sqrt_time",
            ResidualWeightingScheme::InverseDuration => "inverse_duration",
        }
    }

    fn __repr__(&self) -> String {
        format!("ResidualWeightingScheme('{}')", self.name())
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
        let rhs = other.extract::<PyResidualWeightingScheme>().ok();
        richcmp_eq_ne(
            py,
            &self.discriminant(),
            rhs.as_ref().map(PyResidualWeightingScheme::discriminant),
            op,
        )
    }
}

// =============================================================================
// RateBoundsPolicy
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "RateBoundsPolicy",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyRateBoundsPolicy {
    pub(crate) inner: RateBoundsPolicy,
}

impl PyRateBoundsPolicy {
    pub(crate) const fn new(inner: RateBoundsPolicy) -> Self {
        Self { inner }
    }

    const fn discriminant(&self) -> isize {
        match self.inner {
            RateBoundsPolicy::AutoCurrency => 0,
            RateBoundsPolicy::Explicit => 1,
        }
    }
}

#[pymethods]
impl PyRateBoundsPolicy {
    #[classattr]
    const AUTO_CURRENCY: Self = Self::new(RateBoundsPolicy::AutoCurrency);
    #[classattr]
    const EXPLICIT: Self = Self::new(RateBoundsPolicy::Explicit);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "auto_currency" | "auto" => Ok(Self::AUTO_CURRENCY),
            "explicit" => Ok(Self::EXPLICIT),
            other => Err(PyKeyError::new_err(format!(
                "Unknown rate bounds policy: {other}"
            ))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            RateBoundsPolicy::AutoCurrency => "auto_currency",
            RateBoundsPolicy::Explicit => "explicit",
        }
    }

    fn __repr__(&self) -> String {
        format!("RateBoundsPolicy('{}')", self.name())
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
        let rhs = other.extract::<PyRateBoundsPolicy>().ok();
        richcmp_eq_ne(
            py,
            &self.discriminant(),
            rhs.as_ref().map(PyRateBoundsPolicy::discriminant),
            op,
        )
    }
}

// =============================================================================
// RatesStepConventions
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "RatesStepConventions",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRatesStepConventions {
    pub(crate) inner: RatesStepConventions,
}

#[pymethods]
impl PyRatesStepConventions {
    #[new]
    #[pyo3(signature = (curve_day_count=None))]
    fn ctor(curve_day_count: Option<DayCountArg>) -> Self {
        Self {
            inner: RatesStepConventions {
                curve_day_count: curve_day_count.map(|d| d.0),
            },
        }
    }

    #[getter]
    fn curve_day_count(&self) -> Option<PyDayCount> {
        self.inner.curve_day_count.map(PyDayCount::new)
    }

    fn __repr__(&self) -> String {
        match self.inner.curve_day_count {
            Some(dc) => format!("RatesStepConventions(curve_day_count='{dc:?}')"),
            None => "RatesStepConventions()".to_string(),
        }
    }
}

// =============================================================================
// HazardCurveSolveConfig
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "HazardCurveSolveConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyHazardCurveSolveConfig {
    pub(crate) inner: HazardCurveSolveConfig,
}

impl PyHazardCurveSolveConfig {
    pub(crate) fn new(inner: HazardCurveSolveConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHazardCurveSolveConfig {
    #[new]
    #[pyo3(signature = (hazard_hard_min=None, hazard_hard_max=None, weighting_scheme=None, validation_tolerance=None))]
    fn ctor(
        hazard_hard_min: Option<f64>,
        hazard_hard_max: Option<f64>,
        weighting_scheme: Option<PyRef<PyResidualWeightingScheme>>,
        validation_tolerance: Option<f64>,
    ) -> Self {
        let mut inner = HazardCurveSolveConfig::default();
        if let Some(v) = hazard_hard_min {
            inner.hazard_hard_min = v;
        }
        if let Some(v) = hazard_hard_max {
            inner.hazard_hard_max = v;
        }
        if let Some(ws) = weighting_scheme {
            inner.weighting_scheme = ws.inner.clone();
        }
        if let Some(v) = validation_tolerance {
            inner.validation_tolerance = v;
        }
        Self::new(inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn distressed(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(HazardCurveSolveConfig::distressed())
    }

    #[getter]
    fn hazard_hard_min(&self) -> f64 {
        self.inner.hazard_hard_min
    }

    #[getter]
    fn hazard_hard_max(&self) -> f64 {
        self.inner.hazard_hard_max
    }

    #[getter]
    fn weighting_scheme(&self) -> PyResidualWeightingScheme {
        PyResidualWeightingScheme::new(self.inner.weighting_scheme.clone())
    }

    #[getter]
    fn validation_tolerance(&self) -> f64 {
        self.inner.validation_tolerance
    }

    fn __repr__(&self) -> String {
        format!(
            "HazardCurveSolveConfig(hazard_hard_min={}, hazard_hard_max={}, validation_tolerance={:.2e})",
            self.inner.hazard_hard_min, self.inner.hazard_hard_max, self.inner.validation_tolerance
        )
    }
}

// =============================================================================
// InflationCurveSolveConfig
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "InflationCurveSolveConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInflationCurveSolveConfig {
    pub(crate) inner: InflationCurveSolveConfig,
}

impl PyInflationCurveSolveConfig {
    pub(crate) fn new(inner: InflationCurveSolveConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationCurveSolveConfig {
    #[new]
    #[pyo3(signature = (weighting_scheme=None, validation_tolerance=None))]
    fn ctor(
        weighting_scheme: Option<PyRef<PyResidualWeightingScheme>>,
        validation_tolerance: Option<f64>,
    ) -> Self {
        let mut inner = InflationCurveSolveConfig::default();
        if let Some(ws) = weighting_scheme {
            inner.weighting_scheme = ws.inner.clone();
        }
        if let Some(v) = validation_tolerance {
            inner.validation_tolerance = v;
        }
        Self::new(inner)
    }

    #[getter]
    fn weighting_scheme(&self) -> PyResidualWeightingScheme {
        PyResidualWeightingScheme::new(self.inner.weighting_scheme.clone())
    }

    #[getter]
    fn validation_tolerance(&self) -> f64 {
        self.inner.validation_tolerance
    }

    fn __repr__(&self) -> String {
        format!(
            "InflationCurveSolveConfig(validation_tolerance={:.2e})",
            self.inner.validation_tolerance
        )
    }
}

// =============================================================================
// DiscountCurveSolveConfig
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "DiscountCurveSolveConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDiscountCurveSolveConfig {
    pub(crate) inner: DiscountCurveSolveConfig,
}

impl PyDiscountCurveSolveConfig {
    pub(crate) fn new(inner: DiscountCurveSolveConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDiscountCurveSolveConfig {
    #[new]
    #[pyo3(signature = (
        scan_grid_points=None,
        min_scan_grid_points=None,
        scan_grid_step=None,
        df_hard_min=None,
        df_hard_max=None,
        min_t_spot=None,
        interp_style=None,
        extrapolation_policy=None,
        bootstrap_seed_global_solve=None,
        allow_non_monotonic_final=None,
        weighting_scheme=None,
        jacobian_step_size=None,
        validation_tolerance=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        scan_grid_points: Option<usize>,
        min_scan_grid_points: Option<usize>,
        scan_grid_step: Option<f64>,
        df_hard_min: Option<f64>,
        df_hard_max: Option<f64>,
        min_t_spot: Option<f64>,
        interp_style: Option<InterpStyleArg>,
        extrapolation_policy: Option<ExtrapolationPolicyArg>,
        bootstrap_seed_global_solve: Option<bool>,
        allow_non_monotonic_final: Option<bool>,
        weighting_scheme: Option<PyRef<PyResidualWeightingScheme>>,
        jacobian_step_size: Option<f64>,
        validation_tolerance: Option<f64>,
    ) -> Self {
        let mut inner = DiscountCurveSolveConfig::default();
        if let Some(v) = scan_grid_points {
            inner.scan_grid_points = v;
        }
        if let Some(v) = min_scan_grid_points {
            inner.min_scan_grid_points = v;
        }
        if let Some(v) = scan_grid_step {
            inner.scan_grid_step = v;
        }
        if let Some(v) = df_hard_min {
            inner.df_hard_min = v;
        }
        if let Some(v) = df_hard_max {
            inner.df_hard_max = v;
        }
        if let Some(v) = min_t_spot {
            inner.min_t_spot = v;
        }
        if let Some(InterpStyleArg(v)) = interp_style {
            inner.interp_style = v;
        }
        if let Some(ExtrapolationPolicyArg(v)) = extrapolation_policy {
            inner.extrapolation_policy = v;
        }
        if let Some(v) = bootstrap_seed_global_solve {
            inner.bootstrap_seed_global_solve = v;
        }
        if allow_non_monotonic_final.is_some() {
            inner.allow_non_monotonic_final = allow_non_monotonic_final;
        }
        if let Some(ws) = weighting_scheme {
            inner.weighting_scheme = ws.inner.clone();
        }
        if let Some(v) = jacobian_step_size {
            inner.jacobian_step_size = v;
        }
        if let Some(v) = validation_tolerance {
            inner.validation_tolerance = v;
        }
        Self::new(inner)
    }

    #[getter]
    fn scan_grid_points(&self) -> usize {
        self.inner.scan_grid_points
    }

    #[getter]
    fn min_scan_grid_points(&self) -> usize {
        self.inner.min_scan_grid_points
    }

    #[getter]
    fn scan_grid_step(&self) -> f64 {
        self.inner.scan_grid_step
    }

    #[getter]
    fn df_hard_min(&self) -> f64 {
        self.inner.df_hard_min
    }

    #[getter]
    fn df_hard_max(&self) -> f64 {
        self.inner.df_hard_max
    }

    #[getter]
    fn min_t_spot(&self) -> f64 {
        self.inner.min_t_spot
    }

    #[getter]
    fn interp_style(&self) -> PyInterpStyle {
        PyInterpStyle::new(self.inner.interp_style)
    }

    #[getter]
    fn extrapolation_policy(&self) -> PyExtrapolationPolicy {
        PyExtrapolationPolicy::new(self.inner.extrapolation_policy)
    }

    #[getter]
    fn bootstrap_seed_global_solve(&self) -> bool {
        self.inner.bootstrap_seed_global_solve
    }

    #[getter]
    fn allow_non_monotonic_final(&self) -> Option<bool> {
        self.inner.allow_non_monotonic_final
    }

    #[getter]
    fn weighting_scheme(&self) -> PyResidualWeightingScheme {
        PyResidualWeightingScheme::new(self.inner.weighting_scheme.clone())
    }

    #[getter]
    fn jacobian_step_size(&self) -> f64 {
        self.inner.jacobian_step_size
    }

    #[getter]
    fn validation_tolerance(&self) -> f64 {
        self.inner.validation_tolerance
    }

    fn __repr__(&self) -> String {
        format!(
            "DiscountCurveSolveConfig(scan_grid_points={}, df_hard_min={:.2e}, validation_tolerance={:.2e})",
            self.inner.scan_grid_points, self.inner.df_hard_min, self.inner.validation_tolerance
        )
    }
}

// =============================================================================
// CalibrationMethod
// =============================================================================

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CalibrationMethod",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyCalibrationMethod {
    pub(crate) inner: CalibrationMethod,
}

impl PyCalibrationMethod {
    pub(crate) const fn new(inner: CalibrationMethod) -> Self {
        Self { inner }
    }

    const fn discriminant(&self) -> (isize, bool) {
        match self.inner {
            CalibrationMethod::Bootstrap => (0, false),
            CalibrationMethod::GlobalSolve {
                use_analytical_jacobian,
            } => (1, use_analytical_jacobian),
        }
    }
}

#[pymethods]
impl PyCalibrationMethod {
    #[classattr]
    const BOOTSTRAP: Self = Self::new(CalibrationMethod::Bootstrap);
    #[classattr]
    const GLOBAL_SOLVE: Self = Self::new(CalibrationMethod::GlobalSolve {
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
            "bootstrap" => Ok(Self::new(CalibrationMethod::Bootstrap)),
            "global_solve" => Ok(Self::new(CalibrationMethod::GlobalSolve {
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
            CalibrationMethod::Bootstrap => "bootstrap",
            CalibrationMethod::GlobalSolve { .. } => "global_solve",
        }
    }

    #[getter]
    fn use_analytical_jacobian(&self) -> bool {
        match self.inner {
            CalibrationMethod::Bootstrap => false,
            CalibrationMethod::GlobalSolve {
                use_analytical_jacobian,
            } => use_analytical_jacobian,
        }
    }

    fn with_use_analytical_jacobian(&self, value: bool) -> Self {
        match self.inner {
            CalibrationMethod::Bootstrap => Self::new(CalibrationMethod::Bootstrap),
            CalibrationMethod::GlobalSolve { .. } => Self::new(CalibrationMethod::GlobalSolve {
                use_analytical_jacobian: value,
            }),
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            CalibrationMethod::Bootstrap => "CalibrationMethod('bootstrap')".to_string(),
            CalibrationMethod::GlobalSolve {
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
    frozen,
    from_py_object
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
    frozen,
    from_py_object
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

    #[classmethod]
    #[pyo3(text_signature = "(cls, currency)")]
    fn for_currency(_cls: &Bound<'_, PyType>, currency: CurrencyArg) -> Self {
        Self::new(RateBounds::for_currency(currency.0))
    }

    #[getter]
    fn min_rate(&self) -> f64 {
        self.inner.min_rate
    }

    #[getter]
    fn max_rate(&self) -> f64 {
        self.inner.max_rate
    }

    fn contains(&self, rate: f64) -> bool {
        self.inner.contains(rate)
    }

    fn clamp(&self, rate: f64) -> f64 {
        self.inner.clamp(rate)
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
    frozen,
    from_py_object
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
            rate_bounds_policy=None,
            explain=None,
            compute_diagnostics=None,
            discount_curve=None,
            hazard_curve=None,
            inflation_curve=None
        ),
        text_signature = "(tolerance=None, max_iterations=None, use_parallel=None, verbose=None, solver_kind=None, calibration_method=None, validation_mode=None, validation=None, rate_bounds=None, rate_bounds_policy=None, explain=None, compute_diagnostics=None, discount_curve=None, hazard_curve=None, inflation_curve=None)"
    )]
    #[allow(clippy::too_many_arguments)]
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
        rate_bounds_policy: Option<PyRef<PyRateBoundsPolicy>>,
        explain: Option<bool>,
        compute_diagnostics: Option<bool>,
        discount_curve: Option<PyRef<PyDiscountCurveSolveConfig>>,
        hazard_curve: Option<PyRef<PyHazardCurveSolveConfig>>,
        inflation_curve: Option<PyRef<PyInflationCurveSolveConfig>>,
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
        if let Some(policy) = rate_bounds_policy {
            inner.rate_bounds_policy = policy.inner.clone();
        }
        if explain.unwrap_or(false) {
            inner.explain = ExplainOpts::enabled();
        }
        if let Some(flag) = compute_diagnostics {
            inner.compute_diagnostics = flag;
        }
        if let Some(dc) = discount_curve {
            inner.discount_curve = dc.inner.clone();
        }
        if let Some(hc) = hazard_curve {
            inner.hazard_curve = hc.inner.clone();
        }
        if let Some(ic) = inflation_curve {
            inner.inflation_curve = ic.inner.clone();
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

    #[getter]
    fn compute_diagnostics(&self) -> bool {
        self.inner.compute_diagnostics
    }

    #[getter]
    fn rate_bounds_policy(&self) -> PyRateBoundsPolicy {
        PyRateBoundsPolicy::new(self.inner.rate_bounds_policy.clone())
    }

    #[getter]
    fn discount_curve(&self) -> PyDiscountCurveSolveConfig {
        PyDiscountCurveSolveConfig::new(self.inner.discount_curve.clone())
    }

    #[getter]
    fn hazard_curve(&self) -> PyHazardCurveSolveConfig {
        PyHazardCurveSolveConfig::new(self.inner.hazard_curve.clone())
    }

    #[getter]
    fn inflation_curve(&self) -> PyInflationCurveSolveConfig {
        PyInflationCurveSolveConfig::new(self.inner.inflation_curve.clone())
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
        let current_default = match next.solver {
            SolverConfig::Newton { .. } => SolverConfig::newton_default(),
            SolverConfig::Brent { .. } => SolverConfig::brent_default(),
        };

        let tol = next.solver.tolerance();
        let max_it = next.solver.max_iterations();
        let has_explicit_tolerance = (tol - current_default.tolerance()).abs() > f64::EPSILON;
        let has_explicit_max_iterations = max_it != current_default.max_iterations();

        next.solver = kind.inner.clone();

        if has_explicit_tolerance {
            next.solver = next.solver.with_tolerance(tol);
        }
        if has_explicit_max_iterations {
            next.solver = next.solver.with_max_iterations(max_it);
        }
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

    fn with_compute_diagnostics(&self, enabled: bool) -> Self {
        let mut next = self.inner.clone();
        next.compute_diagnostics = enabled;
        Self::new(next)
    }

    fn with_rate_bounds_policy(&self, policy: PyRef<PyRateBoundsPolicy>) -> Self {
        let mut next = self.inner.clone();
        next.rate_bounds_policy = policy.inner.clone();
        Self::new(next)
    }

    fn with_discount_curve(&self, config: PyRef<PyDiscountCurveSolveConfig>) -> Self {
        let mut next = self.inner.clone();
        next.discount_curve = config.inner.clone();
        Self::new(next)
    }

    fn with_hazard_curve(&self, config: PyRef<PyHazardCurveSolveConfig>) -> Self {
        let mut next = self.inner.clone();
        next.hazard_curve = config.inner.clone();
        Self::new(next)
    }

    fn with_inflation_curve(&self, config: PyRef<PyInflationCurveSolveConfig>) -> Self {
        let mut next = self.inner.clone();
        next.inflation_curve = config.inner.clone();
        Self::new(next)
    }

    fn with_rate_bounds_for_currency(&self, currency: CurrencyArg) -> Self {
        Self::new(self.inner.clone().with_rate_bounds_for_currency(currency.0))
    }

    fn effective_rate_bounds(&self, currency: CurrencyArg) -> PyRateBounds {
        PyRateBounds::new(self.inner.effective_rate_bounds(currency.0))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn conservative(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(CalibrationConfig::conservative())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn aggressive(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(CalibrationConfig::aggressive())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn fast(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(CalibrationConfig::fast())
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
            format!("compute_diagnostics={}", self.inner.compute_diagnostics),
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
    module.add_class::<PyRateBoundsPolicy>()?;
    module.add_class::<PyResidualWeightingScheme>()?;
    module.add_class::<PyRatesStepConventions>()?;
    module.add_class::<PyDiscountCurveSolveConfig>()?;
    module.add_class::<PyHazardCurveSolveConfig>()?;
    module.add_class::<PyInflationCurveSolveConfig>()?;
    module.add_class::<PyCalibrationConfig>()?;
    Ok(vec![
        "CalibrationConfig",
        "CalibrationMethod",
        "DiscountCurveSolveConfig",
        "HazardCurveSolveConfig",
        "InflationCurveSolveConfig",
        "RateBounds",
        "RateBoundsPolicy",
        "RatesStepConventions",
        "ResidualWeightingScheme",
        "SolverKind",
        "ValidationMode",
    ])
}
