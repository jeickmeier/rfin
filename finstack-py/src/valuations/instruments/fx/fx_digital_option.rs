use crate::core::common::args::CurrencyArg;
use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_digital_option::{DigitalPayoutType, FxDigitalOption};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::sync::Arc;

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxDigitalOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxDigitalOption {
    pub(crate) inner: Arc<FxDigitalOption>,
}

impl PyFxDigitalOption {
    pub(crate) fn new(inner: FxDigitalOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxDigitalOptionBuilder",
    unsendable
)]
pub struct PyFxDigitalOptionBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    option_type: OptionType,
    payout_type: Option<DigitalPayoutType>,
    payout_amount: Option<finstack_core::money::Money>,
    expiry: Option<time::Date>,
    notional: Option<finstack_core::money::Money>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    day_count: DayCount,
}

impl PyFxDigitalOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            strike: None,
            option_type: OptionType::Call,
            payout_type: None,
            payout_amount: None,
            expiry: None,
            notional: None,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            vol_surface_id: None,
            day_count: DayCount::Act365F,
        }
    }
}

#[pymethods]
impl PyFxDigitalOptionBuilder {
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

    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    fn option_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        option_type: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.option_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };
        Ok(slf)
    }

    fn payout_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        payout_type: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.payout_type = Some(match normalize_label(payout_type).as_str() {
            "cash_or_nothing" | "cashornothing" => DigitalPayoutType::CashOrNothing,
            "asset_or_nothing" | "assetornothing" => DigitalPayoutType::AssetOrNothing,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown payout type: {other}"
                )))
            }
        });
        Ok(slf)
    }

    fn payout_amount<'py>(
        mut slf: PyRefMut<'py, Self>,
        amount: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.payout_amount = Some(extract_money(&amount).context("payout_amount")?);
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

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxDigitalOption> {
        let base = slf
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;
        let quote = slf
            .quote_currency
            .ok_or_else(|| PyValueError::new_err("quote_currency is required"))?;
        let strike = slf
            .strike
            .ok_or_else(|| PyValueError::new_err("strike is required"))?;
        let payout_type = slf
            .payout_type
            .ok_or_else(|| PyValueError::new_err("payout_type is required"))?;
        let payout_amount = slf
            .payout_amount
            .ok_or_else(|| PyValueError::new_err("payout_amount is required"))?;
        let expiry = slf
            .expiry
            .ok_or_else(|| PyValueError::new_err("expiry is required"))?;
        let notional = slf
            .notional
            .ok_or_else(|| PyValueError::new_err("notional is required"))?;
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

        let option = FxDigitalOption::builder()
            .id(slf.instrument_id.clone())
            .base_currency(base)
            .quote_currency(quote)
            .strike(strike)
            .option_type(slf.option_type)
            .payout_type(payout_type)
            .payout_amount(payout_amount)
            .expiry(expiry)
            .day_count(slf.day_count)
            .notional(notional)
            .domestic_discount_curve_id(domestic)
            .foreign_discount_curve_id(foreign)
            .vol_surface_id(vol)
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
            .attributes(finstack_valuations::instruments::Attributes::new())
            .build()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to build FxDigitalOption: {e}"
                ))
            })?;
        Ok(PyFxDigitalOption::new(option))
    }

    fn __repr__(&self) -> String {
        format!("FxDigitalOptionBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyFxDigitalOption {
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyFxDigitalOptionBuilder {
        PyFxDigitalOptionBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxDigitalOption)
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
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    #[getter]
    fn payout_type(&self) -> &'static str {
        match self.inner.payout_type {
            DigitalPayoutType::CashOrNothing => "cash_or_nothing",
            DigitalPayoutType::AssetOrNothing => "asset_or_nothing",
            _ => "unknown",
        }
    }

    #[getter]
    fn payout_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.payout_amount)
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

    /// Calculate present value of the FX digital option.
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
            "FxDigitalOption(id='{}', strike={}, option_type='{}', payout_type='{}')",
            self.inner.id.as_str(),
            self.inner.strike,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            },
            match self.inner.payout_type {
                DigitalPayoutType::CashOrNothing => "cash_or_nothing",
                DigitalPayoutType::AssetOrNothing => "asset_or_nothing",
                _ => "unknown",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyFxDigitalOption>()?;
    parent.add_class::<PyFxDigitalOptionBuilder>()?;
    Ok(vec!["FxDigitalOption", "FxDigitalOptionBuilder"])
}
