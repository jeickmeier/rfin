use crate::core::currency::PyCurrency;
use crate::core::money::PyMoney;
use crate::core::utils::date_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::term_loan::TermLoan;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::Bound;
use std::fmt;

/// Term loan instrument with DDTL (Delayed Draw Term Loan) support.
///
/// A term loan is a debt instrument with a defined maturity, optional amortization,
/// and support for both fixed and floating rates. The DDTL variant allows for
/// delayed draws during an availability period with commitment fees and usage fees.
///
/// Examples:
///     >>> from finstack.valuations.instruments import TermLoan
///     >>> from finstack.core.money import Money
///     >>> from datetime import date
///     >>>
///     >>> # Create a simple fixed-rate term loan
///     >>> loan = TermLoan.from_json('''{
///     ...     "id": "loan_001",
///     ...     "discount_curve_id": "usd_discount",
///     ...     "currency": "USD",
///     ...     "issue": "2024-01-01",
///     ...     "maturity": "2029-01-01",
///     ...     "rate": {"Fixed": {"rate_bp": 500}},
///     ...     "pay_freq": {"months": 3},
///     ...     "day_count": "Act360",
///     ...     "bdc": "Following",
///     ...     "calendar_id": null,
///     ...     "stub": "None",
///     ...     "amortization": "None",
///     ...     "coupon_type": "Cash",
///     ...     "upfront_fee": null,
///     ...     "ddtl": null,
///     ...     "covenants": null,
///     ...     "oid_eir": null,
///     ...     "pricing_overrides": {},
///     ...     "call_schedule": null
///     ... }''')
///     >>> loan.instrument_id
///     'loan_001'
#[pyclass(module = "finstack.valuations.instruments", name = "TermLoan", frozen)]
#[derive(Clone, Debug)]
pub struct PyTermLoan {
    pub(crate) inner: TermLoan,
}

impl PyTermLoan {
    pub(crate) fn new(inner: TermLoan) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTermLoan {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create a term loan from a JSON string specification.
    ///
    /// The JSON should match the TermLoan schema from finstack-valuations.
    /// This is the recommended way to create complex term loans with DDTL features,
    /// covenants, and custom amortization schedules.
    ///
    /// Args:
    ///     json_str: JSON string matching the TermLoan schema.
    ///
    /// Returns:
    ///     TermLoan: Configured term loan instrument.
    ///
    /// Raises:
    ///     ValueError: If JSON cannot be parsed or is invalid.
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Serialize the term loan to a JSON string.
    ///
    /// Returns:
    ///     str: JSON representation of the term loan.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Currency for all cashflows.
    ///
    /// Returns:
    ///     Currency: Currency object.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Maximum commitment / notional limit.
    ///
    /// Returns:
    ///     Money: Notional limit wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional_limit(&self) -> PyMoney {
        PyMoney::new(self.inner.notional_limit)
    }

    /// Issue (effective) date.
    ///
    /// Returns:
    ///     datetime.date: Issue date converted to Python.
    #[getter]
    fn issue(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.issue)
    }

    /// Maturity date.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier for the discount curve.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.TERM_LOAN``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::TermLoan)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TermLoan(id='{}', issue='{}', maturity='{}')",
            self.inner.id, self.inner.issue, self.inner.maturity
        ))
    }
}

impl fmt::Display for PyTermLoan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TermLoan({}, {} -> {})",
            self.inner.id, self.inner.issue, self.inner.maturity
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTermLoan>()?;
    Ok(vec!["TermLoan"])
}
