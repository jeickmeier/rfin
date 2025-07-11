#![allow(clippy::useless_conversion)]
//! Python bindings for Money type.

use super::currency::PyCurrency;
use pyo3::prelude::*;
use rfin_core::error::Error;
use rfin_core::money::Money as CoreMoney;

/// Python wrapper for the Money type
#[pyclass(name = "Money")]
#[derive(Clone)]
pub struct PyMoney {
    inner: CoreMoney,
}

#[pymethods]
impl PyMoney {
    /// Create a new Money value with the specified amount and currency
    #[new]
    fn new(amount: f64, currency: &PyCurrency) -> Self {
        PyMoney {
            inner: CoreMoney::new(amount, currency.inner()),
        }
    }

    /// Get the amount
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// Get the currency
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.currency())
    }

    /// Add two Money values (same currency required)
    fn __add__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_add(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Cannot add money with different currencies: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Money addition failed: {}",
                err
            ))),
        }
    }

    /// Subtract two Money values (same currency required)
    fn __sub__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_sub(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Cannot subtract money with different currencies: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Money subtraction failed: {}",
                err
            ))),
        }
    }

    /// Multiply Money by a scalar
    fn __mul__(&self, scalar: f64) -> PyMoney {
        PyMoney {
            inner: self.inner * scalar,
        }
    }

    /// Divide Money by a scalar
    fn __truediv__(&self, scalar: f64) -> PyMoney {
        PyMoney {
            inner: self.inner / scalar,
        }
    }

    /// Right multiplication (scalar * money)
    fn __rmul__(&self, scalar: f64) -> PyMoney {
        self.__mul__(scalar)
    }

    /// String representation
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Debug representation
    fn __repr__(&self) -> String {
        format!("Money({}, {})", self.inner.amount(), self.inner.currency())
    }

    /// Format representation for f-strings
    fn __format__(&self, _format_spec: &str) -> String {
        // For now, ignore format_spec and just return the string representation
        // Future enhancement could parse format_spec for number formatting
        format!("{}", self.inner)
    }

    /// Equality comparison
    fn __eq__(&self, other: &PyMoney) -> bool {
        self.inner == other.inner
    }

    /// Convert to parts (amount, currency)
    #[allow(clippy::wrong_self_convention)]
    fn to_parts(&self) -> (f64, PyCurrency) {
        let (amount, currency) = self.inner.into_parts();
        (amount, PyCurrency::from_inner(currency))
    }

    /// Deprecated alias for `to_parts` to maintain backward compatibility.
    #[pyo3(name = "into_parts")]
    #[allow(clippy::wrong_self_convention)]
    fn into_parts_alias(&self) -> (f64, PyCurrency) {
        self.to_parts()
    }

    /// Add two Money values with explicit error handling
    fn checked_add(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_add(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Currency mismatch: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Addition failed: {}",
                err
            ))),
        }
    }

    /// Subtract two Money values with explicit error handling
    fn checked_sub(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_sub(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Currency mismatch: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Subtraction failed: {}",
                err
            ))),
        }
    }
}

impl PyMoney {
    /// Get the inner Money type
    pub fn inner(&self) -> CoreMoney {
        self.inner
    }
}
