use crate::core::common::args::BusinessDayConventionArg;
use crate::core::common::pycmp::richcmp_eq_ne;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::{calendar_not_found, core_to_py, unknown_business_day_convention, PyContext};
use finstack_core::dates::{self, adjust as core_adjust, BusinessDayConvention, CalendarMetadata, CalendarRegistry, HolidayCalendar};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyModule, PyType};
use pyo3::{Bound, IntoPyObjectExt, PyRef};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// Enumerate how dates should adjust relative to a business-day calendar.
///
/// Parameters
/// ----------
/// None
///     Instantiate via provided class attributes such as :attr:`BusinessDayConvention.FOLLOWING`.
///
/// Returns
/// -------
/// BusinessDayConvention
///     Convention token that can be supplied to scheduling helpers.
#[pyclass(name = "BusinessDayConvention", module = "finstack.core.dates", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBusinessDayConvention {
    pub(crate) inner: BusinessDayConvention,
}

impl PyBusinessDayConvention {
    pub(crate) const fn new(inner: BusinessDayConvention) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            BusinessDayConvention::Unadjusted => "unadjusted",
            BusinessDayConvention::Following => "following",
            BusinessDayConvention::ModifiedFollowing => "modified_following",
            BusinessDayConvention::Preceding => "preceding",
            BusinessDayConvention::ModifiedPreceding => "modified_preceding",
            _ => "custom",
        }
    }
}

#[pymethods]
impl PyBusinessDayConvention {
    #[classattr]
    const UNADJUSTED: Self = Self {
        inner: BusinessDayConvention::Unadjusted,
    };
    #[classattr]
    const FOLLOWING: Self = Self {
        inner: BusinessDayConvention::Following,
    };
    #[classattr]
    const MODIFIED_FOLLOWING: Self = Self {
        inner: BusinessDayConvention::ModifiedFollowing,
    };
    #[classattr]
    const PRECEDING: Self = Self {
        inner: BusinessDayConvention::Preceding,
    };
    #[classattr]
    const MODIFIED_PRECEDING: Self = Self {
        inner: BusinessDayConvention::ModifiedPreceding,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a business-day convention from a snake-case string.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_business_day_convention(name)
    }

    #[getter]
    /// Canonical snake-case name for the convention.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("BusinessDayConvention('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match extract_business_day_convention(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

impl fmt::Display for PyBusinessDayConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Holiday calendar sourced from the finstack registry.
///
/// Parameters
/// ----------
/// code : str
///     Registry identifier such as ``"usny"``.
///
/// Returns
/// -------
/// Calendar
///     Calendar object exposing business-day queries and metadata.
#[pyclass(name = "Calendar", module = "finstack.core.dates", unsendable)]
#[derive(Clone)]
pub struct PyCalendar {
    pub(crate) code: Cow<'static, str>,
    pub(crate) name: Cow<'static, str>,
    pub(crate) ignore_weekends: bool,
    pub(crate) inner: &'static dyn HolidayCalendar,
}

impl PyCalendar {
    pub(crate) fn from_metadata(metadata: CalendarMetadata, inner: &'static dyn HolidayCalendar) -> Self {
        Self {
            code: Cow::Borrowed(metadata.id),
            name: Cow::Borrowed(metadata.name),
            ignore_weekends: metadata.ignore_weekends,
            inner,
        }
    }

    pub(crate) fn fallback(code: &str, inner: &'static dyn HolidayCalendar) -> Self {
        Self {
            code: Cow::Owned(code.to_ascii_lowercase()),
            name: Cow::Owned(code.to_string()),
            ignore_weekends: false,
            inner,
        }
    }
}

#[pymethods]
impl PyCalendar {
    #[getter]
    /// Short calendar identifier (matching the registry code).
    fn code(&self) -> &str {
        self.code.as_ref()
    }

    #[getter]
    /// Descriptive calendar name from the registry.
    fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[getter]
    /// Whether weekends are ignored (i.e. Saturday/Sunday count as business days).
    fn ignore_weekends(&self) -> bool {
        self.ignore_weekends
    }

    #[pyo3(text_signature = "(self, date)")]
    /// Return ``True`` if the provided date is a business day.
    fn is_business_day(&self, date: Bound<'_, PyAny>) -> PyResult<bool> {
        let d = py_to_date(&date).context("date")?;
        Ok(self.inner.is_business_day(d))
    }

    #[pyo3(text_signature = "(self, date)")]
    /// Return ``True`` if the provided date is an observed holiday.
    fn is_holiday(&self, date: Bound<'_, PyAny>) -> PyResult<bool> {
        let d = py_to_date(&date).context("date")?;
        Ok(self.inner.is_holiday(d))
    }

    fn __repr__(&self) -> String {
        format!(
            "Calendar(code='{code}', name='{name}')",
            code = self.code(),
            name = self.name()
        )
    }

    fn __str__(&self) -> String {
        format!("{code} ({name})", code = self.code(), name = self.name())
    }

    fn __hash__(&self) -> isize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.code.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match extract_calendar(other.as_ref()) {
            Ok(value) => Some(value.code().to_string()),
            Err(_) => None,
        };

        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.code()).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.code()).unwrap_or(true),
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };
        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }
}

/// Adjust a date according to a convention and calendar.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Anchor date that may require adjustment.
/// convention : BusinessDayConvention or str
///     Convention controlling the adjustment.
/// calendar : Calendar or str
///     Calendar that defines business days.
///
/// Returns
/// -------
/// datetime.date
///     Adjusted business-day date.
#[pyfunction(name = "adjust", text_signature = "(date, convention, calendar)")]
pub(crate) fn adjust_py<'py>(
    py: Python<'py>,
    date: Bound<'py, PyAny>,
    convention: Bound<'py, PyAny>,
    calendar: Bound<'py, PyAny>,
) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    let BusinessDayConventionArg(conv) = convention.extract().context("convention")?;
    let cal = extract_calendar(&calendar).context("calendar")?;
    let adjusted = core_adjust(d, conv, cal.inner).map_err(core_to_py)?;
    date_to_py(py, adjusted)
}

/// Return all registered calendars as :class:`Calendar` instances.
///
/// Returns
/// -------
/// list[Calendar]
///     Collection of available calendars.
#[pyfunction(name = "available_calendars", text_signature = "()")]
pub(crate) fn available_calendars_py() -> PyResult<Vec<PyCalendar>> {
    let registry = CalendarRegistry::global();
    let mut out = Vec::new();
    for code in dates::available_calendars() {
        let cal = resolve_calendar(registry, code)?;
        out.push(cal);
    }
    Ok(out)
}

/// Return the list of calendar codes understood by the registry.
///
/// Returns
/// -------
/// list[str]
///     Canonical calendar identifiers.
#[pyfunction(name = "available_calendar_codes", text_signature = "()")]
pub(crate) fn available_calendar_codes_py() -> PyResult<Vec<&'static str>> {
    Ok(CalendarRegistry::global().available_ids().to_vec())
}

/// Fetch a calendar by code (case-insensitive).
///
/// Parameters
/// ----------
/// code : str
///     Calendar identifier such as ``"usny"``.
///
/// Returns
/// -------
/// Calendar
///     Calendar instance resolved from the registry.
#[pyfunction(name = "get_calendar", text_signature = "(code)")]
pub(crate) fn get_calendar_py(code: &str) -> PyResult<PyCalendar> {
    let registry = CalendarRegistry::global();
    resolve_calendar(registry, code)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBusinessDayConvention>()?;
    module.add_class::<PyCalendar>()?;
    module.add_function(wrap_pyfunction!(adjust_py, module)?)?;
    module.add_function(wrap_pyfunction!(available_calendars_py, module)?)?;
    module.add_function(wrap_pyfunction!(available_calendar_codes_py, module)?)?;
    module.add_function(wrap_pyfunction!(get_calendar_py, module)?)?;
    let exports = [
        "BusinessDayConvention",
        "Calendar",
        "adjust",
        "available_calendars",
        "available_calendar_codes",
        "get_calendar",
    ];
    Ok(exports.to_vec())
}

fn parse_business_day_convention(name: &str) -> PyResult<PyBusinessDayConvention> {
    BusinessDayConvention::from_str(name)
        .map(PyBusinessDayConvention::new)
        .map_err(|_| unknown_business_day_convention(name))
}

fn extract_business_day_convention(value: &Bound<'_, PyAny>) -> PyResult<BusinessDayConvention> {
    if let Ok(conv) = value.extract::<PyRef<PyBusinessDayConvention>>() {
        return Ok(conv.inner);
    }
    if let Ok(name) = value.extract::<&str>() {
        return parse_business_day_convention(name).map(|wrapper| wrapper.inner);
    }
    Err(PyTypeError::new_err(
        "Expected BusinessDayConvention or string identifier",
    ))
}

pub(crate) fn extract_calendar(value: &Bound<'_, PyAny>) -> PyResult<PyCalendar> {
    if let Ok(cal) = value.extract::<PyRef<PyCalendar>>() {
        return Ok(cal.clone());
    }

    if let Ok(code) = value.extract::<&str>() {
        let registry = CalendarRegistry::global();
        return resolve_calendar(registry, code);
    }

    Err(PyTypeError::new_err(
        "Expected Calendar instance or calendar code",
    ))
}

fn resolve_calendar(registry: &CalendarRegistry<'static>, code: &str) -> PyResult<PyCalendar> {
    let norm = code.to_ascii_lowercase();
    let calendar = registry
        .resolve_str(&norm)
        .ok_or_else(|| calendar_not_found(code))?;

    if let Some(meta) = calendar.metadata() {
        return Ok(PyCalendar::from_metadata(meta, calendar));
    }

    Ok(PyCalendar::fallback(&norm, calendar))
}
