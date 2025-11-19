//! Periods: fiscal configs, period identifiers, period records, and builders.
//!
//! Provides helpers to construct calendar or fiscal periods and containers for
//! downstream analysis. Use `build_periods("2024Q1..Q4")` or
//! `build_fiscal_periods(range, FiscalConfig.US_FEDERAL)` to generate plans.
use crate::errors::core_to_py;
use crate::core::utils::date_to_py;
use finstack_core::dates::{build_fiscal_periods, build_periods, FiscalConfig, Period, PeriodId};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyRef};
use std::fmt;
use std::str::FromStr;

/// Describe the starting month/day for a fiscal calendar.
///
/// Parameters
/// ----------
/// start_month : int
///     Month (1-12) where the fiscal year begins when using the constructor.
/// start_day : int
///     Day-of-month where the fiscal year begins.
///
/// Returns
/// -------
/// FiscalConfig
///     Configuration token used by :func:`build_fiscal_periods`.
#[pyclass(name = "FiscalConfig", module = "finstack.core.dates.periods", frozen)]
#[derive(Clone, Copy)]
pub struct PyFiscalConfig {
    pub(crate) inner: FiscalConfig,
}

impl PyFiscalConfig {
    pub(crate) fn new(inner: FiscalConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFiscalConfig {
    #[new]
    #[pyo3(text_signature = "(start_month, start_day)")]
    /// Create a fiscal calendar starting on the given month/day.
    fn ctor(start_month: u8, start_day: u8) -> PyResult<Self> {
        FiscalConfig::new(start_month, start_day)
            .map(Self::new)
            .map_err(core_to_py)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn CALENDAR_YEAR() -> Self {
        Self::new(FiscalConfig::calendar_year())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn US_FEDERAL() -> Self {
        Self::new(FiscalConfig::us_federal())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn UK() -> Self {
        Self::new(FiscalConfig::uk())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn JAPAN() -> Self {
        Self::new(FiscalConfig::japan())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn CANADA() -> Self {
        Self::new(FiscalConfig::canada())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn AUSTRALIA() -> Self {
        Self::new(FiscalConfig::australia())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn GERMANY() -> Self {
        Self::new(FiscalConfig::germany())
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn FRANCE() -> Self {
        Self::new(FiscalConfig::france())
    }

    #[getter]
    /// Month (1-12) the fiscal year begins.
    fn start_month(&self) -> u8 {
        self.inner.start_month
    }

    #[getter]
    /// Day-of-month the fiscal year begins.
    fn start_day(&self) -> u8 {
        self.inner.start_day
    }

    fn __repr__(&self) -> String {
        format!(
            "FiscalConfig(start_month={}, start_day={})",
            self.inner.start_month, self.inner.start_day
        )
    }
}

/// Identifier for calendar periods (quarters, months, weeks, halves, or years).
///
/// Parameters
/// ----------
/// year : int
///     Calendar year for the period.
/// index : int
///     Sub-period index such as quarter number when using helper constructors.
///
/// Returns
/// -------
/// PeriodId
///     Strongly typed period identifier.
#[pyclass(name = "PeriodId", module = "finstack.core.dates.periods", frozen)]
#[derive(Clone, Copy)]
pub struct PyPeriodId {
    pub(crate) inner: PeriodId,
}

impl PyPeriodId {
    pub(crate) fn new(inner: PeriodId) -> Self {
        Self { inner }
    }

    fn kind_label(&self) -> &'static str {
        let code = self.inner.to_string();
        if code.contains('Q') {
            "quarter"
        } else if code.contains('M') {
            "month"
        } else if code.contains('W') {
            "week"
        } else if code.contains('H') {
            "half"
        } else {
            "year"
        }
    }
}

#[pymethods]
impl PyPeriodId {
    #[classmethod]
    #[pyo3(text_signature = "(cls, year, quarter)")]
    /// Construct a period id for a specific calendar quarter.
    fn quarter(_cls: &Bound<'_, PyType>, year: i32, quarter: u8) -> PyResult<Self> {
        if !(1..=4).contains(&quarter) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Quarter must be in 1..=4",
            ));
        }
        Ok(Self::new(PeriodId::quarter(year, quarter)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, year, month)")]
    /// Construct a period id for a specific month.
    fn month(_cls: &Bound<'_, PyType>, year: i32, month: u8) -> PyResult<Self> {
        if !(1..=12).contains(&month) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Month must be in 1..=12",
            ));
        }
        Ok(Self::new(PeriodId::month(year, month)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, year, week)")]
    /// Construct a period id referencing an ISO-ish week number.
    fn week(_cls: &Bound<'_, PyType>, year: i32, week: u8) -> PyResult<Self> {
        if week == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err("Week must be >= 1"));
        }
        Ok(Self::new(PeriodId::week(year, week)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, year, half)")]
    /// Construct a period id for the first/second half of a year.
    fn half(_cls: &Bound<'_, PyType>, year: i32, half: u8) -> PyResult<Self> {
        if !(1..=2).contains(&half) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Half must be 1 or 2",
            ));
        }
        Ok(Self::new(PeriodId::half(year, half)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, year)")]
    /// Create an annual period id for the given year.
    fn annual(_cls: &Bound<'_, PyType>, year: i32) -> Self {
        Self::new(PeriodId::annual(year))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, code)")]
    /// Parse a period id from string codes like ``"2024Q3"``.
    fn parse(_cls: &Bound<'_, PyType>, code: &str) -> PyResult<Self> {
        PeriodId::from_str(code).map(Self::new).map_err(core_to_py)
    }

    #[getter]
    /// Canonical string code for the period.
    fn code(&self) -> String {
        self.inner.to_string()
    }

    #[getter]
    /// Year component of the period id.
    fn year(&self) -> i32 {
        self.inner.year
    }

    #[getter]
    /// Zero-based index within the year (quarter/month/etc.).
    fn index(&self) -> u8 {
        self.inner.index
    }

    #[getter]
    /// Human-readable classification such as ``"quarter"`` or ``"month"``.
    fn kind(&self) -> &'static str {
        self.kind_label()
    }

    fn __repr__(&self) -> String {
        format!("PeriodId('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

impl fmt::Display for PyPeriodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// Calendar or fiscal period with start/end boundaries.
///
/// Parameters
/// ----------
/// None
///     Instances are produced by :func:`build_periods` or :func:`build_fiscal_periods`.
///
/// Returns
/// -------
/// Period
///     Period record exposing id, start, end, and actual flags.
#[pyclass(name = "Period", module = "finstack.core.dates.periods", frozen)]
#[derive(Clone)]
pub struct PyPeriod {
    pub(crate) inner: Period,
}

impl PyPeriod {
    pub(crate) fn new(inner: Period) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPeriod {
    #[getter]
    /// Identifier for this period (quarter/month/etc.).
    fn id(&self) -> PyPeriodId {
        PyPeriodId::new(self.inner.id)
    }

    #[getter]
    /// Start date of the period (inclusive).
    fn start(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.start)
    }

    #[getter]
    /// End date of the period (exclusive).
    fn end(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.end)
    }

    #[getter]
    /// Whether this period is classified as "actual" (data already available).
    fn is_actual(&self) -> bool {
        self.inner.is_actual
    }

    fn __repr__(&self) -> String {
        format!(
            "Period(id='{}', start={}, end={}, actual={})",
            self.inner.id, self.inner.start, self.inner.end, self.inner.is_actual
        )
    }
}

/// Ordered collection of :class:`Period` objects.
///
/// Parameters
/// ----------
/// periods : list[Period]
///     Sequence of periods returned by the period builders.
///
/// Returns
/// -------
/// PeriodPlan
///     Container supporting iteration over periods.
#[pyclass(name = "PeriodPlan", module = "finstack.core.dates.periods", frozen)]
#[derive(Clone)]
pub struct PyPeriodPlan {
    pub(crate) periods: Vec<Period>,
}

impl PyPeriodPlan {
    pub(crate) fn new(periods: Vec<Period>) -> Self {
        Self { periods }
    }
}

#[pymethods]
impl PyPeriodPlan {
    #[getter]
    /// Period entries contained in the plan.
    fn periods(&self, py: Python<'_>) -> PyResult<Vec<Py<PyPeriod>>> {
        self.periods
            .iter()
            .cloned()
            .map(|p| Py::new(py, PyPeriod::new(p)))
            .collect()
    }

    fn __len__(&self) -> usize {
        self.periods.len()
    }

    fn __repr__(&self) -> String {
        format!("PeriodPlan({} periods)", self.periods.len())
    }
}

/// Build calendar periods from a range string.
///
/// Parameters
/// ----------
/// range : str
///     Period range expression, e.g. ``"2024Q1..Q4"`` or ``"2023M1..2023M12"``.
/// actuals_until : str, optional
///     Optional cutoff code; periods up to and including this code are marked actual.
///
/// Returns
/// -------
/// PeriodPlan
///     Ordered collection of `Period` records.
///
/// Examples
/// --------
/// >>> plan = build_periods("2024Q1..Q4", actuals_until="2024Q2")
/// >>> [p.id.code for p in plan.periods]
/// ['2024Q1', '2024Q2', '2024Q3', '2024Q4']
#[pyfunction(name = "build_periods", text_signature = "(range, actuals_until=None)")]
fn build_periods_py(range: &str, actuals_until: Option<&str>) -> PyResult<PyPeriodPlan> {
    let plan = build_periods(range, actuals_until).map_err(core_to_py)?;
    Ok(PyPeriodPlan::new(plan.periods))
}

#[pyfunction(
    name = "build_fiscal_periods",
    text_signature = "(range, config, actuals_until=None)"
)]
/// Build fiscal periods using a `FiscalConfig` (e.g. ``FiscalConfig.US_FEDERAL``).
///
/// Parameters
/// ----------
/// range : str
///     Period range expression using fiscal period codes.
/// config : FiscalConfig
///     Fiscal-year configuration (country presets available as class attributes).
/// actuals_until : str, optional
///     Optional cutoff code marking earlier periods as actual.
///
/// Returns
/// -------
/// PeriodPlan
///     Ordered collection of fiscal `Period` records.
fn build_fiscal_periods_py(
    range: &str,
    config: PyRef<PyFiscalConfig>,
    actuals_until: Option<&str>,
) -> PyResult<PyPeriodPlan> {
    let plan = build_fiscal_periods(range, config.inner, actuals_until).map_err(core_to_py)?;
    Ok(PyPeriodPlan::new(plan.periods))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "periods")?;
    module.setattr(
        "__doc__",
        "Fiscal and calendar period helpers mirroring finstack_core::dates::periods.",
    )?;
    module.add_class::<PyFiscalConfig>()?;
    module.add_class::<PyPeriodId>()?;
    module.add_class::<PyPeriod>()?;
    module.add_class::<PyPeriodPlan>()?;
    module.add_function(wrap_pyfunction!(build_periods_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_fiscal_periods_py, &module)?)?;
    let exports = [
        "FiscalConfig",
        "PeriodId",
        "Period",
        "PeriodPlan",
        "build_periods",
        "build_fiscal_periods",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
