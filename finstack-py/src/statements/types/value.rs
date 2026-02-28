//! AmountOrScalar binding.

use crate::core::currency::PyCurrency;
use finstack_core::money::Money;
use finstack_statements::types::AmountOrScalar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::Bound;

/// Union type for scalar values or currency amounts.
///
/// Used in statement models to represent values that can be either:
/// - Scalar: Dimensionless numbers (ratios, percentages, counts)
/// - Amount: Currency-denominated values (Money)
#[pyclass(
    module = "finstack.statements.types",
    name = "AmountOrScalar",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAmountOrScalar {
    pub(crate) inner: AmountOrScalar,
}

impl PyAmountOrScalar {
    pub(crate) fn new(inner: AmountOrScalar) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAmountOrScalar {
    #[classmethod]
    #[pyo3(text_signature = "(cls, value)")]
    /// Create a scalar (dimensionless) value.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Numeric value
    ///
    /// Returns
    /// -------
    /// AmountOrScalar
    ///     Scalar value
    fn scalar(_cls: &Bound<'_, PyType>, value: f64) -> Self {
        Self::new(AmountOrScalar::Scalar(value))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, value, currency)")]
    /// Create a currency-denominated amount.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Numeric value
    /// currency : Currency
    ///     Currency code
    ///
    /// Returns
    /// -------
    /// AmountOrScalar
    ///     Currency amount
    fn amount(_cls: &Bound<'_, PyType>, value: f64, currency: &PyCurrency) -> Self {
        let money = Money::new(value, currency.inner);
        Self::new(AmountOrScalar::Amount(money))
    }

    #[getter]
    /// Check if this is a scalar value.
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if scalar, False if amount
    fn is_scalar(&self) -> bool {
        matches!(self.inner, AmountOrScalar::Scalar(_))
    }

    #[getter]
    /// Get the numeric value.
    ///
    /// Returns
    /// -------
    /// float
    ///     Numeric value
    fn value(&self) -> f64 {
        match &self.inner {
            AmountOrScalar::Scalar(v) => *v,
            AmountOrScalar::Amount(m) => m.amount(),
        }
    }

    #[getter]
    /// Get the currency if this is an amount.
    ///
    /// Returns
    /// -------
    /// Currency | None
    ///     Currency if amount, None if scalar
    fn currency(&self) -> Option<PyCurrency> {
        match &self.inner {
            AmountOrScalar::Scalar(_) => None,
            AmountOrScalar::Amount(m) => Some(PyCurrency::new(m.currency())),
        }
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// AmountOrScalar
    ///     Deserialized value
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            AmountOrScalar::Scalar(v) => format!("AmountOrScalar.scalar({})", v),
            AmountOrScalar::Amount(m) => {
                format!("AmountOrScalar.amount({}, {})", m.amount(), m.currency())
            }
        }
    }

    fn __str__(&self) -> String {
        match &self.inner {
            AmountOrScalar::Scalar(v) => format!("{}", v),
            AmountOrScalar::Amount(m) => format!("{} {}", m.amount(), m.currency()),
        }
    }
}

pub(crate) fn register<'py>(_py: Python<'py>, module: &Bound<'py, PyModule>) -> PyResult<()> {
    module.add_class::<PyAmountOrScalar>()?;
    Ok(())
}
