use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::cashflow::builder::{PyFixedCouponSpec, PyFloatingCouponSpec};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::{FixedCouponSpec, FloatingCouponSpec};
use finstack_valuations::instruments::fixed_income::bond::{CallPut, CallPutSchedule};
use finstack_valuations::instruments::fixed_income::convertible::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::prelude::InstrumentNpvExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_call_put_list(
    items: Vec<(Bound<'_, PyAny>, f64)>,
    context_label: &str,
) -> PyResult<Vec<CallPut>> {
    let mut out = Vec::with_capacity(items.len());
    for (date_obj, pct) in items {
        use crate::errors::PyContext;
        let date = py_to_date(&date_obj).context(context_label)?;
        out.push(CallPut {
            date,
            price_pct_of_par: pct,
        });
    }
    Ok(out)
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
        use crate::errors::PyContext;
        let date = py_to_date(&conversion_date).context("conversion_date")?;
        Ok(Self::new(ConversionPolicy::MandatoryOn(date)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, start, end)")]
    fn window(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
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
    pub(crate) fn from_inner(inner: ConversionSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyConversionSpec {
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
    #[new]
    fn new(
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
        Ok(Self::from_inner(ConversionSpec {
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
    pub(crate) inner: Arc<ConvertibleBond>,
}

impl PyConvertibleBond {
    pub(crate) fn new(inner: ConvertibleBond) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConvertibleBondBuilder",
    unsendable
)]
pub struct PyConvertibleBondBuilder {
    instrument_id: InstrumentId,
    notional: Option<finstack_core::money::Money>,
    issue: Option<time::Date>,
    maturity: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    conversion: Option<ConversionSpec>,
    underlying_equity_id: Option<String>,
    calls: Vec<CallPut>,
    puts: Vec<CallPut>,
    fixed_coupon: Option<FixedCouponSpec>,
    floating_coupon: Option<FloatingCouponSpec>,
}

impl PyConvertibleBondBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            notional: None,
            issue: None,
            maturity: None,
            discount_curve_id: None,
            conversion: None,
            underlying_equity_id: None,
            calls: Vec::new(),
            puts: Vec::new(),
            fixed_coupon: None,
            floating_coupon: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.issue.is_none() {
            return Err(PyValueError::new_err("issue() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.conversion.is_none() {
            return Err(PyValueError::new_err("conversion() is required."));
        }
        if self.fixed_coupon.is_some() && self.floating_coupon.is_some() {
            return Err(PyValueError::new_err(
                "Specify either fixed_coupon or floating_coupon, not both",
            ));
        }
        Ok(())
    }
}

#[pymethods]
impl PyConvertibleBondBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, issue)")]
    fn issue<'py>(
        mut slf: PyRefMut<'py, Self>,
        issue: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.issue = Some(py_to_date(&issue).context("issue")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(discount_curve.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, conversion)")]
    fn conversion<'py>(
        mut slf: PyRefMut<'py, Self>,
        conversion: &PyConversionSpec,
    ) -> PyRefMut<'py, Self> {
        slf.conversion = Some(conversion.inner.clone());
        slf
    }

    #[pyo3(
        text_signature = "($self, underlying_equity_id=None)",
        signature = (underlying_equity_id=None)
    )]
    fn underlying_equity_id(
        mut slf: PyRefMut<'_, Self>,
        underlying_equity_id: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.underlying_equity_id = underlying_equity_id;
        slf
    }

    #[pyo3(text_signature = "($self, call_schedule=None)", signature = (call_schedule=None))]
    fn call_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        call_schedule: Option<Vec<(Bound<'py, PyAny>, f64)>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.calls = if let Some(items) = call_schedule {
            parse_call_put_list(items, "call_schedule date")?
        } else {
            Vec::new()
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, put_schedule=None)", signature = (put_schedule=None))]
    fn put_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        put_schedule: Option<Vec<(Bound<'py, PyAny>, f64)>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.puts = if let Some(items) = put_schedule {
            parse_call_put_list(items, "put_schedule date")?
        } else {
            Vec::new()
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, fixed_coupon)")]
    fn fixed_coupon<'py>(
        mut slf: PyRefMut<'py, Self>,
        fixed_coupon: &PyFixedCouponSpec,
    ) -> PyRefMut<'py, Self> {
        slf.fixed_coupon = Some(fixed_coupon.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, floating_coupon)")]
    fn floating_coupon<'py>(
        mut slf: PyRefMut<'py, Self>,
        floating_coupon: &PyFloatingCouponSpec,
    ) -> PyRefMut<'py, Self> {
        slf.floating_coupon = Some(floating_coupon.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyConvertibleBond> {
        slf.ensure_ready()?;

        let call_put = if slf.calls.is_empty() && slf.puts.is_empty() {
            None
        } else {
            Some(CallPutSchedule {
                calls: slf.calls.clone(),
                puts: slf.puts.clone(),
                ..Default::default()
            })
        };

        let bond = ConvertibleBond {
            id: slf.instrument_id.clone(),
            notional: slf.notional.unwrap(),
            issue: slf.issue.unwrap(),
            maturity: slf.maturity.unwrap(),
            discount_curve_id: slf.discount_curve_id.clone().unwrap(),
            credit_curve_id: None,
            conversion: slf.conversion.clone().unwrap(),
            underlying_equity_id: slf.underlying_equity_id.clone(),
            call_put,
            fixed_coupon: slf.fixed_coupon.clone(),
            floating_coupon: slf.floating_coupon.clone(),
            attributes: Attributes::new(),
        };

        Ok(PyConvertibleBond::new(bond))
    }

    fn __repr__(&self) -> String {
        "ConvertibleBondBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyConvertibleBond {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyConvertibleBondBuilder>> {
        let py = cls.py();
        let builder = PyConvertibleBondBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
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
        self.inner.discount_curve_id.as_str().to_string()
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
    fn issue(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.issue)
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
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
    module.add_class::<PyConvertibleBondBuilder>()?;
    Ok(vec![
        "ConversionEvent",
        "ConversionPolicy",
        "AntiDilutionPolicy",
        "DividendAdjustment",
        "ConversionSpec",
        "ConvertibleBond",
        "ConvertibleBondBuilder",
    ])
}
