use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::basis_swap::{BasisSwap, BasisSwapLeg};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_frequency(label: Option<&str>) -> PyResult<Tenor> {
    crate::valuations::common::parse_frequency_label(label)
}

fn parse_stub(label: Option<&str>) -> PyResult<finstack_core::dates::StubKind> {
    crate::valuations::common::parse_stub_kind(label)
}

/// Basis swap leg helper mirroring ``BasisSwapLeg``.
///
/// Examples:
///     >>> leg = BasisSwapLeg(
///     ...     "usd_libor_3m",
///     ...     spread=12.5,
///     ...     frequency="quarterly"
///     ... )
///     >>> leg.spread
///     12.5
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasisSwapLeg",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyBasisSwapLeg {
    pub(crate) inner: BasisSwapLeg,
}

#[pymethods]
impl PyBasisSwapLeg {
    #[new]
    #[pyo3(
        signature = (
            forward_curve,
            *,
            frequency=None,
            day_count=None,
            business_day_convention=None,
            spread=0.0
        ),
        text_signature = "(forward_curve, /, *, frequency='quarterly', day_count='act_360', business_day_convention='modified_following', spread=0.0)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a basis swap leg specification.
    ///
    /// Args:
    ///     forward_curve: Identifier for the forward curve.
    ///     frequency: Optional payment frequency label (e.g., ``"quarterly"``).
    ///     day_count: Optional day-count convention.
    ///     business_day_convention: Optional business-day adjustment rule.
    ///     spread: Optional spread in basis points.
    ///
    /// Returns:
    ///     BasisSwapLeg: Leg specification describing accrual and spread inputs.
    ///
    /// Raises:
    ///     ValueError: If frequency or stub labels are invalid.
    ///     TypeError: If curve identifiers or conventions cannot be parsed.
    fn new(
        forward_curve: Bound<'_, PyAny>,
        frequency: Option<&str>,
        day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        spread: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let forward_id = forward_curve.extract::<&str>().context("forward_curve")?;
        let freq = parse_frequency(frequency).context("frequency")?;
        let dc = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract().context("day_count")?;
            value
        } else {
            DayCount::Act360
        };
        let bdc = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) =
                obj.extract().context("business_day_convention")?;
            value
        } else {
            BusinessDayConvention::ModifiedFollowing
        };

        Ok(Self {
            inner: BasisSwapLeg {
                forward_curve_id: forward_id.into(),
                frequency: freq,
                day_count: dc,
                bdc,
                spread: spread.unwrap_or(0.0),
                payment_lag_days: 0,
                reset_lag_days: 0,
            },
        })
    }

    /// Forward curve identifier.
    ///
    /// Returns:
    ///     str: Identifier used to retrieve the forward curve.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    /// Spread in basis points.
    ///
    /// Returns:
    ///     float: Leg spread applied to the floating rate.
    #[getter]
    fn spread(&self) -> f64 {
        self.inner.spread
    }
}

/// Basis swap wrapper with convenience constructor.
///
/// Examples:
///     >>> swap = (
///     ...     BasisSwap.builder("basis_usd")
///     ...     .money(Money("USD", 10_000_000))
///     ...     .start_date(date(2024, 1, 2))
///     ...     .maturity(date(2027, 1, 2))
///     ...     .primary_leg(primary_leg)
///     ...     .reference_leg(reference_leg)
///     ...     .disc_id("usd_discount")
///     ...     .build()
///     ... )
///     >>> swap.notional.amount
///     10000000
#[pyclass(module = "finstack.valuations.instruments", name = "BasisSwap", frozen)]
#[derive(Clone, Debug)]
pub struct PyBasisSwap {
    pub(crate) inner: Arc<BasisSwap>,
}

impl PyBasisSwap {
    pub(crate) fn new(inner: BasisSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasisSwapBuilder",
    unsendable
)]
pub struct PyBasisSwapBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    start_date: Option<time::Date>,
    maturity: Option<time::Date>,
    primary_leg: Option<BasisSwapLeg>,
    reference_leg: Option<BasisSwapLeg>,
    discount_curve_id: Option<CurveId>,
    calendar: Option<String>,
    stub: finstack_core::dates::StubKind,
}

impl PyBasisSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            start_date: None,
            maturity: None,
            primary_leg: None,
            reference_leg: None,
            discount_curve_id: None,
            calendar: None,
            stub: finstack_core::dates::StubKind::None,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.primary_leg.is_none() {
            return Err(PyValueError::new_err("primary_leg() is required."));
        }
        if self.reference_leg.is_none() {
            return Err(PyValueError::new_err("reference_leg() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("disc_id() is required."));
        }
        Ok(())
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<finstack_core::currency::Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<finstack_core::currency::Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }
}

#[pymethods]
impl PyBasisSwapBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyResult<PyRefMut<'_, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.pending_notional_amount = Some(amount);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.pending_currency = Some(Self::parse_currency(currency)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, start_date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        start_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&start_date).context("start_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, primary_leg)")]
    fn primary_leg(mut slf: PyRefMut<'_, Self>, primary_leg: PyBasisSwapLeg) -> PyRefMut<'_, Self> {
        slf.primary_leg = Some(primary_leg.inner);
        slf
    }

    #[pyo3(text_signature = "($self, reference_leg)")]
    fn reference_leg(
        mut slf: PyRefMut<'_, Self>,
        reference_leg: PyBasisSwapLeg,
    ) -> PyRefMut<'_, Self> {
        slf.reference_leg = Some(reference_leg.inner);
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, calendar=None)", signature = (calendar=None))]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar = calendar;
        slf
    }

    #[pyo3(text_signature = "($self, stub)")]
    fn stub(mut slf: PyRefMut<'_, Self>, stub: Option<String>) -> PyResult<PyRefMut<'_, Self>> {
        slf.stub = parse_stub(stub.as_deref())?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyBasisSwap> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing notional after validation",
            )
        })?;
        let start = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing start_date after validation",
            )
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing maturity after validation",
            )
        })?;
        let primary_leg = slf.primary_leg.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing primary leg after validation",
            )
        })?;
        let reference_leg = slf.reference_leg.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing reference leg after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing discount curve after validation",
            )
        })?;

        let swap = BasisSwap::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .start_date(start)
            .maturity_date(maturity)
            .primary_leg(primary_leg)
            .reference_leg(reference_leg)
            .discount_curve_id(discount_curve_id)
            .stub_kind(slf.stub)
            .calendar_id_opt(slf.calendar.clone())
            .allow_calendar_fallback(false)
            .allow_same_curve(false)
            .attributes(Default::default())
            .build()
            .map_err(core_to_py)?;

        Ok(PyBasisSwap::new(swap))
    }

    fn __repr__(&self) -> String {
        "BasisSwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyBasisSwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyBasisSwapBuilder>> {
        let py = cls.py();
        let builder = PyBasisSwapBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the swap.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Swap notional amount.
    ///
    /// Returns:
    ///     Money: Notional expressed as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier of the discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.BasisSwap``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::BasisSwap)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("BasisSwap(id='{}')", self.inner.id))
    }
}

impl fmt::Display for PyBasisSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BasisSwap({}, notional={})",
            self.inner.id, self.inner.notional
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBasisSwapLeg>()?;
    module.add_class::<PyBasisSwap>()?;
    module.add_class::<PyBasisSwapBuilder>()?;
    Ok(vec!["BasisSwapLeg", "BasisSwap", "BasisSwapBuilder"])
}
