use super::common::{require_builder_clone, require_builder_field, require_notional_money};
use crate::core::common::args::CurrencyArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::ir_future_option::IrFutureOption;
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

fn parse_option_type(label: &str) -> PyResult<OptionType> {
    OptionType::from_str(label).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Exchange-traded option on an interest rate future.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "IrFutureOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyIrFutureOption {
    pub(crate) inner: Arc<IrFutureOption>,
}

impl PyIrFutureOption {
    pub(crate) fn new(inner: IrFutureOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "IrFutureOptionBuilder"
)]
pub struct PyIrFutureOptionBuilder {
    instrument_id: InstrumentId,
    futures_price: Option<f64>,
    strike: Option<f64>,
    expiry: Option<time::Date>,
    option_type: OptionType,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    tick_size: Option<f64>,
    tick_value: Option<f64>,
    volatility: Option<f64>,
    discount_curve_id: Option<CurveId>,
}

impl PyIrFutureOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            futures_price: None,
            strike: None,
            expiry: None,
            option_type: OptionType::Call,
            pending_notional_amount: None,
            pending_currency: None,
            tick_size: None,
            tick_value: None,
            volatility: None,
            discount_curve_id: None,
        }
    }

    fn notional_money(&self) -> Option<finstack_core::money::Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => {
                Some(finstack_core::money::Money::new(amount, currency))
            }
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.futures_price.is_none() {
            return Err(PyValueError::new_err("futures_price() is required."));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "A notional must be provided via money() or notional() + currency().",
            ));
        }
        if self.tick_size.is_none() {
            return Err(PyValueError::new_err("tick_size() is required."));
        }
        if self.tick_value.is_none() {
            return Err(PyValueError::new_err("tick_value() is required."));
        }
        if self.volatility.is_none() {
            return Err(PyValueError::new_err("volatility() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyIrFutureOptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, futures_price)")]
    fn futures_price(mut slf: PyRefMut<'_, Self>, futures_price: f64) -> PyRefMut<'_, Self> {
        slf.futures_price = Some(futures_price);
        slf
    }

    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    #[pyo3(text_signature = "($self, expiry)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
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

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(
        mut slf: PyRefMut<'py, Self>,
        money: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let money = extract_money(&money).context("money")?;
        slf.pending_notional_amount = Some(money.amount());
        slf.pending_currency = Some(money.currency());
        Ok(slf)
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
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.pending_currency = Some(ccy);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, tick_size)")]
    fn tick_size(mut slf: PyRefMut<'_, Self>, tick_size: f64) -> PyRefMut<'_, Self> {
        slf.tick_size = Some(tick_size);
        slf
    }

    #[pyo3(text_signature = "($self, tick_value)")]
    fn tick_value(mut slf: PyRefMut<'_, Self>, tick_value: f64) -> PyRefMut<'_, Self> {
        slf.tick_value = Some(tick_value);
        slf
    }

    #[pyo3(text_signature = "($self, volatility)")]
    fn volatility(mut slf: PyRefMut<'_, Self>, volatility: f64) -> PyRefMut<'_, Self> {
        slf.volatility = Some(volatility);
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyIrFutureOption> {
        slf.ensure_ready()?;
        let futures_price =
            require_builder_field("IrFutureOptionBuilder", "futures_price", slf.futures_price)?;
        let strike = require_builder_field("IrFutureOptionBuilder", "strike", slf.strike)?;
        let expiry = require_builder_field("IrFutureOptionBuilder", "expiry", slf.expiry)?;
        let notional = require_notional_money(
            "IrFutureOptionBuilder",
            slf.pending_notional_amount,
            slf.pending_currency,
        )?;
        let tick_size = require_builder_field("IrFutureOptionBuilder", "tick_size", slf.tick_size)?;
        let tick_value =
            require_builder_field("IrFutureOptionBuilder", "tick_value", slf.tick_value)?;
        let volatility =
            require_builder_field("IrFutureOptionBuilder", "volatility", slf.volatility)?;
        let discount_curve_id = require_builder_clone(
            "IrFutureOptionBuilder",
            "discount_curve",
            slf.discount_curve_id.as_ref(),
        )?;

        IrFutureOption::builder()
            .id(slf.instrument_id.clone())
            .futures_price(futures_price)
            .strike(strike)
            .expiry(expiry)
            .option_type(slf.option_type)
            .notional(notional)
            .tick_size(tick_size)
            .tick_value(tick_value)
            .volatility(volatility)
            .discount_curve_id(discount_curve_id)
            .build()
            .map(PyIrFutureOption::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "IrFutureOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyIrFutureOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyIrFutureOptionBuilder>> {
        Py::new(
            cls.py(),
            PyIrFutureOptionBuilder::new_with_id(InstrumentId::new(instrument_id)),
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn futures_price(&self) -> f64 {
        self.inner.futures_price
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    #[getter]
    fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }

    #[getter]
    fn volatility(&self) -> f64 {
        self.inner.volatility
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::IrFutureOption)
    }

    fn __repr__(&self) -> String {
        format!(
            "IrFutureOption(id='{}', strike={}, option_type='{}')",
            self.inner.id.as_str(),
            self.inner.strike,
            self.option_type()
        )
    }
}

impl fmt::Display for PyIrFutureOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IrFutureOption({}, strike={}, option_type={})",
            self.inner.id.as_str(),
            self.inner.strike,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyIrFutureOption>()?;
    module.add_class::<PyIrFutureOptionBuilder>()?;
    Ok(vec!["IrFutureOption", "IrFutureOptionBuilder"])
}
