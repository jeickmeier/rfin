use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::rates::basis_swap::{BasisSwap, BasisSwapLeg};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::fmt;
use std::sync::Arc;

fn parse_frequency(label: Option<&str>) -> PyResult<Tenor> {
    crate::valuations::common::parse_frequency_label(label)
}

fn parse_stub(label: Option<&str>) -> PyResult<StubKind> {
    crate::valuations::common::parse_stub_kind(label)
}

/// Basis swap leg helper mirroring ``BasisSwapLeg``.
///
/// Each leg owns its own dates, discount curve, calendar, and stub conventions.
///
/// Examples:
///     >>> from datetime import date
///     >>> leg = BasisSwapLeg(
///     ...     "usd_sofr_3m",
///     ...     discount_curve="usd_ois",
///     ...     start_date=date(2024, 1, 3),
///     ...     end_date=date(2025, 1, 3),
///     ...     spread_bp=12.5,
///     ...     frequency="quarterly",
///     ... )
///     >>> leg.spread_bp
///     12.5
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasisSwapLeg",
    frozen,
    from_py_object
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
            discount_curve,
            start_date,
            end_date,
            frequency=None,
            day_count=None,
            business_day_convention=None,
            calendar_id=None,
            stub=None,
            spread_bp=0.0,
            payment_lag_days=0,
            reset_lag_days=0,
        ),
        text_signature = "(forward_curve, /, *, discount_curve, start_date, end_date, frequency='quarterly', day_count='act_360', business_day_convention='modified_following', calendar_id=None, stub='short_front', spread_bp=0.0, payment_lag_days=0, reset_lag_days=0)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a basis swap leg specification.
    ///
    /// Args:
    ///     forward_curve: Identifier for the forward curve.
    ///     discount_curve: Identifier for the discount curve.
    ///     start_date: Leg start date.
    ///     end_date: Leg end date.
    ///     frequency: Optional payment frequency label (e.g., ``"quarterly"``).
    ///     day_count: Optional day-count convention.
    ///     business_day_convention: Optional business-day adjustment rule.
    ///     calendar_id: Optional calendar for business day adjustments.
    ///     stub: Optional stub convention (e.g., ``"short_front"``).
    ///     spread_bp: Spread in basis points (default 0).
    ///     payment_lag_days: Payment lag in business days (default 0).
    ///     reset_lag_days: Reset lag in business days (default 0).
    ///
    /// Returns:
    ///     BasisSwapLeg: Leg specification describing accrual, curve, and schedule inputs.
    fn new(
        forward_curve: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        frequency: Option<&str>,
        day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        calendar_id: Option<String>,
        stub: Option<&str>,
        spread_bp: Option<f64>,
        payment_lag_days: Option<i32>,
        reset_lag_days: Option<i32>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let forward_id = forward_curve.extract::<&str>().context("forward_curve")?;
        let discount_id = discount_curve.extract::<&str>().context("discount_curve")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&end_date).context("end_date")?;
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
        let stub_kind = parse_stub(stub).context("stub")?;

        Ok(Self {
            inner: BasisSwapLeg {
                forward_curve_id: forward_id.into(),
                discount_curve_id: discount_id.into(),
                start,
                end,
                frequency: freq,
                day_count: dc,
                bdc,
                calendar_id,
                stub: stub_kind,
                spread_bp: Decimal::try_from(spread_bp.unwrap_or(0.0)).unwrap_or(Decimal::ZERO),
                payment_lag_days: payment_lag_days.unwrap_or(0),
                reset_lag_days: reset_lag_days.unwrap_or(0),
            },
        })
    }

    /// Forward curve identifier.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    /// Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Spread in basis points.
    #[getter]
    fn spread_bp(&self) -> f64 {
        self.inner.spread_bp.to_f64().unwrap_or(0.0)
    }
}

/// Basis swap wrapper with convenience constructor.
///
/// Examples:
///     >>> swap = (
///     ...     BasisSwap.builder("basis_usd")
///     ...     .money(Money("USD", 10_000_000))
///     ...     .primary_leg(primary_leg)
///     ...     .reference_leg(reference_leg)
///     ...     .build()
///     ... )
///     >>> swap.notional.amount
///     10000000
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasisSwap",
    frozen,
    from_py_object
)]
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
    primary_leg: Option<BasisSwapLeg>,
    reference_leg: Option<BasisSwapLeg>,
}

impl PyBasisSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            primary_leg: None,
            reference_leg: None,
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
        if self.primary_leg.is_none() {
            return Err(PyValueError::new_err("primary_leg() is required."));
        }
        if self.reference_leg.is_none() {
            return Err(PyValueError::new_err("reference_leg() is required."));
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

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyBasisSwap> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "BasisSwapBuilder internal error: missing notional after validation",
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

        let swap = BasisSwap::new(
            slf.instrument_id.as_str(),
            notional,
            primary_leg,
            reference_leg,
        )
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
