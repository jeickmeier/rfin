use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwap, IsdaCdsParams, PayReceive, PremiumLegSpec, ProtectionLegSpec,
};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

fn day_count_label(dc: finstack_core::dates::DayCount) -> &'static str {
    use finstack_core::dates::DayCount;
    match dc {
        DayCount::Act360 => "act_360",
        DayCount::Act365F => "act_365f",
        DayCount::Act365L => "act_365l",
        DayCount::Thirty360 => "thirty_360",
        DayCount::ThirtyE360 => "thirty_e_360",
        DayCount::ActAct => "act_act",
        DayCount::ActActIsma => "act_act_isma",
        DayCount::Bus252 => "bus_252",
        _ => "custom",
    }
}

/// Pay/receive indicator for CDS premium leg.
///
/// Examples:
///     >>> CDSPayReceive.from_name("buy")
///     CDSPayReceive('pay_protection')
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CDSPayReceive",
    frozen,
    from_py_object
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
    ) -> PyResult<Py<PyAny>> {
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

/// ISDA CDS convention for regional market standards.
///
/// Examples:
///     >>> CDSConvention.ISDA_NA.day_count
///     'act_360'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CDSConvention",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyCdsConvention {
    pub(crate) inner: CDSConvention,
}

impl PyCdsConvention {
    pub(crate) const fn new(inner: CDSConvention) -> Self {
        Self { inner }
    }

    fn label(&self) -> String {
        self.inner.to_string()
    }
}

#[pymethods]
impl PyCdsConvention {
    #[classattr]
    const ISDA_NA: Self = Self::new(CDSConvention::IsdaNa);
    #[classattr]
    const ISDA_EU: Self = Self::new(CDSConvention::IsdaEu);
    #[classattr]
    const ISDA_AS: Self = Self::new(CDSConvention::IsdaAs);
    #[classattr]
    const CUSTOM: Self = Self::new(CDSConvention::Custom);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<CDSConvention>()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, currency)")]
    fn detect_from_currency(
        _cls: &Bound<'_, PyType>,
        currency: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use finstack_core::currency::Currency;
        let ccy = if let Ok(py_ccy) = currency.extract::<crate::core::currency::PyCurrency>() {
            py_ccy.inner
        } else if let Ok(code) = currency.extract::<&str>() {
            Currency::from_str(code)
                .map_err(|_| PyValueError::new_err(format!("Unknown currency code: '{}'", code)))?
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Expected Currency or string",
            ));
        };
        Ok(Self::new(CDSConvention::detect_from_currency(ccy)))
    }

    #[getter]
    fn name(&self) -> String {
        self.label()
    }

    #[getter]
    fn day_count(&self) -> &'static str {
        day_count_label(self.inner.day_count())
    }

    #[getter]
    fn frequency(&self) -> String {
        format!("{}", self.inner.frequency())
    }

    #[getter]
    fn business_day_convention(&self) -> String {
        format!("{}", self.inner.business_day_convention())
    }

    #[getter]
    fn settlement_delay(&self) -> u16 {
        self.inner.settlement_delay()
    }

    #[getter]
    fn default_calendar(&self) -> &'static str {
        self.inner.default_calendar()
    }

    fn __repr__(&self) -> String {
        format!("CDSConvention('{}')", self.label())
    }

    fn __str__(&self) -> String {
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
    ) -> PyResult<Py<PyAny>> {
        let rhs = other
            .extract::<PyRef<Self>>()
            .ok()
            .map(|ref_obj| ref_obj.inner);
        crate::core::common::pycmp::richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

impl fmt::Display for PyCdsConvention {
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
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCreditDefaultSwap {
    pub(crate) inner: Arc<CreditDefaultSwap>,
}

impl PyCreditDefaultSwap {
    pub(crate) fn new(inner: CreditDefaultSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
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
            settlement_delay=None,
            convention=None
        ),
        text_signature = "(cls, instrument_id, notional, spread_bp, start_date, maturity, discount_curve, credit_curve, /, *, recovery_rate=None, settlement_delay=None, convention=None)"
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
    ///     convention: Optional ISDA convention (default: ISDA_NA).
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
        convention: Option<PyCdsConvention>,
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
            convention,
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
            settlement_delay=None,
            convention=None
        ),
        text_signature = "(cls, instrument_id, notional, spread_bp, start_date, maturity, discount_curve, credit_curve, /, *, recovery_rate=None, settlement_delay=None, convention=None)"
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
    ///     convention: Optional ISDA convention (default: ISDA_NA).
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
        convention: Option<PyCdsConvention>,
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
            convention,
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
    /// Returns
    /// -------
    /// float
    ///     Premium spread for the CDS in basis points.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the internal decimal value cannot be represented as float.
    #[getter]
    fn spread_bp(&self) -> PyResult<f64> {
        self.inner
            .premium
            .spread_bp
            .to_f64()
            .ok_or_else(|| PyValueError::new_err("spread_bp: decimal to f64 conversion failed"))
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
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.premium.start)
    }

    /// Protection maturity date.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
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

    /// ISDA convention used for this CDS.
    #[getter]
    fn convention(&self) -> PyCdsConvention {
        PyCdsConvention::new(self.inner.convention)
    }

    /// Day count convention for premium leg.
    #[getter]
    fn day_count(&self) -> &'static str {
        day_count_label(self.inner.premium.day_count)
    }

    /// Payment frequency for premium leg.
    #[getter]
    fn frequency(&self) -> String {
        format!("{}", self.inner.premium.frequency)
    }

    /// Holiday calendar identifier.
    #[getter]
    fn calendar(&self) -> Option<String> {
        self.inner.premium.calendar_id.clone()
    }

    /// ISDA-standard coupon date schedule.
    ///
    /// Returns:
    ///     list[datetime.date]: Coupon dates including start and maturity.
    ///
    /// Raises:
    ///     ValueError: If schedule generation fails.
    fn isda_coupon_schedule(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let dates = self
            .inner
            .isda_coupon_schedule()
            .map_err(crate::errors::core_to_py)?;
        dates.into_iter().map(|d| date_to_py(py, d)).collect()
    }

    /// Create a CDS with standard ISDA conventions.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique identifier.
    /// notional : Money
    ///     Notional principal amount.
    /// side : CDSPayReceive or str
    ///     Pay or receive protection.
    /// convention : CDSConvention
    ///     ISDA regional convention.
    /// spread_bp : float
    ///     Running spread in basis points.
    /// start : datetime.date
    ///     Premium leg start date.
    /// end : datetime.date
    ///     Maturity (protection end date).
    /// recovery_rate : float
    ///     Recovery rate in decimal form (e.g. 0.40).
    /// discount_curve_id : str
    ///     Discount curve identifier.
    /// credit_curve_id : str
    ///     Credit/hazard curve identifier.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwap
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If construction or validation fails.
    #[classmethod]
    #[pyo3(
        signature = (instrument_id, notional, side, convention, spread_bp, start, end, recovery_rate, discount_curve_id, credit_curve_id),
        text_signature = "(cls, instrument_id, notional, side, convention, spread_bp, start, end, recovery_rate, discount_curve_id, credit_curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn from_isda(
        _cls: &Bound<'_, PyType>,
        instrument_id: &str,
        notional: Bound<'_, PyAny>,
        side: Bound<'_, PyAny>,
        convention: PyCdsConvention,
        spread_bp: f64,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        recovery_rate: f64,
        discount_curve_id: &str,
        credit_curve_id: &str,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let amt = extract_money(&notional).context("notional")?;
        let parsed_side = if let Ok(py_side) = side.extract::<PyRef<PyCdsPayReceive>>() {
            py_side.inner
        } else if let Ok(name) = side.extract::<&str>() {
            normalize_cds_side(name)?
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "side expects CDSPayReceive or str",
            ));
        };
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;

        let params = IsdaCdsParams {
            id: InstrumentId::new(instrument_id),
            notional: amt,
            side: parsed_side,
            convention: convention.inner,
            spread_bp,
            start: start_date,
            end: end_date,
            recovery_rate,
            discount_curve_id,
            credit_curve_id,
        };

        CreditDefaultSwap::from_isda(params)
            .map(PyCreditDefaultSwap::new)
            .map_err(core_to_py)
    }

    /// Effective protection start date.
    ///
    /// For a forward-starting CDS, returns the explicit protection effective date.
    /// For a standard CDS, returns the premium start date.
    ///
    /// Returns:
    ///     datetime.date: Protection start date.
    #[getter]
    fn protection_start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.protection_start())
    }

    /// Effective ISDA documentation clause.
    ///
    /// If an explicit doc_clause is set, returns it. Otherwise derives
    /// the standard clause from the CDS convention.
    ///
    /// Returns:
    ///     CdsDocClause: Resolved documentation clause.
    #[getter]
    fn doc_clause_effective(&self) -> crate::valuations::market::conventions::PyCdsDocClause {
        crate::valuations::market::conventions::PyCdsDocClause::new(
            self.inner.doc_clause_effective(),
        )
    }

    /// Build the premium leg cashflow schedule.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data context (curves).
    /// as_of : datetime.date
    ///     Valuation date.
    ///
    /// Returns
    /// -------
    /// list[tuple[datetime.date, Money]]
    ///     Premium leg cashflows as (date, amount) pairs.
    #[pyo3(
        signature = (market, as_of),
        text_signature = "($self, market, as_of)"
    )]
    fn build_premium_schedule(
        &self,
        py: Python<'_>,
        market: &crate::core::market_data::PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
        let as_of_date = py_to_date(&as_of)?;
        let flows = self
            .inner
            .build_premium_schedule(&market.inner, as_of_date)
            .map_err(core_to_py)?;
        flows
            .into_iter()
            .map(|(date, money)| Ok((date_to_py(py, date)?, PyMoney::new(money))))
            .collect()
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

#[allow(clippy::too_many_arguments)]
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
    convention: Option<PyCdsConvention>,
) -> PyResult<PyCreditDefaultSwap> {
    use crate::errors::PyContext;
    let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
    let amt = extract_money(&notional).context("notional")?;
    let start = py_to_date(&start_date).context("start_date")?;
    let end = py_to_date(&maturity).context("maturity")?;
    let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
    let credit = credit_curve.extract::<&str>().context("credit_curve")?;

    let convention = convention.map(|c| c.inner).unwrap_or(CDSConvention::IsdaNa);
    let dc = convention.day_count();
    let freq = convention.frequency();
    let bdc = convention.business_day_convention();
    let stub = convention.stub_convention();

    let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
        PyValueError::new_err(format!("spread_bp cannot be represented as Decimal: {e}"))
    })?;

    let mut cds = CreditDefaultSwap::builder()
        .id(id.clone())
        .notional(amt)
        .side(side)
        .convention(convention)
        .premium(PremiumLegSpec {
            start,
            end,
            frequency: freq,
            stub,
            bdc,
            calendar_id: Some(convention.default_calendar().to_string()),
            day_count: dc,
            spread_bp: spread_bp_decimal,
            discount_curve_id: disc,
        })
        .protection(ProtectionLegSpec {
            credit_curve_id: CurveId::new(credit),
            recovery_rate: finstack_valuations::instruments::credit_derivatives::cds::RECOVERY_SENIOR_UNSECURED,
            settlement_delay: convention.settlement_delay(),
        })
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .map_err(core_to_py)?;

    if let Some(rr) = recovery_rate {
        cds = cds.with_recovery_rate(rr);
    }
    if let Some(delay) = settlement_delay {
        cds = cds.with_settlement_delay(delay);
    }

    cds.validate().map_err(core_to_py)?;

    Ok(PyCreditDefaultSwap::new(cds))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsPayReceive>()?;
    module.add_class::<PyCdsConvention>()?;
    module.add_class::<PyCreditDefaultSwap>()?;
    Ok(vec!["CDSPayReceive", "CDSConvention", "CreditDefaultSwap"])
}
