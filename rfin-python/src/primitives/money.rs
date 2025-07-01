//! Python bindings for Money type.

use pyo3::prelude::*;
use pyo3::types::PyType;
use rfin_core::primitives::money::Money as CoreMoney;
use super::currency::PyCurrency;

/// Python wrapper for the Money type
#[pyclass(name = "Money")]
#[derive(Clone)]
pub struct PyMoney {
    inner: CoreMoney<f64>,
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

    /// Create Money in USD
    #[classmethod]
    fn usd(_cls: &Bound<'_, PyType>, amount: f64) -> Self {
        PyMoney {
            inner: CoreMoney::usd(amount),
        }
    }

    /// Create Money in EUR
    #[classmethod]
    fn eur(_cls: &Bound<'_, PyType>, amount: f64) -> Self {
        PyMoney {
            inner: CoreMoney::eur(amount),
        }
    }

    /// Get the amount
    #[getter]
    fn amount(&self) -> f64 {
        *self.inner.amount()
    }

    /// Get the currency
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.currency())
    }

    /// Add two Money values (same currency required)
    fn __add__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match std::panic::catch_unwind(|| self.inner + other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Cannot add money with different currencies: {} and {}",
                self.inner.currency(),
                other.inner.currency()
            ))),
        }
    }

    /// Subtract two Money values (same currency required)
    fn __sub__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match std::panic::catch_unwind(|| self.inner - other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Cannot subtract money with different currencies: {} and {}",
                self.inner.currency(),
                other.inner.currency()
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
    fn into_parts(&self) -> (f64, PyCurrency) {
        let (amount, currency) = self.inner.into_parts();
        (amount, PyCurrency::from_inner(currency))
    }
}

impl PyMoney {
    /// Get the inner Money type
    pub fn inner(&self) -> CoreMoney<f64> {
        self.inner
    }
}