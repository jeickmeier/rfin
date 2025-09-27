use crate::config::PyFinstackConfig;
use crate::currency::{extract_currency, PyCurrency};
use crate::error::core_to_py;
use finstack_core::money::Money;
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule, PyModuleMethods, PyTuple, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::fmt;

/// Currency-tagged monetary amount with safe arithmetic.
#[pyclass(name = "Money", module = "finstack.money")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyMoney {
    pub(crate) inner: Money,
}

impl PyMoney {
    pub(crate) fn new(inner: Money) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMoney {
    #[new]
    #[pyo3(text_signature = "(amount, currency)")]
    /// Create a money amount with the provided ISO currency.
    fn ctor(amount: f64, currency: Bound<'_, PyAny>) -> PyResult<Self> {
        let ccy = extract_currency(&currency)?;
        Ok(Self::new(Money::new(amount, ccy)))
    }

    /// Construct a money value using a configuration for ingest rounding.
    #[classmethod]
    #[pyo3(text_signature = "(cls, amount, currency, config)")]
    fn from_config(
        _cls: &Bound<'_, PyType>,
        amount: f64,
        currency: Bound<'_, PyAny>,
        config: &PyFinstackConfig,
    ) -> PyResult<Self> {
        let ccy = extract_currency(&currency)?;
        Ok(Self::new(Money::new_with_config(
            amount,
            ccy,
            &config.inner,
        )))
    }

    /// Zero amount in the requested currency.
    #[classmethod]
    #[pyo3(text_signature = "(cls, currency)")]
    fn zero(_cls: &Bound<'_, PyType>, currency: Bound<'_, PyAny>) -> PyResult<Self> {
        let ccy = extract_currency(&currency)?;
        Ok(Self::new(Money::new(0.0, ccy)))
    }

    /// Construct from a `(amount, currency)` tuple or another `Money` instance.
    #[classmethod]
    #[pyo3(text_signature = "(cls, value)")]
    fn from_tuple(_cls: &Bound<'_, PyType>, value: Bound<'_, PyAny>) -> PyResult<Self> {
        let inner = extract_money(&value)?;
        Ok(Self::new(inner))
    }

    /// Amount as floating-point value.
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// Currency of this amount.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency())
    }

    /// Return `(amount, currency_code)` tuple.
    #[pyo3(text_signature = "(self)")]
    fn to_tuple(&self) -> PyResult<(f64, PyCurrency)> {
        Ok((self.inner.amount(), PyCurrency::new(self.inner.currency())))
    }

    /// Format using ISO minor units (e.g., `USD 10.00`).
    #[pyo3(text_signature = "(self)")]
    fn format(&self) -> String {
        format!("{}", self.inner)
    }

    /// Format using a configuration (custom rounding/scales).
    #[pyo3(text_signature = "(self, config)")]
    fn format_with_config(&self, config: &PyFinstackConfig) -> String {
        self.inner.format_with_config(&config.inner)
    }

    /// Checked addition. Raises `ValueError` on currency mismatch.
    #[pyo3(text_signature = "(self, other)")]
    fn checked_add(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other)?;
        (self.inner + rhs).map(Self::new).map_err(core_to_py)
    }

    /// Checked subtraction. Raises `ValueError` on currency mismatch.
    #[pyo3(text_signature = "(self, other)")]
    fn checked_sub(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other)?;
        (self.inner - rhs).map(Self::new).map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Money(amount={amount:.6}, currency='{code}')",
            amount = self.inner.amount(),
            code = self.inner.currency()
        )
    }

    fn __str__(&self) -> String {
        self.format()
    }

    fn __hash__(&self) -> isize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.currency().hash(&mut hasher);
        // Multiply by 1e9 to include amount significance while keeping deterministic.
        let scaled = (self.inner.amount() * 1_000_000_000.0).round() as i64;
        scaled.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match extract_money(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };

        let result = match op {
            CompareOp::Eq => rhs
                .map(|v| v.amount() == self.inner.amount() && v.currency() == self.inner.currency())
                .unwrap_or(false),
            CompareOp::Ne => rhs
                .map(|v| v.amount() != self.inner.amount() || v.currency() != self.inner.currency())
                .unwrap_or(true),
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };
        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }

    fn __add__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other)?;
        (self.inner + rhs).map(Self::new).map_err(core_to_py)
    }

    fn __radd__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        self.__add__(other)
    }

    fn __sub__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other)?;
        (self.inner - rhs).map(Self::new).map_err(core_to_py)
    }

    fn __rsub__(&self, other: Bound<'_, PyAny>) -> PyResult<Self> {
        let rhs = extract_money(&other)?;
        (rhs - self.inner).map(Self::new).map_err(core_to_py)
    }

    fn __mul__(&self, factor: f64) -> Self {
        Self::new(self.inner * factor)
    }

    fn __rmul__(&self, factor: f64) -> Self {
        Self::new(self.inner * factor)
    }

    fn __truediv__(&self, divisor: f64) -> PyResult<Self> {
        if divisor == 0.0 {
            return Err(PyValueError::new_err("Cannot divide by zero"));
        }
        Ok(Self::new(self.inner / divisor))
    }

    fn __rtruediv__(&self, value: f64) -> PyResult<()> {
        let _ = value;
        Err(PyTypeError::new_err(
            "Division of scalar by Money is undefined",
        ))
    }

    /// Deconstruct into `(amount, currency_code)` tuple (used by pickle).
    fn __getnewargs__(&self) -> PyResult<(f64, PyCurrency)> {
        Ok((self.inner.amount(), PyCurrency::new(self.inner.currency())))
    }
}

impl fmt::Display for PyMoney {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "money")?;
    module.add_class::<PyMoney>()?;
    let all = PyList::new(py, ["Money"])?;
    module.setattr("__all__", all)?;
    parent.add_submodule(&module)?;
    Ok(())
}

fn extract_money(value: &Bound<'_, PyAny>) -> PyResult<Money> {
    if let Ok(mny) = value.extract::<PyRef<PyMoney>>() {
        return Ok(mny.inner);
    }

    if let Ok(tuple) = value.downcast::<PyTuple>() {
        if tuple.len() == 2 {
            let amount_item = tuple.get_item(0)?;
            let amount = amount_item.extract::<f64>()?;
            let ccy_item = tuple.get_item(1)?;
            let ccy = extract_currency(&ccy_item)?;
            return Ok(Money::new(amount, ccy));
        }
    }

    Err(PyTypeError::new_err(
        "Expected Money instance or (amount, currency) tuple",
    ))
}
