use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use pyo3::basic::CompareOp;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, PyObject, PyRef};
use std::fmt;

/// Pay/receive direction for swap fixed-leg cashflows.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PayReceive",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPayReceive {
    pub(crate) inner: PayReceive,
}

impl PyPayReceive {
    pub(crate) const fn new(inner: PayReceive) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            PayReceive::PayFixed => "pay_fixed",
            PayReceive::ReceiveFixed => "receive_fixed",
        }
    }
}

#[pymethods]
impl PyPayReceive {
    #[classattr]
    const PAY_FIXED: Self = Self::new(PayReceive::PayFixed);
    #[classattr]
    const RECEIVE_FIXED: Self = Self::new(PayReceive::ReceiveFixed);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse ``"pay_fixed"`` or ``"receive_fixed"``.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<PayReceive>()
            .map(Self::new)
            .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("PayReceive('{}')", self.label())
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
        let rhs = if let Ok(value) = other.extract::<PyRef<Self>>() {
            Some(value.inner)
        } else {
            None
        };
        crate::core::common::pycmp::richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

impl fmt::Display for PyPayReceive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Plain-vanilla interest rate swap with fixed-for-floating legs.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInterestRateSwap {
    pub(crate) inner: InterestRateSwap,
}

impl PyInterestRateSwap {
    pub(crate) fn new(inner: InterestRateSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInterestRateSwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, fixed_rate, start, end)")]
    /// Create a USD SOFR swap where the caller pays fixed and receives floating.
    fn usd_pay_fixed(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        Ok(Self::new(InterestRateSwap::usd_pay_fixed(
            id, amt, fixed_rate, start_date, end_date,
        )))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, fixed_rate, start, end)")]
    /// Create a USD SOFR swap where the caller receives fixed.
    fn usd_receive_fixed(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        Ok(Self::new(InterestRateSwap::usd_receive_fixed(
            id, amt, fixed_rate, start_date, end_date,
        )))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, start, end, primary_spread_bp, reference_spread_bp)"
    )]
    /// Create a USD basis swap with spreads applied to both floating legs.
    fn usd_basis_swap(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        primary_spread_bp: f64,
        reference_spread_bp: f64,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        Ok(Self::new(InterestRateSwap::usd_basis_swap(
            id,
            amt,
            start_date,
            end_date,
            primary_spread_bp,
            reference_spread_bp,
        )))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional amount shared by both legs.
    ///
    /// Returns:
    ///     Any: Notional amount shared by both legs.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Pay/receive direction of the fixed leg.
    ///
    /// Returns:
    ///     Any: Pay/receive direction of the fixed leg.
    #[getter]
    fn side(&self) -> PyPayReceive {
        PyPayReceive::new(self.inner.side)
    }

    /// Fixed leg coupon rate.
    ///
    /// Returns:
    ///     Any: Fixed leg coupon rate.
    #[getter]
    fn fixed_rate(&self) -> f64 {
        self.inner.fixed.rate
    }

    /// Floating leg spread in basis points.
    ///
    /// Returns:
    ///     Any: Floating leg spread in basis points.
    #[getter]
    fn float_spread_bp(&self) -> f64 {
        self.inner.float.spread_bp
    }

    /// Effective start date (from fixed leg spec).
    ///
    /// Returns:
    ///     Any: Effective start date (from fixed leg spec).
    #[getter]
    fn start(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.fixed.start)
    }

    /// Effective end date (from fixed leg spec).
    ///
    /// Returns:
    ///     Any: Effective end date (from fixed leg spec).
    #[getter]
    fn end(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.fixed.end)
    }

    /// Discount curve identifier used by the fixed leg.
    ///
    /// Returns:
    ///     Any: Discount curve identifier used by the fixed leg.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.fixed.disc_id.as_str().to_string()
    }

    /// Floating forward curve identifier.
    ///
    /// Returns:
    ///     Any: Floating forward curve identifier.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.float.fwd_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.IRS``).
    ///
    /// Returns:
    ///     Any: Instrument type enum (``InstrumentType.IRS``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::IRS)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InterestRateSwap(id='{}', notional={}, side='{}')",
            self.inner.id,
            self.inner.notional,
            PyPayReceive::new(self.inner.side).label()
        ))
    }
}

impl fmt::Display for PyInterestRateSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IRS({}, rate={:.4}, side={})",
            self.inner.id,
            self.inner.fixed.rate,
            PyPayReceive::new(self.inner.side).label()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPayReceive>()?;
    module.add_class::<PyInterestRateSwap>()?;
    Ok(vec!["PayReceive", "InterestRateSwap"])
}
