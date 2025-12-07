//! Currency bindings: ISO-4217 metadata and helpers for Python.
//!
//! Exposes `Currency` as a Python class with convenient constructors and
//! properties. All arithmetic using `Money` requires matching currencies; this
//! module provides the canonical way to identify currencies via codes or
//! numeric identifiers. The module also re-exports every known currency by code
//! for ergonomic access (e.g., `finstack.core.currency.USD`).
use crate::errors::unknown_currency;
use finstack_core::currency::Currency;
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::fmt;
use std::str::FromStr;
use strum::IntoEnumIterator;

/// Wrap ISO-4217 currency metadata for Python usage.
///
/// Parameters
/// ----------
/// code : str
///     Three-letter ISO code such as ``"USD"`` or ``"eur"``.
///
/// Returns
/// -------
/// Currency
///     Strongly typed currency object used throughout the bindings.
#[pyclass(name = "Currency", module = "finstack.core.currency", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyCurrency {
    pub(crate) inner: Currency,
}

impl PyCurrency {
    pub(crate) fn new(inner: Currency) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCurrency {
    #[new]
    #[pyo3(text_signature = "(code)")]
    /// Construct a currency from a three-letter ISO code (case-insensitive).
    ///
    /// Parameters
    /// ----------
    /// code : str
    ///     Three-letter ISO currency code such as ``"USD"``.
    ///
    /// Returns
    /// -------
    /// Currency
    ///     Currency instance corresponding to ``code``.
    fn ctor(code: &str) -> PyResult<Self> {
        Currency::from_str(code)
            .map(Self::new)
            .map_err(|_| unknown_currency(code))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, numeric)")]
    /// Construct from an ISO numeric currency code (e.g. ``840 → USD``).
    ///
    /// Parameters
    /// ----------
    /// numeric : int
    ///     ISO-4217 numeric currency code.
    ///
    /// Returns
    /// -------
    /// Currency
    ///     Currency instance associated with ``numeric``.
    fn from_numeric(_cls: &Bound<'_, PyType>, numeric: u16) -> PyResult<Self> {
        Currency::try_from(numeric)
            .map(Self::new)
            .map_err(|_| PyValueError::new_err(format!("Unknown currency numeric code: {numeric}")))
    }

    #[getter]
    /// Three-letter currency code (always upper-case).
    ///
    /// Returns
    /// -------
    /// str
    ///     Upper-case ISO code (e.g. ``"USD"``).
    fn code(&self) -> String {
        self.inner.to_string()
    }

    #[getter]
    /// ISO numeric currency code.
    ///
    /// Returns
    /// -------
    /// int
    ///     Numeric ISO identifier (e.g. ``840`` for USD).
    fn numeric(&self) -> u16 {
        self.inner as u16
    }

    #[getter]
    /// Number of decimal places (minor units) for the currency.
    ///
    /// Returns
    /// -------
    /// int
    ///     Minor unit precision (e.g. ``2`` for USD, ``0`` for JPY).
    fn decimals(&self) -> u8 {
        self.inner.decimals()
    }

    #[pyo3(text_signature = "(self)")]
    /// Return this currency as a tuple of `(code, numeric, decimals)`.
    ///
    /// Returns
    /// -------
    /// tuple[str, int, int]
    ///     Tuple containing ISO code, numeric id, and decimal count.
    fn to_tuple(&self) -> (String, u16, u8) {
        (self.code(), self.numeric(), self.decimals())
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// List all ISO currencies compiled into the bindings.
    ///
    /// Returns
    /// -------
    /// list[Currency]
    ///     Sequence of all currencies recognised by finstack.
    fn all(_cls: &Bound<'_, PyType>) -> Vec<Self> {
        Currency::iter().map(Self::new).collect()
    }

    fn __repr__(&self) -> String {
        format!("Currency('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.numeric() as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match extract_currency(&other) {
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

/// Look up a currency by ISO code.
///
/// Parameters
/// ----------
/// code : str
///     Three-letter ISO currency code (case-insensitive).
///
/// Returns
/// -------
/// Currency
///     Currency instance matching ``code``.
#[pyfunction]
#[pyo3(name = "get", text_signature = "(code)")]
fn get_currency(code: &str) -> PyResult<PyCurrency> {
    Currency::from_str(code)
        .map(PyCurrency::new)
        .map_err(|_| unknown_currency(code))
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "currency")?;
    module.setattr(
        "__doc__",
        "ISO-4217 currency helpers used across finstack calculations.",
    )?;
    module.add_class::<PyCurrency>()?;
    module.add_function(wrap_pyfunction!(get_currency, &module)?)?;

    let mut exported = Vec::new();
    for currency in Currency::iter() {
        let code = currency.to_string();
        let instance = Py::new(py, PyCurrency::new(currency))?;
        module.add(&code, instance.clone_ref(py))?;
        exported.push(code);
    }

    let all = PyList::new(py, &exported)?;
    module.setattr("__all__", all)?;
    parent.add_submodule(&module)?;
    Ok(())
}

/// Attempt to coerce a Python value to a core `Currency`.
///
/// Accepted inputs are a `Currency` instance or a string ISO code (case-insensitive).
/// Returns a `Currency` on success, otherwise raises a `TypeError` or `ValueError`.
pub(crate) fn extract_currency(value: &Bound<'_, PyAny>) -> PyResult<Currency> {
    if let Ok(curr) = value.extract::<PyRef<PyCurrency>>() {
        return Ok(curr.inner);
    }

    if let Ok(code) = value.extract::<&str>() {
        return Currency::from_str(code)
            .map_err(|_| unknown_currency(code))
            .map(|c| c);
    }

    Err(PyTypeError::new_err(
        "Expected Currency instance or ISO currency code string",
    ))
}

impl fmt::Display for PyCurrency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}
