use super::calendar::{extract_calendar, PyBusinessDayConvention};
use super::daycount::PyDayCount;
use super::utils::{date_to_py, py_to_date};
use crate::errors::{core_to_py, PyContext};
use finstack_core::dates::{BusinessDayConvention, Tenor, TenorUnit};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::fmt;
use std::hash::{Hash, Hasher};

#[pyclass(name = "TenorUnit", module = "finstack.core.dates.tenor", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyTenorUnit {
    pub(crate) inner: TenorUnit,
}

impl PyTenorUnit {
    pub(crate) const fn new(inner: TenorUnit) -> Self {
        Self { inner }
    }

    fn label(self) -> &'static str {
        match self.inner {
            TenorUnit::Days => "days",
            TenorUnit::Weeks => "weeks",
            TenorUnit::Months => "months",
            TenorUnit::Years => "years",
        }
    }
}

#[pymethods]
impl PyTenorUnit {
    #[classattr]
    const DAYS: Self = Self::new(TenorUnit::Days);
    #[classattr]
    const WEEKS: Self = Self::new(TenorUnit::Weeks);
    #[classattr]
    const MONTHS: Self = Self::new(TenorUnit::Months);
    #[classattr]
    const YEARS: Self = Self::new(TenorUnit::Years);

    #[classmethod]
    #[pyo3(text_signature = "(cls, symbol)")]
    fn from_symbol(_cls: &Bound<'_, PyType>, symbol: &str) -> PyResult<Self> {
        let ch = symbol
            .chars()
            .next()
            .ok_or_else(|| PyTypeError::new_err("symbol must be a non-empty string"))?;
        TenorUnit::from_char(ch).map(Self::new).map_err(core_to_py)
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("TenorUnit('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match other.extract::<PyRef<PyTenorUnit>>() {
            Ok(v) => Some(v.inner),
            Err(_) => None,
        };
        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyTypeError::new_err("Unsupported comparison")),
        };
        Ok(result.into_bound_py_any(py)?.into())
    }
}

impl fmt::Display for PyTenorUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[pyclass(name = "Tenor", module = "finstack.core.dates.tenor")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyTenor {
    pub(crate) inner: Tenor,
}

impl PyTenor {
    pub(crate) const fn new(inner: Tenor) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTenor {
    #[new]
    #[pyo3(text_signature = "(count, unit)")]
    fn ctor(count: u32, unit: PyRef<PyTenorUnit>) -> Self {
        Self::new(Tenor::new(count, unit.inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, text)")]
    fn parse(_cls: &Bound<'_, PyType>, text: &str) -> PyResult<Self> {
        Tenor::parse(text).map(Self::new).map_err(core_to_py)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, years, day_count)")]
    fn from_years(_cls: &Bound<'_, PyType>, years: f64, day_count: PyRef<PyDayCount>) -> Self {
        Self::new(Tenor::from_years(years, day_count.inner))
    }

    #[staticmethod]
    fn daily() -> Self {
        Self::new(Tenor::daily())
    }

    #[staticmethod]
    fn weekly() -> Self {
        Self::new(Tenor::weekly())
    }

    #[staticmethod]
    fn biweekly() -> Self {
        Self::new(Tenor::biweekly())
    }

    #[staticmethod]
    fn monthly() -> Self {
        Self::new(Tenor::monthly())
    }

    #[staticmethod]
    fn bimonthly() -> Self {
        Self::new(Tenor::bimonthly())
    }

    #[staticmethod]
    fn quarterly() -> Self {
        Self::new(Tenor::quarterly())
    }

    #[staticmethod]
    fn semi_annual() -> Self {
        Self::new(Tenor::semi_annual())
    }

    #[staticmethod]
    fn annual() -> Self {
        Self::new(Tenor::annual())
    }

    #[getter]
    fn count(&self) -> u32 {
        self.inner.count
    }

    #[getter]
    fn unit(&self) -> PyTenorUnit {
        PyTenorUnit::new(self.inner.unit)
    }

    #[pyo3(name = "to_years_simple", text_signature = "(self)")]
    fn years_simple(&self) -> f64 {
        self.inner.to_years_simple()
    }

    #[pyo3(
        signature = (date, *, calendar=None, convention=None),
        text_signature = "(self, date, *, calendar=None, convention=None)"
    )]
    fn add_to_date(
        &self,
        py: Python<'_>,
        date: Bound<'_, PyAny>,
        calendar: Option<Bound<'_, PyAny>>,
        convention: Option<PyRef<PyBusinessDayConvention>>,
    ) -> PyResult<Py<PyAny>> {
        let base = py_to_date(&date).context("date")?;
        let cal = calendar
            .map(|c| extract_calendar(&c).context("calendar"))
            .transpose()?;
        let bdc = convention
            .map(|c| c.inner)
            .unwrap_or(BusinessDayConvention::Following);
        let result = self
            .inner
            .add_to_date(base, cal.as_ref().map(|c| c.inner), bdc)
            .map_err(core_to_py)?;
        date_to_py(py, result)
    }

    #[pyo3(
        signature = (as_of, *, calendar=None, convention=None, day_count),
        text_signature = "(self, as_of, *, calendar=None, convention=None, day_count)"
    )]
    #[pyo3(name = "to_years_with_context")]
    fn years_with_context(
        &self,
        as_of: Bound<'_, PyAny>,
        calendar: Option<Bound<'_, PyAny>>,
        convention: Option<PyRef<PyBusinessDayConvention>>,
        day_count: PyRef<PyDayCount>,
    ) -> PyResult<f64> {
        let base = py_to_date(&as_of).context("as_of")?;
        let cal = calendar
            .map(|c| extract_calendar(&c).context("calendar"))
            .transpose()?;
        let bdc = convention
            .map(|c| c.inner)
            .unwrap_or(BusinessDayConvention::Following);
        self.inner
            .to_years_with_context(base, cal.as_ref().map(|c| c.inner), bdc, day_count.inner)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!("Tenor('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.count.hash(&mut hasher);
        self.inner.unit.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match other.extract::<PyRef<PyTenor>>() {
            Ok(v) => Some(v.inner),
            Err(_) => None,
        };
        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyTypeError::new_err("Unsupported comparison")),
        };
        Ok(result.into_bound_py_any(py)?.into())
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "tenor")?;
    module.setattr(
        "__doc__",
        "Tenor parsing and calendar-aware conversions from finstack_core::dates::tenor.",
    )?;

    module.add_class::<PyTenorUnit>()?;
    module.add_class::<PyTenor>()?;

    let exports = ["TenorUnit", "Tenor"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
