use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_touch_option::{
    BarrierDirection, FxTouchOption, PayoutTiming, TouchType,
};
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::str::FromStr;
use std::sync::Arc;

fn parse_touch_type(value: &Bound<'_, PyAny>) -> PyResult<TouchType> {
    if let Ok(typed) = value.extract::<PyRef<'_, PyTouchType>>() {
        return Ok(typed.inner);
    }
    if let Ok(label) = value.extract::<&str>() {
        return TouchType::from_str(label).map_err(|e| PyValueError::new_err(e.to_string()));
    }
    Err(PyTypeError::new_err(
        "touch_type() expects TouchType or str",
    ))
}

fn parse_barrier_direction(value: &Bound<'_, PyAny>) -> PyResult<BarrierDirection> {
    if let Ok(typed) = value.extract::<PyRef<'_, PyBarrierDirection>>() {
        return Ok(typed.inner);
    }
    if let Ok(label) = value.extract::<&str>() {
        return BarrierDirection::from_str(label).map_err(|e| PyValueError::new_err(e.to_string()));
    }
    Err(PyTypeError::new_err(
        "barrier_direction() expects BarrierDirection or str",
    ))
}

fn parse_payout_timing(value: &Bound<'_, PyAny>) -> PyResult<PayoutTiming> {
    if let Ok(typed) = value.extract::<PyRef<'_, PyPayoutTiming>>() {
        return Ok(typed.inner);
    }
    if let Ok(label) = value.extract::<&str>() {
        return PayoutTiming::from_str(label).map_err(|e| PyValueError::new_err(e.to_string()));
    }
    Err(PyTypeError::new_err(
        "payout_timing() expects PayoutTiming or str",
    ))
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TouchType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyTouchType {
    pub(crate) inner: TouchType,
}

impl PyTouchType {
    pub(crate) const fn new(inner: TouchType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTouchType {
    #[classattr]
    const ONE_TOUCH: Self = Self::new(TouchType::OneTouch);
    #[classattr]
    const NO_TOUCH: Self = Self::new(TouchType::NoTouch);

    #[classmethod]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        TouchType::from_str(name)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            TouchType::OneTouch => "one_touch",
            TouchType::NoTouch => "no_touch",
            _ => "unknown",
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BarrierDirection",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBarrierDirection {
    pub(crate) inner: BarrierDirection,
}

impl PyBarrierDirection {
    pub(crate) const fn new(inner: BarrierDirection) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBarrierDirection {
    #[classattr]
    const UP: Self = Self::new(BarrierDirection::Up);
    #[classattr]
    const DOWN: Self = Self::new(BarrierDirection::Down);

    #[classmethod]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        BarrierDirection::from_str(name)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            BarrierDirection::Up => "up",
            BarrierDirection::Down => "down",
            _ => "unknown",
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PayoutTiming",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPayoutTiming {
    pub(crate) inner: PayoutTiming,
}

impl PyPayoutTiming {
    pub(crate) const fn new(inner: PayoutTiming) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPayoutTiming {
    #[classattr]
    const AT_HIT: Self = Self::new(PayoutTiming::AtHit);
    #[classattr]
    const AT_EXPIRY: Self = Self::new(PayoutTiming::AtExpiry);

    #[classmethod]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        PayoutTiming::from_str(name)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            PayoutTiming::AtHit => "at_hit",
            PayoutTiming::AtExpiry => "at_expiry",
            _ => "unknown",
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxTouchOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxTouchOption {
    pub(crate) inner: Arc<FxTouchOption>,
}

impl PyFxTouchOption {
    pub(crate) fn new(inner: FxTouchOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxTouchOptionBuilder"
)]
pub struct PyFxTouchOptionBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    barrier_level: Option<f64>,
    touch_type: Option<TouchType>,
    barrier_direction: Option<BarrierDirection>,
    payout_amount: Option<finstack_core::money::Money>,
    payout_timing: PayoutTiming,
    expiry: Option<time::Date>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    day_count: DayCount,
}

impl PyFxTouchOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            barrier_level: None,
            touch_type: None,
            barrier_direction: None,
            payout_amount: None,
            payout_timing: PayoutTiming::AtExpiry,
            expiry: None,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            vol_surface_id: None,
            day_count: DayCount::Act365F,
        }
    }
}

#[pymethods]
impl PyFxTouchOptionBuilder {
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(c) = ccy.extract().context("base_currency")?;
        slf.base_currency = Some(c);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(c) = ccy.extract().context("quote_currency")?;
        slf.quote_currency = Some(c);
        Ok(slf)
    }

    fn barrier_level(mut slf: PyRefMut<'_, Self>, level: f64) -> PyRefMut<'_, Self> {
        slf.barrier_level = Some(level);
        slf
    }

    fn touch_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        touch_type: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.touch_type = Some(parse_touch_type(&touch_type)?);
        Ok(slf)
    }

    fn barrier_direction<'py>(
        mut slf: PyRefMut<'py, Self>,
        direction: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.barrier_direction = Some(parse_barrier_direction(&direction)?);
        Ok(slf)
    }

    fn payout_amount<'py>(
        mut slf: PyRefMut<'py, Self>,
        amount: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.payout_amount = Some(extract_money(&amount).context("payout_amount")?);
        Ok(slf)
    }

    fn payout_timing<'py>(
        mut slf: PyRefMut<'py, Self>,
        timing: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.payout_timing = parse_payout_timing(&timing)?;
        Ok(slf)
    }

    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&date).context("expiry")?);
        Ok(slf)
    }

    fn domestic_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.domestic_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn foreign_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.foreign_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn vol_surface<'py>(mut slf: PyRefMut<'py, Self>, surface_id: &str) -> PyRefMut<'py, Self> {
        slf.vol_surface_id = Some(CurveId::new(surface_id));
        slf
    }

    fn day_count<'py>(mut slf: PyRefMut<'py, Self>, dc: &PyDayCount) -> PyRefMut<'py, Self> {
        slf.day_count = dc.inner;
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxTouchOption> {
        let base = slf
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;
        let quote = slf
            .quote_currency
            .ok_or_else(|| PyValueError::new_err("quote_currency is required"))?;
        let barrier_level = slf
            .barrier_level
            .ok_or_else(|| PyValueError::new_err("barrier_level is required"))?;
        let touch_type = slf
            .touch_type
            .ok_or_else(|| PyValueError::new_err("touch_type is required"))?;
        let barrier_direction = slf
            .barrier_direction
            .ok_or_else(|| PyValueError::new_err("barrier_direction is required"))?;
        let payout_amount = slf
            .payout_amount
            .ok_or_else(|| PyValueError::new_err("payout_amount is required"))?;
        let expiry = slf
            .expiry
            .ok_or_else(|| PyValueError::new_err("expiry is required"))?;
        let domestic = slf
            .domestic_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("domestic_discount_curve is required"))?;
        let foreign = slf
            .foreign_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("foreign_discount_curve is required"))?;
        let vol = slf
            .vol_surface_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("vol_surface is required"))?;

        let option = FxTouchOption::builder()
            .id(slf.instrument_id.clone())
            .base_currency(base)
            .quote_currency(quote)
            .barrier_level(barrier_level)
            .touch_type(touch_type)
            .barrier_direction(barrier_direction)
            .payout_amount(payout_amount)
            .payout_timing(slf.payout_timing)
            .expiry(expiry)
            .day_count(slf.day_count)
            .domestic_discount_curve_id(domestic)
            .foreign_discount_curve_id(foreign)
            .vol_surface_id(vol)
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
            .attributes(finstack_valuations::instruments::Attributes::new())
            .build()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to build FxTouchOption: {e}"
                ))
            })?;
        Ok(PyFxTouchOption::new(option))
    }

    fn __repr__(&self) -> String {
        format!("FxTouchOptionBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyFxTouchOption {
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyFxTouchOptionBuilder {
        PyFxTouchOptionBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxTouchOption)
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    #[getter]
    fn barrier_level(&self) -> f64 {
        self.inner.barrier_level
    }

    #[getter]
    fn touch_type(&self) -> PyTouchType {
        PyTouchType::new(self.inner.touch_type)
    }

    #[getter]
    fn barrier_direction(&self) -> PyBarrierDirection {
        PyBarrierDirection::new(self.inner.barrier_direction)
    }

    #[getter]
    fn payout_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.payout_amount)
    }

    #[getter]
    fn payout_timing(&self) -> PyPayoutTiming {
        PyPayoutTiming::new(self.inner.payout_timing)
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn domestic_discount_curve(&self) -> String {
        self.inner.domestic_discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn foreign_discount_curve(&self) -> String {
        self.inner.foreign_discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Calculate present value of the FX touch option.
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| Instrument::value(self.inner.as_ref(), &market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn __repr__(&self) -> String {
        format!(
            "FxTouchOption(id='{}', barrier={}, touch_type='{}', direction='{}')",
            self.inner.id.as_str(),
            self.inner.barrier_level,
            match self.inner.touch_type {
                TouchType::OneTouch => "one_touch",
                TouchType::NoTouch => "no_touch",
                _ => "unknown",
            },
            match self.inner.barrier_direction {
                BarrierDirection::Up => "up",
                BarrierDirection::Down => "down",
                _ => "unknown",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyTouchType>()?;
    parent.add_class::<PyBarrierDirection>()?;
    parent.add_class::<PyPayoutTiming>()?;
    parent.add_class::<PyFxTouchOption>()?;
    parent.add_class::<PyFxTouchOptionBuilder>()?;
    Ok(vec![
        "TouchType",
        "BarrierDirection",
        "PayoutTiming",
        "FxTouchOption",
        "FxTouchOptionBuilder",
    ])
}
