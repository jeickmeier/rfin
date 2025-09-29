use crate::core::common::labels::normalize_label;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::cds::{CreditDefaultSwap, PayReceive};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, PyObject, PyRef};
use std::fmt;

/// Pay/receive indicator for CDS premium leg.
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
            PayReceive::PayProtection => "pay_protection",
            PayReceive::ReceiveProtection => "receive_protection",
        }
    }
}

#[pymethods]
impl PyCdsPayReceive {
    #[classattr]
    const PAY_PROTECTION: Self = Self::new(PayReceive::PayProtection);
    #[classattr]
    const RECEIVE_PROTECTION: Self = Self::new(PayReceive::ReceiveProtection);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        normalize_cds_side(name).map(Self::new)
    }

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
    match normalize_label(name).as_str() {
        "pay_protection" | "buyer" | "buy" => Ok(PayReceive::PayProtection),
        "receive_protection" | "seller" | "sell" => Ok(PayReceive::ReceiveProtection),
        other => Err(PyValueError::new_err(format!(
            "Unknown CDS pay/receive label: {other}",
        ))),
    }
}

/// Credit default swap wrapper with helper constructors.
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
            PayReceive::PayProtection,
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
            PayReceive::ReceiveProtection,
            recovery_rate,
            settlement_delay,
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn side(&self) -> PyCdsPayReceive {
        PyCdsPayReceive::new(self.inner.side)
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn spread_bp(&self) -> f64 {
        self.inner.premium.spread_bp
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.premium.disc_id.as_str().to_string()
    }

    #[getter]
    fn credit_curve(&self) -> String {
        self.inner.protection.credit_id.as_str().to_string()
    }

    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.protection.recovery_rate
    }

    #[getter]
    fn settlement_delay(&self) -> u16 {
        self.inner.protection.settlement_delay
    }

    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.premium.start)
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.premium.end)
    }

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
    let id = extract_instrument_id(&instrument_id)?;
    let amt = extract_money(&notional)?;
    let start = py_to_date(&start_date)?;
    let end = py_to_date(&maturity)?;
    let disc = extract_curve_id(&discount_curve)?;
    let credit = extract_curve_id(&credit_curve)?;

    let builder_result = match side {
        PayReceive::PayProtection => {
            CreditDefaultSwap::buy_protection(id.clone(), amt, spread_bp, start, end, disc, credit)
        }
        PayReceive::ReceiveProtection => {
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
