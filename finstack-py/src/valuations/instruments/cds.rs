use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{PyInstrumentType};
use finstack_valuations::instruments::cds::{CreditDefaultSwap, PayReceive};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, PyObject, PyRef};
use std::fmt;
use finstack_core::types::{CurveId, InstrumentId};

/// Pay/receive indicator for CDS premium leg.
///
/// Examples:
///     >>> CDSPayReceive.from_name("buy")
///     CDSPayReceive('pay_protection')
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CDSPayReceive",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyCdsPayReceive {
    pub(crate) inner: PayReceive,
}

impl PyCdsPayReceive {
    pub(crate) const fn new(inner: PayReceive) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            PayReceive::PayFixed => "pay_protection",
            PayReceive::ReceiveFixed => "receive_protection",
        }
    }
}

#[pymethods]
impl PyCdsPayReceive {
    #[classattr]
    const PAY_PROTECTION: Self = Self::new(PayReceive::PayFixed);
    #[classattr]
    const RECEIVE_PROTECTION: Self = Self::new(PayReceive::ReceiveFixed);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a textual label into a pay/receive indicator.
    ///
    /// Args:
    ///     name: Label such as ``"buy"`` or ``"sell"``.
    ///
    /// Returns:
    ///     CDSPayReceive: Enumeration corresponding to the label.
    ///
    /// Raises:
    ///     ValueError: If the label is not recognized.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<PayReceive>()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    /// Canonical snake-case name.
    ///
    /// Returns:
    ///     str: Canonical indicator label.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("CDSPayReceive('{}')", self.label())
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
        let rhs = other
            .extract::<PyRef<Self>>()
            .ok()
            .map(|ref_obj| ref_obj.inner);
        crate::core::common::pycmp::richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

impl fmt::Display for PyCdsPayReceive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

pub(crate) fn normalize_cds_side(name: &str) -> PyResult<PayReceive> {
    name.parse().map_err(|e: String| PyValueError::new_err(e))
}

/// Credit default swap wrapper with helper constructors.
///
/// Examples:
///     >>> cds = CreditDefaultSwap.buy_protection(
///     ...     "cds_xyz",
///     ...     Money("USD", 10_000_000),
///     ...     120.0,
///     ...     date(2024, 1, 1),
///     ...     date(2029, 1, 1),
///     ...     "usd_discount",
///     ...     "xyz_hazard"
///     ... )
///     >>> cds.spread_bp
///     120.0
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CreditDefaultSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCreditDefaultSwap {
    pub(crate) inner: CreditDefaultSwap,
}

impl PyCreditDefaultSwap {
    pub(crate) fn new(inner: CreditDefaultSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditDefaultSwap {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            spread_bp,
            start_date,
            maturity,
            discount_curve,
            credit_curve,
            *,
            recovery_rate=None,
            settlement_delay=None
        ),
        text_signature = "(cls, instrument_id, notional, spread_bp, start_date, maturity, discount_curve, credit_curve, /, *, recovery_rate=None, settlement_delay=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CDS where the caller buys protection (pays premium, receives protection).
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     spread_bp: Premium spread in basis points.
    ///     start_date: Start date of premium payments.
    ///     maturity: Protection maturity date.
    ///     discount_curve: Discount curve identifier.
    ///     credit_curve: Credit curve identifier.
    ///     recovery_rate: Optional recovery rate override.
    ///     settlement_delay: Optional settlement delay in days.
    ///
    /// Returns:
    ///     CreditDefaultSwap: Configured CDS instrument with pay-protection side.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn buy_protection(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        spread_bp: f64,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        credit_curve: Bound<'_, PyAny>,
        recovery_rate: Option<f64>,
        settlement_delay: Option<u16>,
    ) -> PyResult<Self> {
        construct_cds(
            instrument_id,
            notional,
            spread_bp,
            start_date,
            maturity,
            discount_curve,
            credit_curve,
            PayReceive::PayFixed,
            recovery_rate,
            settlement_delay,
        )
    }

    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            spread_bp,
            start_date,
            maturity,
            discount_curve,
            credit_curve,
            *,
            recovery_rate=None,
            settlement_delay=None
        ),
        text_signature = "(cls, instrument_id, notional, spread_bp, start_date, maturity, discount_curve, credit_curve, /, *, recovery_rate=None, settlement_delay=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CDS where the caller sells protection (receives premium).
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     spread_bp: Premium spread in basis points.
    ///     start_date: Start date of premium payments.
    ///     maturity: Protection maturity date.
    ///     discount_curve: Discount curve identifier.
    ///     credit_curve: Credit curve identifier.
    ///     recovery_rate: Optional recovery rate override.
    ///     settlement_delay: Optional settlement delay in days.
    ///
    /// Returns:
    ///     CreditDefaultSwap: Configured CDS instrument with receive-protection side.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn sell_protection(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        spread_bp: f64,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        credit_curve: Bound<'_, PyAny>,
        recovery_rate: Option<f64>,
        settlement_delay: Option<u16>,
    ) -> PyResult<Self> {
        construct_cds(
            instrument_id,
            notional,
            spread_bp,
            start_date,
            maturity,
            discount_curve,
            credit_curve,
            PayReceive::ReceiveFixed,
            recovery_rate,
            settlement_delay,
        )
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the CDS.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Pay/receive side of the trade.
    ///
    /// Returns:
    ///     CDSPayReceive: Enumeration describing protection direction.
    #[getter]
    fn side(&self) -> PyCdsPayReceive {
        PyCdsPayReceive::new(self.inner.side)
    }

    /// Notional principal.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Premium spread in basis points.
    ///
    /// Returns:
    ///     float: Premium spread for the CDS.
    #[getter]
    fn spread_bp(&self) -> f64 {
        self.inner.premium.spread_bp
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for premium leg.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.premium.discount_curve_id.as_str().to_string()
    }

    /// Credit curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve used for protection leg.
    #[getter]
    fn credit_curve(&self) -> String {
        self.inner.protection.credit_curve_id.as_str().to_string()
    }

    /// Recovery rate applied upon default.
    ///
    /// Returns:
    ///     float: Recovery rate expressed as decimal.
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.protection.recovery_rate
    }

    /// Settlement delay in days.
    ///
    /// Returns:
    ///     int: Settlement delay between default and payout.
    #[getter]
    fn settlement_delay(&self) -> u16 {
        self.inner.protection.settlement_delay
    }

    /// Start date of premium payments.
    ///
    /// Returns:
    ///     datetime.date: Start date converted to Python.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.premium.start)
    }

    /// Protection maturity date.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.premium.end)
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDS)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CreditDefaultSwap(id='{}', side='{}', spread_bp={:.1})",
            self.inner.id,
            PyCdsPayReceive::new(self.inner.side).name(),
            self.inner.premium.spread_bp
        ))
    }
}

impl fmt::Display for PyCreditDefaultSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CDS({}, side={}, spread_bp={:.1})",
            self.inner.id,
            PyCdsPayReceive::new(self.inner.side).name(),
            self.inner.premium.spread_bp
        )
    }
}

fn construct_cds(
    instrument_id: Bound<'_, PyAny>,
    notional: Bound<'_, PyAny>,
    spread_bp: f64,
    start_date: Bound<'_, PyAny>,
    maturity: Bound<'_, PyAny>,
    discount_curve: Bound<'_, PyAny>,
    credit_curve: Bound<'_, PyAny>,
    side: PayReceive,
    recovery_rate: Option<f64>,
    settlement_delay: Option<u16>,
) -> PyResult<PyCreditDefaultSwap> {
    let id = InstrumentId::new(instrument_id.extract::<&str>()?);
    let amt = extract_money(&notional)?;
    let start = py_to_date(&start_date)?;
    let end = py_to_date(&maturity)?;
    let disc = CurveId::new(discount_curve.extract::<&str>()?);
    let credit = credit_curve.extract::<&str>()?;

    let builder_result = match side {
        PayReceive::PayFixed => {
            CreditDefaultSwap::buy_protection(id.clone(), amt, spread_bp, start, end, disc, credit)
        }
        PayReceive::ReceiveFixed => {
            CreditDefaultSwap::sell_protection(id.clone(), amt, spread_bp, start, end, disc, credit)
        }
    };

    let mut cds = builder_result;
    if let Some(rr) = recovery_rate {
        cds.protection.recovery_rate = rr;
    }
    if let Some(delay) = settlement_delay {
        cds.protection.settlement_delay = delay;
    }

    Ok(PyCreditDefaultSwap::new(cds))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsPayReceive>()?;
    module.add_class::<PyCreditDefaultSwap>()?;
    Ok(vec!["CDSPayReceive", "CreditDefaultSwap"])
}
