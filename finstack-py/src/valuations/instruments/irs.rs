use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::intern_calendar_id_opt;
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::irs::{
    FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive,
};
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
    ///
    /// Note: Uses USD market conventions (fixed: semi-annual 30/360; float: quarterly Act/360).
    ///       For explicit curve and convention control, use ``builder()``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed leg rate in decimal form.
    ///     start: Effective start date.
    ///     end: Maturity date.
    ///
    /// Returns:
    ///     InterestRateSwap: Configured swap with USD-OIS discount and USD-SOFR-3M forward.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
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
        use finstack_valuations::instruments::common::parameters::PayReceive;
        Ok(Self::new(InterestRateSwap::new(
            id,
            amt,
            fixed_rate,
            start_date,
            end_date,
            PayReceive::PayFixed,
        )))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, fixed_rate, start, end)")]
    /// Create a USD SOFR swap where the caller receives fixed.
    ///
    /// Note: Uses USD market conventions (fixed: semi-annual 30/360; float: quarterly Act/360).
    ///       For explicit curve and convention control, use ``builder()``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed leg rate in decimal form.
    ///     start: Effective start date.
    ///     end: Maturity date.
    ///
    /// Returns:
    ///     InterestRateSwap: Configured swap with USD-OIS discount and USD-SOFR-3M forward.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
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
        use finstack_valuations::instruments::common::parameters::PayReceive;
        Ok(Self::new(InterestRateSwap::new(
            id,
            amt,
            fixed_rate,
            start_date,
            end_date,
            PayReceive::ReceiveFixed,
        )))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, start, end, primary_spread_bp, reference_spread_bp)"
    )]
    /// Create a USD basis swap with spreads applied to both floating legs.
    ///
    /// Note: Uses USD market conventions. For explicit curve control, use ``builder()``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     start: Effective start date.
    ///     end: Maturity date.
    ///     primary_spread_bp: Spread in basis points for the primary floating leg.
    ///     reference_spread_bp: Spread in basis points for the reference floating leg.
    ///
    /// Returns:
    ///     InterestRateSwap: Configured basis swap.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
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

    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            fixed_rate,
            start,
            end,
            side,
            discount_curve,
            forward_curve,
            *,
            fixed_frequency=None,
            float_frequency=None,
            fixed_day_count=None,
            float_day_count=None,
            business_day_convention=None,
            float_spread_bp=0.0,
            reset_lag_days=2,
            calendar=None,
            stub=None
        ),
        text_signature = "(cls, instrument_id, notional, fixed_rate, start, end, side, discount_curve, forward_curve, /, *, fixed_frequency='semi_annual', float_frequency='quarterly', fixed_day_count='thirty_360', float_day_count='act_360', business_day_convention='modified_following', float_spread_bp=0.0, reset_lag_days=2, calendar=None, stub='none')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a fully customizable interest rate swap with explicit curves and conventions.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed leg rate in decimal form.
    ///     start: Effective start date.
    ///     end: Maturity date.
    ///     side: Pay/receive direction (``"pay_fixed"`` or ``"receive_fixed"``).
    ///     discount_curve: Discount curve identifier for both legs.
    ///     forward_curve: Forward curve identifier for floating leg.
    ///     fixed_frequency: Optional fixed leg frequency.
    ///     float_frequency: Optional floating leg frequency.
    ///     fixed_day_count: Optional fixed leg day-count convention.
    ///     float_day_count: Optional floating leg day-count convention.
    ///     business_day_convention: Optional business-day adjustment rule.
    ///     float_spread_bp: Optional floating leg spread in basis points.
    ///     reset_lag_days: Optional reset lag in business days.
    ///     calendar: Optional calendar identifier for date adjustments.
    ///     stub: Optional stub period rule.
    ///
    /// Returns:
    ///     InterestRateSwap: Fully configured swap with all inputs user-controlled.
    ///
    /// Raises:
    ///     ValueError: If identifiers, dates, or labels cannot be parsed.
    ///     RuntimeError: When the underlying builder detects invalid input.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        side: &str,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        fixed_frequency: Option<crate::core::dates::schedule::PyFrequency>,
        float_frequency: Option<crate::core::dates::schedule::PyFrequency>,
        fixed_day_count: Option<Bound<'_, PyAny>>,
        float_day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        float_spread_bp: Option<f64>,
        reset_lag_days: Option<i32>,
        calendar: Option<&str>,
        stub: Option<crate::core::dates::schedule::PyStubKind>,
    ) -> PyResult<Self> {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};

        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let fwd_id = extract_curve_id(&forward_curve)?;

        let side_value = side
            .parse::<PayReceive>()
            .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e))?;

        let fixed_freq = fixed_frequency
            .map(|f| f.inner)
            .unwrap_or_else(Frequency::semi_annual);
        let float_freq = float_frequency
            .map(|f| f.inner)
            .unwrap_or_else(Frequency::quarterly);

        let fixed_dc = if let Some(obj) = fixed_day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Thirty360
        };
        let float_dc = if let Some(obj) = float_day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Act360
        };

        let bdc = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) = obj.extract()?;
            value
        } else {
            BusinessDayConvention::ModifiedFollowing
        };

        let stub_kind = stub.map(|s| s.inner).unwrap_or(StubKind::None);
        let cal_id_opt = intern_calendar_id_opt(calendar).map(|s| s.to_string());

        let fixed_leg = FixedLegSpec {
            disc_id: disc_id.clone(),
            rate: fixed_rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: cal_id_opt.clone(),
            stub: stub_kind,
            start: start_date,
            end: end_date,
            par_method: None,
            compounding_simple: true,
        };

        let float_leg = FloatLegSpec {
            disc_id: disc_id.clone(),
            fwd_id,
            spread_bp: float_spread_bp.unwrap_or(0.0),
            freq: float_freq,
            dc: float_dc,
            bdc,
            calendar_id: cal_id_opt,
            stub: stub_kind,
            reset_lag_days: reset_lag_days.unwrap_or(2),
            start: start_date,
            end: end_date,
        };

        InterestRateSwap::builder()
            .id(id)
            .notional(amt)
            .side(side_value)
            .fixed(fixed_leg)
            .float(float_leg)
            .attributes(Default::default())
            .build()
            .map(Self::new)
            .map_err(core_to_py)
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
