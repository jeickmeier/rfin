use crate::error::{calendar_not_found, core_to_py, unknown_business_day_convention};
use crate::utils::{date_to_py, py_to_date};
use finstack_core::dates::calendar::business_days::{self, HolidayCalendar};
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::{self, adjust as core_adjust, BusinessDayConvention};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::borrow::Cow;
use std::fmt;

/// Business-day convention wrapper.
#[pyclass(name = "BusinessDayConvention", module = "finstack.dates", frozen)]
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
    /// Look up a business-day convention by snake-case name.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_business_day_convention(name)
    }

    #[getter]
    /// Snake-case identifier of this convention.
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
    ) -> PyResult<PyObject> {
        let rhs = match extract_business_day_convention(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };

        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };

        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }
}

impl fmt::Display for PyBusinessDayConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Holiday calendar wrapper backed by `finstack-core` calendars.
#[pyclass(name = "Calendar", module = "finstack.dates", unsendable)]
#[derive(Clone)]
pub struct PyCalendar {
    pub(crate) code: Cow<'static, str>,
    pub(crate) name: Cow<'static, str>,
    pub(crate) ignore_weekends: bool,
    pub(crate) inner: &'static dyn HolidayCalendar,
}

impl PyCalendar {
    pub(crate) fn from_metadata(
        metadata: business_days::CalendarMetadata,
        inner: &'static dyn HolidayCalendar,
    ) -> Self {
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
    /// Lowercase identifier for this calendar.
    fn code(&self) -> &str {
        self.code.as_ref()
    }

    #[getter]
    /// Human-readable market name for this calendar.
    fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[getter]
    /// Whether the underlying calendar ignores weekends when classifying holidays.
    fn ignore_weekends(&self) -> bool {
        self.ignore_weekends
    }

    /// Return `True` if the given date is a business day.
    #[pyo3(text_signature = "(self, date)")]
    fn is_business_day(&self, date: Bound<'_, PyAny>) -> PyResult<bool> {
        let d = py_to_date(&date)?;
        Ok(self.inner.is_business_day(d))
    }

    /// Return `True` if the given date is a holiday.
    #[pyo3(text_signature = "(self, date)")]
    fn is_holiday(&self, date: Bound<'_, PyAny>) -> PyResult<bool> {
        let d = py_to_date(&date)?;
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
    ) -> PyResult<PyObject> {
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

/// Adjust a date to a business day under the given convention and calendar.
#[pyfunction]
#[pyo3(name = "adjust", text_signature = "(date, convention, calendar)")]
fn adjust_py<'py>(
    py: Python<'py>,
    date: Bound<'py, PyAny>,
    convention: Bound<'py, PyAny>,
    calendar: Bound<'py, PyAny>,
) -> PyResult<PyObject> {
    let d = py_to_date(&date)?;
    let conv = extract_business_day_convention(&convention)?;
    let cal = extract_calendar(&calendar)?;
    let adjusted = core_adjust(d, conv, cal.inner).map_err(core_to_py)?;
    date_to_py(py, adjusted)
}

/// List all built-in calendars compiled into the library.
#[pyfunction]
#[pyo3(name = "available_calendars", text_signature = "()")]
fn available_calendars_py() -> PyResult<Vec<PyCalendar>> {
    let registry = CalendarRegistry::global();
    let mut out = Vec::new();
    for code in dates::available_calendars() {
        let cal = resolve_calendar(registry, code)?;
        out.push(cal);
    }
    Ok(out)
}

/// List the available calendar identifiers (lowercase codes).
#[pyfunction]
#[pyo3(name = "available_calendar_codes", text_signature = "()")]
fn available_calendar_codes_py() -> PyResult<Vec<&'static str>> {
    Ok(CalendarRegistry::global().available_ids().to_vec())
}

/// Resolve a calendar by its identifier (case-insensitive).
#[pyfunction]
#[pyo3(name = "get_calendar", text_signature = "(code)")]
fn get_calendar_py(code: &str) -> PyResult<PyCalendar> {
    let registry = CalendarRegistry::global();
    resolve_calendar(registry, code)
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "dates")?;
    module.setattr(
        "__doc__",
        "Business-day conventions and holiday calendar utilities.",
    )?;
    module.add_class::<PyBusinessDayConvention>()?;
    module.add_class::<PyCalendar>()?;
    module.add_function(wrap_pyfunction!(adjust_py, &module)?)?;
    module.add_function(wrap_pyfunction!(available_calendars_py, &module)?)?;
    module.add_function(wrap_pyfunction!(available_calendar_codes_py, &module)?)?;
    module.add_function(wrap_pyfunction!(get_calendar_py, &module)?)?;
    let all = PyList::new(
        py,
        [
            "BusinessDayConvention",
            "Calendar",
            "adjust",
            "available_calendars",
            "available_calendar_codes",
            "get_calendar",
        ],
    )?;
    module.setattr("__all__", all)?;
    parent.add_submodule(&module)?;
    Ok(())
}

fn parse_business_day_convention(name: &str) -> PyResult<PyBusinessDayConvention> {
    match name.to_ascii_lowercase().as_str() {
        "unadjusted" => Ok(PyBusinessDayConvention::new(
            BusinessDayConvention::Unadjusted,
        )),
        "following" => Ok(PyBusinessDayConvention::new(
            BusinessDayConvention::Following,
        )),
        "modified_following" | "modified-following" => Ok(PyBusinessDayConvention::new(
            BusinessDayConvention::ModifiedFollowing,
        )),
        "preceding" => Ok(PyBusinessDayConvention::new(
            BusinessDayConvention::Preceding,
        )),
        "modified_preceding" | "modified-preceding" => Ok(PyBusinessDayConvention::new(
            BusinessDayConvention::ModifiedPreceding,
        )),
        other => Err(unknown_business_day_convention(other)),
    }
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

fn extract_calendar(value: &Bound<'_, PyAny>) -> PyResult<PyCalendar> {
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
