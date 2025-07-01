//! Python bindings for Currency type.

use pyo3::prelude::*;
use pyo3::types::PyType;
use rfin_core::primitives::currency::Currency as CoreCurrency;

/// Python wrapper for the Currency enum
#[pyclass(name = "Currency")]
#[derive(Clone)]
pub struct PyCurrency {
    inner: CoreCurrency,
}

#[pymethods]
impl PyCurrency {
    /// Create a new Currency from a string code
    #[new]
    fn new(code: &str) -> PyResult<Self> {
        let currency = code.parse::<CoreCurrency>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid currency code: {}", e))
        })?;
        Ok(PyCurrency { inner: currency })
    }

    /// Create Currency from known variants
    #[classmethod]
    fn usd(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::USD,
        }
    }

    #[classmethod]
    fn eur(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::EUR,
        }
    }

    #[classmethod]
    fn gbp(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::GBP,
        }
    }

    #[classmethod]
    fn jpy(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::JPY,
        }
    }

    #[classmethod]
    fn chf(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::CHF,
        }
    }

    #[classmethod]
    fn aud(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::AUD,
        }
    }

    #[classmethod]
    fn cad(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::CAD,
        }
    }

    /// Get the currency code as a string
    #[getter]
    fn code(&self) -> String {
        format!("{}", self.inner)
    }

    /// Get the ISO 4217 numeric code
    #[getter]
    fn numeric_code(&self) -> u16 {
        self.inner as u16
    }

    /// Get the number of minor units (decimal places)
    #[getter]
    fn minor_units(&self) -> u8 {
        self.inner.minor_units()
    }

    /// String representation
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Debug representation
    fn __repr__(&self) -> String {
        format!("Currency('{}')", self.inner)
    }

    /// Equality comparison
    fn __eq__(&self, other: &PyCurrency) -> bool {
        self.inner == other.inner
    }

    /// Hash for use in sets/dicts
    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl PyCurrency {
    /// Create a new PyCurrency from CoreCurrency (internal use)
    pub fn from_inner(inner: CoreCurrency) -> Self {
        Self { inner }
    }

    /// Get the inner Currency enum
    pub fn inner(&self) -> CoreCurrency {
        self.inner
    }
}
