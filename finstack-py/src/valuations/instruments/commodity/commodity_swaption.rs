use super::common::{option_pricing_overrides, validated_clone, validated_field};
use crate::core::common::args::{CurrencyArg, DayCountArg, TenorArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{CalendarId, CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_swaption::CommoditySwaption;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

fn parse_option_type(label: &str) -> PyResult<OptionType> {
    OptionType::from_str(label).map_err(|e| PyValueError::new_err(e.to_string()))
}

fn parse_bdc(label: &str) -> PyResult<BusinessDayConvention> {
    BusinessDayConvention::from_str(label)
        .map_err(|e| PyValueError::new_err(format!("Invalid bdc: {e}")))
}

/// Option to enter a fixed-for-floating commodity swap.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySwaption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommoditySwaption {
    pub(crate) inner: Arc<CommoditySwaption>,
}

impl PyCommoditySwaption {
    pub(crate) fn new(inner: CommoditySwaption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommoditySwaptionBuilder"
)]
pub struct PyCommoditySwaptionBuilder {
    instrument_id: InstrumentId,
    commodity_type: Option<String>,
    ticker: Option<String>,
    unit: Option<String>,
    currency: Option<finstack_core::currency::Currency>,
    option_type: OptionType,
    expiry: Option<time::Date>,
    swap_start: Option<time::Date>,
    swap_end: Option<time::Date>,
    swap_frequency: Option<Tenor>,
    fixed_price: Option<f64>,
    notional: Option<f64>,
    forward_curve_id: Option<CurveId>,
    discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    calendar_id: Option<CalendarId>,
    bdc: BusinessDayConvention,
    day_count: DayCount,
    implied_volatility: Option<f64>,
}

impl PyCommoditySwaptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            commodity_type: None,
            ticker: None,
            unit: None,
            currency: None,
            option_type: OptionType::Call,
            expiry: None,
            swap_start: None,
            swap_end: None,
            swap_frequency: None,
            fixed_price: None,
            notional: None,
            forward_curve_id: None,
            discount_curve_id: None,
            vol_surface_id: None,
            calendar_id: None,
            bdc: BusinessDayConvention::ModifiedFollowing,
            day_count: DayCount::Act365F,
            implied_volatility: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.commodity_type.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("commodity_type() is required."));
        }
        if self.ticker.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("ticker() is required."));
        }
        if self.unit.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("unit() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.swap_start.is_none() {
            return Err(PyValueError::new_err("swap_start() is required."));
        }
        if self.swap_end.is_none() {
            return Err(PyValueError::new_err("swap_end() is required."));
        }
        if self.swap_frequency.is_none() {
            return Err(PyValueError::new_err("swap_frequency() is required."));
        }
        if self.fixed_price.is_none() {
            return Err(PyValueError::new_err("fixed_price() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.forward_curve_id.is_none() {
            return Err(PyValueError::new_err("forward_curve_id() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        if self.vol_surface_id.is_none() {
            return Err(PyValueError::new_err("vol_surface_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyCommoditySwaptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, commodity_type)")]
    fn commodity_type(mut slf: PyRefMut<'_, Self>, commodity_type: String) -> PyRefMut<'_, Self> {
        slf.commodity_type = Some(commodity_type);
        slf
    }

    #[pyo3(text_signature = "($self, ticker)")]
    fn ticker(mut slf: PyRefMut<'_, Self>, ticker: String) -> PyRefMut<'_, Self> {
        slf.ticker = Some(ticker);
        slf
    }

    #[pyo3(text_signature = "($self, unit)")]
    fn unit(mut slf: PyRefMut<'_, Self>, unit: String) -> PyRefMut<'_, Self> {
        slf.unit = Some(unit);
        slf
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.currency = Some(ccy);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type = parse_option_type(&option_type)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, expiry)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, swap_start)")]
    fn swap_start<'py>(
        mut slf: PyRefMut<'py, Self>,
        swap_start: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.swap_start = Some(py_to_date(&swap_start).context("swap_start")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, swap_end)")]
    fn swap_end<'py>(
        mut slf: PyRefMut<'py, Self>,
        swap_end: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.swap_end = Some(py_to_date(&swap_end).context("swap_end")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, swap_frequency)")]
    fn swap_frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        swap_frequency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let TenorArg(value) = swap_frequency.extract().context("swap_frequency")?;
        slf.swap_frequency = Some(value);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, fixed_price)")]
    fn fixed_price(mut slf: PyRefMut<'_, Self>, fixed_price: f64) -> PyRefMut<'_, Self> {
        slf.fixed_price = Some(fixed_price);
        slf
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional(mut slf: PyRefMut<'_, Self>, notional: f64) -> PyResult<PyRefMut<'_, Self>> {
        if notional <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.notional = Some(notional);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn forward_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn vol_surface_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.vol_surface_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, calendar_id=None)", signature = (calendar_id=None))]
    fn calendar_id(mut slf: PyRefMut<'_, Self>, calendar_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar_id = calendar_id.map(|value| CalendarId::new(value.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, bdc)")]
    fn bdc(mut slf: PyRefMut<'_, Self>, bdc: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.bdc = parse_bdc(&bdc)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(value) = day_count.extract().context("day_count")?;
        slf.day_count = value;
        Ok(slf)
    }

    #[pyo3(
        text_signature = "($self, implied_volatility=None)",
        signature = (implied_volatility=None)
    )]
    fn implied_volatility(
        mut slf: PyRefMut<'_, Self>,
        implied_volatility: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.implied_volatility = implied_volatility;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCommoditySwaption> {
        slf.ensure_ready()?;
        let commodity_type = slf.commodity_type.clone().ok_or_else(|| {
            PyRuntimeError::new_err(
                "CommoditySwaptionBuilder internal error: missing commodity_type after validation",
            )
        })?;
        let ticker = validated_clone("CommoditySwaptionBuilder", "ticker", slf.ticker.as_ref())?;
        let unit = validated_clone("CommoditySwaptionBuilder", "unit", slf.unit.as_ref())?;
        let currency = validated_field("CommoditySwaptionBuilder", "currency", slf.currency)?;
        let expiry = validated_field("CommoditySwaptionBuilder", "expiry", slf.expiry)?;
        let swap_start = validated_field("CommoditySwaptionBuilder", "swap_start", slf.swap_start)?;
        let swap_end = validated_field("CommoditySwaptionBuilder", "swap_end", slf.swap_end)?;
        let swap_frequency = validated_field(
            "CommoditySwaptionBuilder",
            "swap_frequency",
            slf.swap_frequency,
        )?;
        let fixed_price =
            validated_field("CommoditySwaptionBuilder", "fixed_price", slf.fixed_price)?;
        let notional = validated_field("CommoditySwaptionBuilder", "notional", slf.notional)?;
        let forward_curve_id = validated_clone(
            "CommoditySwaptionBuilder",
            "forward_curve_id",
            slf.forward_curve_id.as_ref(),
        )?;
        let discount_curve_id = validated_clone(
            "CommoditySwaptionBuilder",
            "discount_curve_id",
            slf.discount_curve_id.as_ref(),
        )?;
        let vol_surface_id = validated_clone(
            "CommoditySwaptionBuilder",
            "vol_surface_id",
            slf.vol_surface_id.as_ref(),
        )?;

        let mut builder = CommoditySwaption::builder()
            .id(slf.instrument_id.clone())
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency,
            ))
            .option_type(slf.option_type)
            .expiry(expiry)
            .swap_start(swap_start)
            .swap_end(swap_end)
            .swap_frequency(swap_frequency)
            .fixed_price(fixed_price)
            .notional(notional)
            .forward_curve_id(forward_curve_id)
            .discount_curve_id(discount_curve_id)
            .vol_surface_id(vol_surface_id)
            .bdc(slf.bdc)
            .day_count(slf.day_count);

        if let Some(calendar_id) = slf.calendar_id.clone() {
            builder = builder.calendar_id_opt(Some(calendar_id));
        }
        if let Some(implied_volatility) = slf.implied_volatility {
            builder =
                builder.pricing_overrides(option_pricing_overrides(Some(implied_volatility), None));
        }

        builder
            .build()
            .map(PyCommoditySwaption::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "CommoditySwaptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCommoditySwaption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCommoditySwaptionBuilder>> {
        Py::new(
            cls.py(),
            PyCommoditySwaptionBuilder::new_with_id(InstrumentId::new(instrument_id)),
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn commodity_type(&self) -> &str {
        &self.inner.underlying.commodity_type
    }

    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying.ticker
    }

    #[getter]
    fn unit(&self) -> &str {
        &self.inner.underlying.unit
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.underlying.currency)
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn swap_start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.swap_start)
    }

    #[getter]
    fn swap_end(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.swap_end)
    }

    #[getter]
    fn swap_frequency(&self) -> String {
        self.inner.swap_frequency.to_string()
    }

    #[getter]
    fn fixed_price(&self) -> f64 {
        self.inner.fixed_price
    }

    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional
    }

    #[getter]
    fn forward_curve_id(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface_id(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CommoditySwaption)
    }

    fn __repr__(&self) -> String {
        format!(
            "CommoditySwaption(id='{}', ticker='{}', option_type='{}')",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.option_type()
        )
    }
}

impl fmt::Display for PyCommoditySwaption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommoditySwaption({}, ticker={}, option_type={})",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            }
        )
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommoditySwaption>()?;
    parent.add_class::<PyCommoditySwaptionBuilder>()?;
    Ok(())
}
