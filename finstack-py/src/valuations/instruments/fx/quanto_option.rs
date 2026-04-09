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
use finstack_valuations::instruments::fx::quanto_option::QuantoOption;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::str::FromStr;
use std::sync::Arc;

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "QuantoOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyQuantoOption {
    pub(crate) inner: Arc<QuantoOption>,
}

impl PyQuantoOption {
    pub(crate) fn new(inner: QuantoOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "QuantoOptionBuilder"
)]
pub struct PyQuantoOptionBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    ticker: Option<String>,
    equity_strike: Option<f64>,
    option_type: OptionType,
    expiry: Option<time::Date>,
    notional: Option<finstack_core::money::Money>,
    correlation: Option<f64>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    spot_id: Option<String>,
    vol_surface_id: Option<CurveId>,
    div_yield_id: Option<CurveId>,
    fx_rate_id: Option<String>,
    fx_vol_id: Option<CurveId>,
    day_count: DayCount,
    underlying_quantity: Option<f64>,
    payoff_fx_rate: Option<f64>,
}

impl PyQuantoOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            ticker: None,
            equity_strike: None,
            option_type: OptionType::Call,
            expiry: None,
            notional: None,
            correlation: None,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            spot_id: None,
            vol_surface_id: None,
            div_yield_id: None,
            fx_rate_id: None,
            fx_vol_id: None,
            day_count: DayCount::Act365F,
            underlying_quantity: None,
            payoff_fx_rate: None,
        }
    }
}

#[pymethods]
impl PyQuantoOptionBuilder {
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

    fn ticker<'py>(mut slf: PyRefMut<'py, Self>, ticker: &str) -> PyRefMut<'py, Self> {
        slf.ticker = Some(ticker.to_string());
        slf
    }

    fn equity_strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.equity_strike = Some(strike);
        slf
    }

    fn option_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        option_type: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.option_type =
            OptionType::from_str(option_type).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&date).context("expiry")?);
        Ok(slf)
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    fn correlation(mut slf: PyRefMut<'_, Self>, correlation: f64) -> PyRefMut<'_, Self> {
        slf.correlation = Some(correlation);
        slf
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

    fn spot_id<'py>(mut slf: PyRefMut<'py, Self>, id: &str) -> PyRefMut<'py, Self> {
        slf.spot_id = Some(id.to_string());
        slf
    }

    fn vol_surface<'py>(mut slf: PyRefMut<'py, Self>, surface_id: &str) -> PyRefMut<'py, Self> {
        slf.vol_surface_id = Some(CurveId::new(surface_id));
        slf
    }

    fn div_yield_id<'py>(mut slf: PyRefMut<'py, Self>, curve_id: &str) -> PyRefMut<'py, Self> {
        slf.div_yield_id = Some(CurveId::new(curve_id));
        slf
    }

    fn fx_rate_id<'py>(mut slf: PyRefMut<'py, Self>, rate_id: &str) -> PyRefMut<'py, Self> {
        slf.fx_rate_id = Some(rate_id.to_string());
        slf
    }

    fn fx_vol_id<'py>(mut slf: PyRefMut<'py, Self>, vol_id: &str) -> PyRefMut<'py, Self> {
        slf.fx_vol_id = Some(CurveId::new(vol_id));
        slf
    }

    fn day_count<'py>(mut slf: PyRefMut<'py, Self>, dc: &PyDayCount) -> PyRefMut<'py, Self> {
        slf.day_count = dc.inner;
        slf
    }

    fn underlying_quantity(mut slf: PyRefMut<'_, Self>, qty: f64) -> PyRefMut<'_, Self> {
        slf.underlying_quantity = Some(qty);
        slf
    }

    fn payoff_fx_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyRefMut<'_, Self> {
        slf.payoff_fx_rate = Some(rate);
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyQuantoOption> {
        let base = slf
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;
        let quote = slf
            .quote_currency
            .ok_or_else(|| PyValueError::new_err("quote_currency is required"))?;
        let ticker = slf
            .ticker
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("ticker is required"))?
            .clone();
        let equity_strike_val = slf
            .equity_strike
            .ok_or_else(|| PyValueError::new_err("equity_strike is required"))?;
        let expiry = slf
            .expiry
            .ok_or_else(|| PyValueError::new_err("expiry is required"))?;
        let notional = slf
            .notional
            .ok_or_else(|| PyValueError::new_err("notional is required"))?;
        let correlation = slf
            .correlation
            .ok_or_else(|| PyValueError::new_err("correlation is required"))?;
        let domestic = slf
            .domestic_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("domestic_discount_curve is required"))?;
        let foreign = slf
            .foreign_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("foreign_discount_curve is required"))?;
        let spot_id = slf
            .spot_id
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("spot_id is required"))?
            .clone();
        let vol = slf
            .vol_surface_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("vol_surface is required"))?;

        let equity_strike_money = finstack_core::money::Money::new(equity_strike_val, base);

        let mut builder = QuantoOption::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.underlying_ticker(ticker);
        builder = builder.equity_strike(equity_strike_money);
        builder = builder.option_type(slf.option_type);
        builder = builder.expiry(expiry);
        builder = builder.notional(notional);
        builder = builder.base_currency(base);
        builder = builder.quote_currency(quote);
        builder = builder.correlation(correlation);
        builder = builder.day_count(slf.day_count);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.domestic_discount_curve_id(domestic);
        builder = builder.foreign_discount_curve_id(foreign);
        builder = builder.spot_id(spot_id.into());
        builder = builder.vol_surface_id(vol);
        if let Some(ref div) = slf.div_yield_id {
            builder = builder.div_yield_id(div.clone());
        }
        if let Some(ref fx_rate) = slf.fx_rate_id {
            builder = builder.fx_rate_id(fx_rate.clone());
        }
        if let Some(ref fx_vol) = slf.fx_vol_id {
            builder = builder.fx_vol_id(fx_vol.clone());
        }
        if let Some(qty) = slf.underlying_quantity {
            builder = builder.underlying_quantity_opt(Some(qty));
        }
        if let Some(rate) = slf.payoff_fx_rate {
            builder = builder.payoff_fx_rate_opt(Some(rate));
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build QuantoOption: {e}"))
        })?;
        Ok(PyQuantoOption::new(option))
    }

    fn __repr__(&self) -> String {
        format!("QuantoOptionBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyQuantoOption {
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyQuantoOptionBuilder {
        PyQuantoOptionBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::QuantoOption)
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
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    #[getter]
    fn equity_strike(&self) -> PyMoney {
        PyMoney::new(self.inner.equity_strike)
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
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn correlation(&self) -> f64 {
        self.inner.correlation
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
    fn spot_id(&self) -> String {
        self.inner.spot_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn div_yield_id(&self) -> Option<String> {
        self.inner
            .div_yield_id
            .as_ref()
            .map(|c| c.as_str().to_string())
    }

    #[getter]
    fn fx_rate_id(&self) -> Option<String> {
        self.inner.fx_rate_id.clone()
    }

    #[getter]
    fn fx_vol_id(&self) -> Option<String> {
        self.inner
            .fx_vol_id
            .as_ref()
            .map(|c| c.as_str().to_string())
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Calculate present value of the quanto option.
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
            "QuantoOption(id='{}', ticker='{}', correlation={})",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.correlation
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyQuantoOption>()?;
    parent.add_class::<PyQuantoOptionBuilder>()?;
    Ok(vec!["QuantoOption", "QuantoOptionBuilder"])
}
