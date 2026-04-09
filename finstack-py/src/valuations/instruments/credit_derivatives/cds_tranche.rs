use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::errors::PyContext;
use crate::valuations::common::{
    frequency_from_payments_per_year, to_optional_string, PyInstrumentType,
};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::credit_derivatives::cds_tranche::{CDSTranche, TrancheSide};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

use crate::core::market_data::context::PyMarketContext;

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

fn parse_tranche_side(label: Option<&str>) -> PyResult<TrancheSide> {
    match label {
        None => Ok(TrancheSide::BuyProtection),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Buy/sell protection side for CDS tranches.
///
/// Examples:
///     >>> TrancheSide.BUY_PROTECTION
///     TrancheSide('buy_protection')
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheSide",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyTrancheSide {
    pub(crate) inner: TrancheSide,
}

impl PyTrancheSide {
    pub(crate) const fn new(inner: TrancheSide) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            TrancheSide::BuyProtection => "buy_protection",
            TrancheSide::SellProtection => "sell_protection",
        }
    }
}

#[pymethods]
impl PyTrancheSide {
    #[classattr]
    const BUY_PROTECTION: Self = Self::new(TrancheSide::BuyProtection);
    #[classattr]
    const SELL_PROTECTION: Self = Self::new(TrancheSide::SellProtection);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<TrancheSide>()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("TrancheSide('{}')", self.label())
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

impl fmt::Display for PyTrancheSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// CDS tranche wrapper exposing a simplified constructor.
///
/// Examples:
///     >>> tranche = (
///     ...     CDSTranche.builder("itraxx_tranche")
///     ...     .index_name("iTraxx Europe")
///     ...     .series(38)
///     ...     .attach_pct(3.0)
///     ...     .detach_pct(7.0)
///     ...     .notional(Money("EUR", 10_000_000))
///     ...     .maturity(date(2029, 3, 20))
///     ...     .running_coupon_bp(500.0)
///     ...     .discount_curve("eur_discount")
///     ...     .credit_index_curve("itraxx_credit")
///     ...     .build()
///     ... )
///     >>> tranche.attach_pct
///     3.0
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CDSTranche",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCDSTranche {
    pub(crate) inner: Arc<CDSTranche>,
}

impl PyCDSTranche {
    pub(crate) fn new(inner: CDSTranche) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(module = "finstack.valuations.instruments", name = "CDSTrancheBuilder")]
pub struct PyCDSTrancheBuilder {
    instrument_id: InstrumentId,
    index_name: Option<String>,
    series: Option<u16>,
    attach_pct: Option<f64>,
    detach_pct: Option<f64>,
    notional: Option<finstack_core::money::Money>,
    maturity: Option<time::Date>,
    running_coupon_bp: Option<f64>,
    discount_curve_id: Option<CurveId>,
    credit_index_id: Option<CurveId>,
    side: TrancheSide,
    payments_per_year: u32,
    day_count: DayCount,
    business_day_convention: BusinessDayConvention,
    calendar: Option<String>,
    effective_date: Option<time::Date>,
}

impl PyCDSTrancheBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            index_name: None,
            series: None,
            attach_pct: None,
            detach_pct: None,
            notional: None,
            maturity: None,
            running_coupon_bp: None,
            discount_curve_id: None,
            credit_index_id: None,
            side: TrancheSide::BuyProtection,
            payments_per_year: 4,
            day_count: DayCount::Act360,
            business_day_convention: BusinessDayConvention::Following,
            calendar: None,
            effective_date: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.index_name.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("index_name() is required."));
        }
        if self.series.is_none() {
            return Err(PyValueError::new_err("series() is required."));
        }
        if self.attach_pct.is_none() {
            return Err(PyValueError::new_err("attach_pct() is required."));
        }
        if self.detach_pct.is_none() {
            return Err(PyValueError::new_err("detach_pct() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.running_coupon_bp.is_none() {
            return Err(PyValueError::new_err("running_coupon_bp() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.credit_index_id.is_none() {
            return Err(PyValueError::new_err("credit_index_curve() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCDSTrancheBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, index_name)")]
    fn index_name(mut slf: PyRefMut<'_, Self>, index_name: String) -> PyRefMut<'_, Self> {
        slf.index_name = Some(index_name);
        slf
    }

    #[pyo3(text_signature = "($self, series)")]
    fn series(mut slf: PyRefMut<'_, Self>, series: u16) -> PyRefMut<'_, Self> {
        slf.series = Some(series);
        slf
    }

    #[pyo3(text_signature = "($self, attach_pct)")]
    fn attach_pct(mut slf: PyRefMut<'_, Self>, attach_pct: f64) -> PyRefMut<'_, Self> {
        slf.attach_pct = Some(attach_pct);
        slf
    }

    #[pyo3(text_signature = "($self, detach_pct)")]
    fn detach_pct(mut slf: PyRefMut<'_, Self>, detach_pct: f64) -> PyRefMut<'_, Self> {
        slf.detach_pct = Some(detach_pct);
        slf
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional).context("notional")?);
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

    #[pyo3(text_signature = "($self, running_coupon_bp)")]
    fn running_coupon_bp(
        mut slf: PyRefMut<'_, Self>,
        running_coupon_bp: f64,
    ) -> PyRefMut<'_, Self> {
        slf.running_coupon_bp = Some(running_coupon_bp);
        slf
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(discount_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, credit_index_curve)")]
    fn credit_index_curve(
        mut slf: PyRefMut<'_, Self>,
        credit_index_curve: String,
    ) -> PyRefMut<'_, Self> {
        slf.credit_index_id = Some(CurveId::new(credit_index_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn side(mut slf: PyRefMut<'_, Self>, side: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.side = parse_tranche_side(Some(side.as_str())).context("side")?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, payments_per_year)")]
    fn payments_per_year(
        mut slf: PyRefMut<'_, Self>,
        payments_per_year: u32,
    ) -> PyRefMut<'_, Self> {
        slf.payments_per_year = payments_per_year;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(value) = day_count.extract()?;
        slf.day_count = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, business_day_convention)")]
    fn business_day_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        business_day_convention: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let BusinessDayConventionArg(value) = business_day_convention.extract()?;
        slf.business_day_convention = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, calendar=None)", signature = (calendar=None))]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar = calendar;
        slf
    }

    #[pyo3(text_signature = "($self, effective_date=None)", signature = (effective_date=None))]
    fn effective_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        effective_date: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.effective_date = if let Some(date_obj) = effective_date {
            Some(py_to_date(&date_obj)?)
        } else {
            None
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCDSTranche> {
        slf.ensure_ready()?;

        let missing =
            |field: &str| PyValueError::new_err(format!("{field} is required but was not set"));

        let mut builder = CDSTranche::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.index_name(
            slf.index_name
                .clone()
                .ok_or_else(|| missing("index_name"))?,
        );
        builder = builder.series(slf.series.ok_or_else(|| missing("series"))?);
        builder = builder.attach_pct(slf.attach_pct.ok_or_else(|| missing("attach_pct"))?);
        builder = builder.detach_pct(slf.detach_pct.ok_or_else(|| missing("detach_pct"))?);
        builder = builder.notional(slf.notional.ok_or_else(|| missing("notional"))?);
        builder = builder.maturity(slf.maturity.ok_or_else(|| missing("maturity"))?);
        builder = builder.running_coupon_bp(
            slf.running_coupon_bp
                .ok_or_else(|| missing("running_coupon_bp"))?,
        );
        builder = builder.frequency(
            frequency_from_payments_per_year(Some(slf.payments_per_year))
                .map_err(|e| PyValueError::new_err(format!("Invalid payments_per_year: {e}")))?,
        );
        builder = builder.day_count(slf.day_count);
        builder = builder.bdc(slf.business_day_convention);
        builder = builder.calendar_id_opt(to_optional_string(slf.calendar.as_deref()));
        builder = builder.discount_curve_id(
            slf.discount_curve_id
                .clone()
                .ok_or_else(|| missing("discount_curve"))?,
        );
        builder = builder.credit_index_id(
            slf.credit_index_id
                .clone()
                .ok_or_else(|| missing("credit_index_curve"))?,
        );
        builder = builder.side(slf.side);
        builder = builder.effective_date_opt(slf.effective_date);
        builder = builder.attributes(Default::default());
        builder = builder.standard_imm_dates(true);
        builder = builder.accumulated_loss(0.0);

        let tranche = builder.build().map_err(core_to_py)?;
        tranche.validate().map_err(core_to_py)?;
        Ok(PyCDSTranche::new(tranche))
    }

    fn __repr__(&self) -> String {
        "CDSTrancheBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCDSTranche {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCDSTrancheBuilder>> {
        let py = cls.py();
        let builder = PyCDSTrancheBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the tranche.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Tranche notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Attachment point percentage.
    ///
    /// Returns:
    ///     float: Attachment level in percent.
    #[getter]
    fn attach_pct(&self) -> f64 {
        self.inner.attach_pct
    }

    /// Detachment point percentage.
    ///
    /// Returns:
    ///     float: Detachment level in percent.
    #[getter]
    fn detach_pct(&self) -> f64 {
        self.inner.detach_pct
    }

    /// Running coupon in basis points.
    ///
    /// Returns:
    ///     float: Running spread paid on outstanding tranche balance.
    #[getter]
    fn running_coupon_bp(&self) -> f64 {
        self.inner.running_coupon_bp
    }

    /// Maturity date of the tranche.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Credit index curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve used for the index portfolio.
    #[getter]
    fn credit_index_curve(&self) -> String {
        self.inner.credit_index_id.as_str().to_string()
    }

    /// Index name.
    #[getter]
    fn index_name(&self) -> &str {
        &self.inner.index_name
    }

    /// Index series number.
    #[getter]
    fn series(&self) -> u16 {
        self.inner.series
    }

    /// Protection side.
    #[getter]
    fn side(&self) -> PyTrancheSide {
        PyTrancheSide::new(self.inner.side)
    }

    /// Payment frequency.
    #[getter]
    fn frequency(&self) -> String {
        format!("{}", self.inner.frequency)
    }

    /// Day count convention.
    #[getter]
    fn day_count(&self) -> &'static str {
        day_count_label(self.inner.day_count)
    }

    /// Business day convention.
    #[getter]
    fn bdc(&self) -> String {
        format!("{}", self.inner.bdc)
    }

    /// Holiday calendar identifier.
    #[getter]
    fn calendar(&self) -> Option<String> {
        self.inner.calendar_id.clone()
    }

    /// Effective date for schedule anchoring.
    #[getter]
    fn effective_date(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.effective_date {
            Some(d) => Ok(Some(date_to_py(py, d)?)),
            None => Ok(None),
        }
    }

    /// Accumulated realized loss as fraction of original portfolio notional.
    #[getter]
    fn accumulated_loss(&self) -> f64 {
        self.inner.accumulated_loss
    }

    /// Whether standard IMM dates are enforced.
    #[getter]
    fn standard_imm_dates(&self) -> bool {
        self.inner.standard_imm_dates
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_TRANCHE``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSTranche)
    }

    /// Calculate upfront amount.
    #[pyo3(signature = (market, as_of))]
    fn upfront(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.upfront(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate spread DV01 (sensitivity to 1bp running coupon change).
    #[pyo3(signature = (market, as_of))]
    fn spread_dv01(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.spread_dv01(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate par spread (running coupon in basis points).
    #[pyo3(signature = (market, as_of))]
    fn par_spread(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.par_spread(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate expected loss.
    #[pyo3(signature = (market,))]
    fn expected_loss(&self, py: Python<'_>, market: &PyMarketContext) -> PyResult<f64> {
        py.detach(|| self.inner.expected_loss(&market.inner))
            .map_err(core_to_py)
    }

    /// Calculate jump-to-default metric.
    #[pyo3(signature = (market, as_of))]
    fn jump_to_default(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.jump_to_default(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate correlation delta (sensitivity to correlation changes).
    #[pyo3(signature = (market, as_of))]
    fn correlation_delta(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.correlation_delta(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate accrued premium.
    #[pyo3(signature = (market, as_of))]
    fn accrued_premium(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.accrued_premium(&market.inner, date))
            .map_err(core_to_py)
    }

    /// Calculate expected loss curve over time.
    ///
    /// Returns:
    ///     list[tuple[datetime.date, float]]: (date, cumulative_expected_loss) pairs.
    #[pyo3(signature = (market, as_of))]
    fn expected_loss_curve(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<Vec<(Py<PyAny>, f64)>> {
        let date = py_to_date(&as_of)?;
        let curve = py
            .detach(|| self.inner.expected_loss_curve(&market.inner, date))
            .map_err(core_to_py)?;
        curve
            .into_iter()
            .map(|(d, v)| Ok((date_to_py(py, d)?, v)))
            .collect()
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CDSTranche(id='{}', attach={:.2}%, detach={:.2}%)",
            self.inner.id, self.inner.attach_pct, self.inner.detach_pct
        ))
    }
}

impl fmt::Display for PyCDSTranche {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CDSTranche({}, attach={:.2}%, detach={:.2}%)",
            self.inner.index_name, self.inner.attach_pct, self.inner.detach_pct
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTrancheSide>()?;
    module.add_class::<PyCDSTranche>()?;
    module.add_class::<PyCDSTrancheBuilder>()?;
    Ok(vec!["TrancheSide", "CDSTranche", "CDSTrancheBuilder"])
}
