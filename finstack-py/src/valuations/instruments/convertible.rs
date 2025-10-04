use crate::core::error::core_to_py;
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::cashflow::builder::{PyFixedCouponSpec, PyFloatingCouponSpec};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::cashflow::builder::types::{FixedCouponSpec, FloatingCouponSpec};
use finstack_valuations::instruments::bond::{CallPut, CallPutSchedule};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::convertible::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_call_put_schedule(
    calls: Option<Vec<(Bound<'_, PyAny>, f64)>>,
    puts: Option<Vec<(Bound<'_, PyAny>, f64)>>,
) -> PyResult<Option<CallPutSchedule>> {
    if calls.is_none() && puts.is_none() {
        return Ok(None);
    }
    let mut schedule = CallPutSchedule::default();
    if let Some(list) = calls {
        for (date_obj, pct) in list {
            let date = py_to_date(&date_obj)?;
            schedule.calls.push(CallPut {
                date,
                price_pct_of_par: pct,
            });
        }
    }
    if let Some(list) = puts {
        for (date_obj, pct) in list {
            let date = py_to_date(&date_obj)?;
            schedule.puts.push(CallPut {
                date,
                price_pct_of_par: pct,
            });
        }
    }
    Ok(Some(schedule))
}

fn describe_policy(policy: &ConversionPolicy) -> String {
    match policy {
        ConversionPolicy::Voluntary => "voluntary".to_string(),
        ConversionPolicy::MandatoryOn(date) => format!("mandatory_on({date})"),
        ConversionPolicy::Window { start, end } => format!("window({start}..{end})"),
        ConversionPolicy::UponEvent(event) => format!("upon_event({event:?})"),
    }
}

/// Convertible conversion event wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConversionEvent",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyConversionEvent {
    pub(crate) inner: ConversionEvent,
}

impl PyConversionEvent {
    const fn new(inner: ConversionEvent) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyConversionEvent {
    #[classattr]
    const QUALIFIED_IPO: Self = Self::new(ConversionEvent::QualifiedIpo);
    #[classattr]
    const CHANGE_OF_CONTROL: Self = Self::new(ConversionEvent::ChangeOfControl);

    #[classmethod]
    #[pyo3(text_signature = "(cls, threshold, lookback_days)")]
    fn price_trigger(_cls: &Bound<'_, PyType>, threshold: f64, lookback_days: u32) -> Self {
        Self::new(ConversionEvent::PriceTrigger {
            threshold,
            lookback_days,
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            ConversionEvent::QualifiedIpo => "ConversionEvent.QUALIFIED_IPO".to_string(),
            ConversionEvent::ChangeOfControl => "ConversionEvent.CHANGE_OF_CONTROL".to_string(),
            ConversionEvent::PriceTrigger {
                threshold,
                lookback_days,
            } => format!(
                "ConversionEvent.price_trigger(threshold={}, lookback_days={})",
                threshold, lookback_days
            ),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Convertible conversion policy wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConversionPolicy",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyConversionPolicy {
    pub(crate) inner: ConversionPolicy,
}

impl PyConversionPolicy {
    fn new(inner: ConversionPolicy) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyConversionPolicy {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn voluntary(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(ConversionPolicy::Voluntary)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, conversion_date)")]
    fn mandatory_on(_cls: &Bound<'_, PyType>, conversion_date: Bound<'_, PyAny>) -> PyResult<Self> {
        let date = py_to_date(&conversion_date)?;
        Ok(Self::new(ConversionPolicy::MandatoryOn(date)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, start, end)")]
    fn window(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        if end_date < start_date {
            return Err(PyValueError::new_err(
                "Conversion window end must be on/after start",
            ));
        }
        Ok(Self::new(ConversionPolicy::Window {
            start: start_date,
            end: end_date,
        }))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, event)")]
    fn upon_event(_cls: &Bound<'_, PyType>, event: PyConversionEvent) -> Self {
        Self::new(ConversionPolicy::UponEvent(event.inner))
    }

    fn __repr__(&self) -> String {
        describe_policy(&self.inner)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Anti-dilution policy wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AntiDilutionPolicy",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyAntiDilutionPolicy {
    pub(crate) inner: AntiDilutionPolicy,
}

impl PyAntiDilutionPolicy {
    const fn new(inner: AntiDilutionPolicy) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAntiDilutionPolicy {
    #[classattr]
    const NONE: Self = Self::new(AntiDilutionPolicy::None);
    #[classattr]
    const FULL_RATCHET: Self = Self::new(AntiDilutionPolicy::FullRatchet);
    #[classattr]
    const WEIGHTED_AVERAGE: Self = Self::new(AntiDilutionPolicy::WeightedAverage);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            AntiDilutionPolicy::None => "AntiDilutionPolicy.NONE",
            AntiDilutionPolicy::FullRatchet => "AntiDilutionPolicy.FULL_RATCHET",
            AntiDilutionPolicy::WeightedAverage => "AntiDilutionPolicy.WEIGHTED_AVERAGE",
        }
    }

    fn __str__(&self) -> &'static str {
        self.__repr__()
    }
}

/// Dividend adjustment policy wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DividendAdjustment",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyDividendAdjustment {
    pub(crate) inner: DividendAdjustment,
}

impl PyDividendAdjustment {
    const fn new(inner: DividendAdjustment) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDividendAdjustment {
    #[classattr]
    const NONE: Self = Self::new(DividendAdjustment::None);
    #[classattr]
    const ADJUST_PRICE: Self = Self::new(DividendAdjustment::AdjustPrice);
    #[classattr]
    const ADJUST_RATIO: Self = Self::new(DividendAdjustment::AdjustRatio);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            DividendAdjustment::None => "DividendAdjustment.NONE",
            DividendAdjustment::AdjustPrice => "DividendAdjustment.ADJUST_PRICE",
            DividendAdjustment::AdjustRatio => "DividendAdjustment.ADJUST_RATIO",
        }
    }

    fn __str__(&self) -> &'static str {
        self.__repr__()
    }
}

/// Convertible conversion specification.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConversionSpec",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyConversionSpec {
    pub(crate) inner: ConversionSpec,
}

impl PyConversionSpec {
    pub(crate) fn new(inner: ConversionSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyConversionSpec {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, policy, /, *, ratio=None, price=None, anti_dilution=None, dividend_adjustment=None)",
        signature = (
            policy,
            /,
            *,
            ratio=None,
            price=None,
            anti_dilution=None,
            dividend_adjustment=None
        )
    )]
    fn create(
        _cls: &Bound<'_, PyType>,
        policy: PyConversionPolicy,
        ratio: Option<f64>,
        price: Option<f64>,
        anti_dilution: Option<PyAntiDilutionPolicy>,
        dividend_adjustment: Option<PyDividendAdjustment>,
    ) -> PyResult<Self> {
        if ratio.is_none() && price.is_none() {
            return Err(PyValueError::new_err(
                "Provide either conversion ratio or conversion price",
            ));
        }
        Ok(Self::new(ConversionSpec {
            ratio,
            price,
            policy: policy.inner,
            anti_dilution: anti_dilution
                .map(|v| v.inner)
                .unwrap_or(AntiDilutionPolicy::None),
            dividend_adjustment: dividend_adjustment
                .map(|v| v.inner)
                .unwrap_or(DividendAdjustment::None),
        }))
    }

    #[getter]
    fn ratio(&self) -> Option<f64> {
        self.inner.ratio
    }

    #[getter]
    fn price(&self) -> Option<f64> {
        self.inner.price
    }

    #[getter]
    fn policy(&self) -> String {
        describe_policy(&self.inner.policy)
    }
}

/// Convertible bond wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConvertibleBond",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyConvertibleBond {
    pub(crate) inner: ConvertibleBond,
}

impl PyConvertibleBond {
    pub(crate) fn new(inner: ConvertibleBond) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyConvertibleBond {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve, conversion, /, *, underlying_equity_id=None, call_schedule=None, put_schedule=None, fixed_coupon=None, floating_coupon=None)",
        signature = (
            instrument_id,
            notional,
            issue,
            maturity,
            discount_curve,
            conversion,
            /,
            *,
            underlying_equity_id=None,
            call_schedule=None,
            put_schedule=None,
            fixed_coupon=None,
            floating_coupon=None
        )
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        conversion: &PyConversionSpec,
        underlying_equity_id: Option<&str>,
        call_schedule: Option<Vec<(Bound<'_, PyAny>, f64)>>,
        put_schedule: Option<Vec<(Bound<'_, PyAny>, f64)>>,
        fixed_coupon: Option<&PyFixedCouponSpec>,
        floating_coupon: Option<&PyFloatingCouponSpec>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc_id = extract_curve_id(&discount_curve)?;

        if fixed_coupon.is_some() && floating_coupon.is_some() {
            return Err(PyValueError::new_err(
                "Specify either fixed_coupon or floating_coupon, not both",
            ));
        }

        let call_put = parse_call_put_schedule(call_schedule, put_schedule)?;

        let fixed_spec: Option<FixedCouponSpec> = fixed_coupon.map(|c| c.inner.clone());
        let floating_spec: Option<FloatingCouponSpec> = floating_coupon.map(|c| c.inner.clone());

        let bond = ConvertibleBond {
            id,
            notional: notional_money,
            issue: issue_date,
            maturity: maturity_date,
            disc_id,
            conversion: conversion.inner.clone(),
            underlying_equity_id: underlying_equity_id.map(|s| s.to_string()),
            call_put,
            fixed_coupon: fixed_spec,
            floating_coupon: floating_spec,
            attributes: Attributes::new(),
        };

        Ok(Self::new(bond))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Convertible)
    }

    #[getter]
    fn conversion_ratio(&self) -> Option<f64> {
        self.inner.conversion.ratio
    }

    #[getter]
    fn conversion_price(&self) -> Option<f64> {
        self.inner.conversion.price
    }

    #[getter]
    fn conversion_policy(&self) -> String {
        describe_policy(&self.inner.conversion.policy)
    }

    #[getter]
    fn issue(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.issue)
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    fn npv(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let pv = self.inner.npv(&market.inner, date).map_err(core_to_py)?;
        Ok(PyMoney::new(pv))
    }

    fn parity(&self, market: &PyMarketContext) -> PyResult<f64> {
        self.inner.parity(&market.inner).map_err(core_to_py)
    }

    fn conversion_premium(&self, market: &PyMarketContext, bond_price: f64) -> PyResult<f64> {
        self.inner
            .conversion_premium(&market.inner, bond_price)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "ConvertibleBond(id='{}', notional={}, policy='{}')",
            self.inner.id,
            self.inner.notional,
            describe_policy(&self.inner.conversion.policy)
        ))
    }
}

impl fmt::Display for PyConvertibleBond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ConvertibleBond({}, notional={}, policy={})",
            self.inner.id,
            self.inner.notional,
            describe_policy(&self.inner.conversion.policy)
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyConversionEvent>()?;
    module.add_class::<PyConversionPolicy>()?;
    module.add_class::<PyAntiDilutionPolicy>()?;
    module.add_class::<PyDividendAdjustment>()?;
    module.add_class::<PyConversionSpec>()?;
    module.add_class::<PyConvertibleBond>()?;
    Ok(vec![
        "ConversionEvent",
        "ConversionPolicy",
        "AntiDilutionPolicy",
        "DividendAdjustment",
        "ConversionSpec",
        "ConvertibleBond",
    ])
}
