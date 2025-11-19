use crate::core::common::{labels::normalize_label, pycmp::richcmp_eq_ne};
use finstack_core::market_data::term_structures::Seniority;
use finstack_valuations::calibration::{CalibrationConfig, MultiCurveConfig, SolverKind};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use pyo3::Bound;

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SolverKind",
    frozen
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PySolverKind {
    pub(crate) inner: SolverKind,
}

impl PySolverKind {
    pub(crate) const fn new(inner: SolverKind) -> Self {
        Self { inner }
    }

    const fn discriminant(&self) -> isize {
        match self.inner {
            SolverKind::Newton => 0,
            SolverKind::Brent => 1,
            SolverKind::LevenbergMarquardt => 2,
        }
    }
}

#[pymethods]
impl PySolverKind {
    #[classattr]
    const NEWTON: Self = Self::new(SolverKind::Newton);
    #[classattr]
    const BRENT: Self = Self::new(SolverKind::Brent);
    #[classattr]
    const LEVENBERG_MARQUARDT: Self = Self::new(SolverKind::LevenbergMarquardt);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match normalize_label(name).as_str() {
            "newton" => Ok(Self::new(SolverKind::Newton)),
            "brent" => Ok(Self::new(SolverKind::Brent)),
            "levenberg_marquardt" | "levenbergmarquardt" => {
                Ok(Self::new(SolverKind::LevenbergMarquardt))
            }
            other => Err(PyKeyError::new_err(format!("Unknown solver kind: {other}"))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            SolverKind::Newton => "newton",
            SolverKind::Brent => "brent",
            SolverKind::LevenbergMarquardt => "levenberg_marquardt",
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
    ) -> PyResult<PyObject> {
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
    name = "MultiCurveConfig",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyMultiCurveConfig {
    pub(crate) inner: MultiCurveConfig,
}

impl PyMultiCurveConfig {
    pub(crate) fn new(inner: MultiCurveConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMultiCurveConfig {
    #[new]
    #[pyo3(text_signature = "(calibrate_basis=True, enforce_separation=True)")]
    fn ctor(calibrate_basis: Option<bool>, enforce_separation: Option<bool>) -> Self {
        let mut inner = MultiCurveConfig::default();
        if let Some(value) = calibrate_basis {
            inner.calibrate_basis = value;
        }
        if let Some(value) = enforce_separation {
            inner.enforce_separation = value;
        }
        Self::new(inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn new_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(MultiCurveConfig::default())
    }

    #[getter]
    fn calibrate_basis(&self) -> bool {
        self.inner.calibrate_basis
    }

    #[getter]
    fn enforce_separation(&self) -> bool {
        self.inner.enforce_separation
    }

    fn with_calibrate_basis(&self, value: bool) -> Self {
        let mut next = self.inner.clone();
        next.calibrate_basis = value;
        Self::new(next)
    }

    fn with_enforce_separation(&self, value: bool) -> Self {
        let mut next = self.inner.clone();
        next.enforce_separation = value;
        Self::new(next)
    }

    fn __repr__(&self) -> String {
        format!(
            "MultiCurveConfig(calibrate_basis={}, enforce_separation={})",
            self.inner.calibrate_basis, self.inner.enforce_separation
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

    fn map_seniority(dict: Option<Bound<'_, PyDict>>) -> PyResult<Vec<(String, Seniority)>> {
        let mut entries = Vec::new();
        if let Some(items) = dict {
            for (key, value) in items.iter() {
                let entity = key.extract::<String>()?;
                let seniority = value.extract::<String>()?;
                let sen = seniority
                    .parse()
                    .map_err(|e: String| PyValueError::new_err(e))?;
                entries.push((entity, sen));
            }
        }
        Ok(entries)
    }
}

#[pymethods]
impl PyCalibrationConfig {
    #[new]
    #[pyo3(
        text_signature = "(*, tolerance=1e-10, max_iterations=100, use_parallel=False, random_seed=42, verbose=False, solver_kind=None, multi_curve=None, entity_seniority=None)"
    )]
    fn ctor(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        use_parallel: Option<bool>,
        random_seed: Option<Option<u64>>,
        verbose: Option<bool>,
        solver_kind: Option<PyRef<PySolverKind>>,
        multi_curve: Option<PyRef<PyMultiCurveConfig>>,
        entity_seniority: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut inner = CalibrationConfig::default();
        if let Some(val) = tolerance {
            inner.tolerance = val;
        }
        if let Some(val) = max_iterations {
            inner.max_iterations = val;
        }
        if let Some(flag) = use_parallel {
            inner.use_parallel = flag;
        }
        match random_seed {
            Some(Some(seed)) => inner.random_seed = Some(seed),
            Some(None) => inner.random_seed = None,
            None => {}
        }
        if let Some(flag) = verbose {
            inner.verbose = flag;
        }
        if let Some(kind) = solver_kind {
            inner.solver_kind = kind.inner.clone();
        }
        if let Some(cfg) = multi_curve {
            inner.multi_curve = cfg.inner.clone();
        }
        inner.entity_seniority.clear();
        for (entity, seniority) in Self::map_seniority(entity_seniority)? {
            inner.entity_seniority.insert(entity, seniority);
        }
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn multi_curve(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(CalibrationConfig::multi_curve())
    }

    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[getter]
    fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[getter]
    fn use_parallel(&self) -> bool {
        self.inner.use_parallel
    }

    #[getter]
    fn random_seed(&self) -> Option<u64> {
        self.inner.random_seed
    }

    #[getter]
    fn verbose(&self) -> bool {
        self.inner.verbose
    }

    #[getter]
    fn solver_kind(&self) -> PySolverKind {
        PySolverKind::new(self.inner.solver_kind.clone())
    }

    #[getter]
    fn multi_curve_config(&self) -> PyMultiCurveConfig {
        PyMultiCurveConfig::new(self.inner.multi_curve.clone())
    }

    #[getter]
    fn entity_seniority<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (entity, seniority) in self.inner.entity_seniority.iter() {
            dict.set_item(entity, seniority.to_string())?;
        }
        Ok(dict)
    }

    fn with_tolerance(&self, tolerance: f64) -> Self {
        let mut next = self.inner.clone();
        next.tolerance = tolerance;
        Self::new(next)
    }

    fn with_max_iterations(&self, max_iterations: usize) -> Self {
        let mut next = self.inner.clone();
        next.max_iterations = max_iterations;
        Self::new(next)
    }

    fn with_parallel(&self, flag: bool) -> Self {
        let mut next = self.inner.clone();
        next.use_parallel = flag;
        Self::new(next)
    }

    fn with_random_seed(&self, seed: Option<u64>) -> Self {
        let mut next = self.inner.clone();
        next.random_seed = seed;
        Self::new(next)
    }

    fn with_verbose(&self, verbose: bool) -> Self {
        let mut next = self.inner.clone();
        next.verbose = verbose;
        Self::new(next)
    }

    fn with_solver_kind(&self, kind: PyRef<PySolverKind>) -> Self {
        let mut next = self.inner.clone();
        next.solver_kind = kind.inner.clone();
        Self::new(next)
    }

    fn with_multi_curve_config(&self, config: PyRef<PyMultiCurveConfig>) -> Self {
        let mut next = self.inner.clone();
        next.multi_curve = config.inner.clone();
        Self::new(next)
    }

    fn with_entity_seniority(&self, mapping: Bound<'_, PyDict>) -> PyResult<Self> {
        let mut next = self.inner.clone();
        next.entity_seniority.clear();
        for (entity, seniority) in Self::map_seniority(Some(mapping))? {
            next.entity_seniority.insert(entity, seniority);
        }
        Ok(Self::new(next))
    }

    fn __repr__(&self) -> PyResult<String> {
        let mut parts = vec![
            format!("tolerance={}", self.inner.tolerance),
            format!("max_iterations={}", self.inner.max_iterations),
            format!("use_parallel={}", self.inner.use_parallel),
            format!("verbose={}", self.inner.verbose),
            format!(
                "solver_kind='{}'",
                PySolverKind::new(self.inner.solver_kind.clone()).name()
            ),
        ];
        match self.inner.random_seed {
            Some(seed) => parts.push(format!("random_seed={}", seed)),
            None => parts.push("random_seed=None".to_string()),
        }
        parts.push(format!(
            "multi_curve=MultiCurveConfig(calibrate_basis={}, enforce_separation={})",
            self.inner.multi_curve.calibrate_basis, self.inner.multi_curve.enforce_separation
        ));
        Ok(format!("CalibrationConfig({})", parts.join(", ")))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PySolverKind>()?;
    module.add_class::<PyMultiCurveConfig>()?;
    module.add_class::<PyCalibrationConfig>()?;
    Ok(vec!["SolverKind", "MultiCurveConfig", "CalibrationConfig"])
}
