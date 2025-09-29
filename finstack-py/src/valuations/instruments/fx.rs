use crate::core::common::args::{BusinessDayConventionArg, CurrencyArg};
use crate::core::currency::PyCurrency;
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::fx_option::FxOption;
use finstack_valuations::instruments::fx_spot::FxSpot;
use finstack_valuations::instruments::fx_swap::FxSwap;
use finstack_valuations::instruments::{ExerciseStyle, OptionType, SettlementType};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

/// FX spot instrument exchanging base currency for quote currency.
#[pyclass(module = "finstack.valuations.instruments", name = "FxSpot", frozen)]
#[derive(Clone, Debug)]
pub struct PyFxSpot {
    pub(crate) inner: FxSpot,
}

impl PyFxSpot {
    pub(crate) fn new(inner: FxSpot) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxSpot {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, base_currency, quote_currency, *, settlement=None, settlement_lag_days=None, spot_rate=None, notional=None, bdc='following', calendar=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            base_currency,
            quote_currency,
            *,
            settlement=None,
            settlement_lag_days=None,
            spot_rate=None,
            notional=None,
            bdc=None,
            calendar=None
        )
    )]
    /// Create an FX spot position with optional settlement overrides.
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        base_currency: Bound<'_, PyAny>,
        quote_currency: Bound<'_, PyAny>,
        settlement: Option<Bound<'_, PyAny>>,
        settlement_lag_days: Option<i32>,
        spot_rate: Option<f64>,
        notional: Option<Bound<'_, PyAny>>,
        bdc: Option<Bound<'_, PyAny>>,
        calendar: Option<&str>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let CurrencyArg(base) = base_currency.extract()?;
        let CurrencyArg(quote) = quote_currency.extract()?;
        let mut inst = FxSpot::new(id, base, quote);

        if let Some(date_obj) = settlement {
            let date = py_to_date(&date_obj)?;
            inst = inst.with_settlement(date);
        }
        if let Some(lag) = settlement_lag_days {
            inst.settlement_lag_days = Some(lag);
        }
        if let Some(rate) = spot_rate {
            inst = inst.with_rate(rate);
        }
        if let Some(notional_obj) = notional {
            let money = extract_money(&notional_obj)?;
            inst = inst.try_with_notional(money).map_err(core_to_py)?;
        }
        if let Some(bdc_obj) = bdc {
            let BusinessDayConventionArg(conv) = bdc_obj.extract()?;
            inst = inst.with_bdc(conv);
        }
        if let Some(id) = calendar {
            // Leak calendar id to static lifetime as required by core types
            let leaked: &'static str = Box::leak(id.to_string().into_boxed_str());
            inst = inst.with_calendar_id(leaked);
        }

        Ok(Self::new(inst))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Base currency (FX numerator).
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base)
    }

    /// Quote currency (FX denominator).
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote)
    }

    /// Optional notional in base currency (defaults to 1 unit when absent).
    #[getter]
    fn notional(&self) -> Option<PyMoney> {
        self.inner.notional.map(PyMoney::new)
    }

    /// Explicit spot rate if provided.
    #[getter]
    fn spot_rate(&self) -> Option<f64> {
        self.inner.spot_rate
    }

    /// Settlement date if provided.
    #[getter]
    fn settlement(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        Ok(match self.inner.settlement {
            Some(date) => Some(date_to_py(py, date)?),
            None => None,
        })
    }

    /// Settlement lag in business days when settlement date is inferred.
    #[getter]
    fn settlement_lag_days(&self) -> Option<i32> {
        self.inner.settlement_lag_days
    }

    /// Business-day convention used when adjusting settlement.
    #[getter]
    fn business_day_convention(&self) -> &'static str {
        match self.inner.bdc {
            finstack_core::dates::BusinessDayConvention::Following => "following",
            finstack_core::dates::BusinessDayConvention::ModifiedFollowing => "modified_following",
            finstack_core::dates::BusinessDayConvention::Preceding => "preceding",
            finstack_core::dates::BusinessDayConvention::ModifiedPreceding => "modified_preceding",
            finstack_core::dates::BusinessDayConvention::Unadjusted => "unadjusted",
            _ => "unadjusted",
        }
    }

    /// Optional settlement calendar identifier.
    #[getter]
    fn calendar_id(&self) -> Option<&'static str> {
        self.inner.calendar_id
    }

    /// FX pair mnemonic such as ``"EURUSD"``.
    #[getter]
    fn pair_name(&self) -> String {
        self.inner.pair_name()
    }

    /// Instrument type enum (``InstrumentType.FX_SPOT``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxSpot)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "FxSpot(id='{}', pair='{}')",
            self.inner.id,
            self.inner.pair_name()
        ))
    }
}

impl fmt::Display for PyFxSpot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FxSpot({}, pair={})",
            self.inner.id,
            self.inner.pair_name()
        )
    }
}

/// Garman–Kohlhagen FX option with European exercise.
#[pyclass(module = "finstack.valuations.instruments", name = "FxOption", frozen)]
#[derive(Clone, Debug)]
pub struct PyFxOption {
    pub(crate) inner: FxOption,
}

impl PyFxOption {
    pub(crate) fn new(inner: FxOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, base_currency, quote_currency, strike, expiry, notional)"
    )]
    /// Create a European call option with standard USD-centric curves.
    fn european_call(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        base_currency: Bound<'_, PyAny>,
        quote_currency: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let CurrencyArg(base) = base_currency.extract()?;
        let CurrencyArg(quote) = quote_currency.extract()?;
        let expiry_date = py_to_date(&expiry)?;
        let amt = extract_money(&notional)?;
        Ok(Self::new(FxOption::european_call(
            id,
            base,
            quote,
            strike,
            expiry_date,
            amt,
        )))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, base_currency, quote_currency, strike, expiry, notional)"
    )]
    /// Create a European put option with standard USD-centric curves.
    fn european_put(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        base_currency: Bound<'_, PyAny>,
        quote_currency: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let CurrencyArg(base) = base_currency.extract()?;
        let CurrencyArg(quote) = quote_currency.extract()?;
        let expiry_date = py_to_date(&expiry)?;
        let amt = extract_money(&notional)?;
        Ok(Self::new(FxOption::european_put(
            id,
            base,
            quote,
            strike,
            expiry_date,
            amt,
        )))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Base currency for the option underlying.
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency for settlement.
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Notional amount in base currency.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike rate expressed as quote per unit of base.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.expiry)
    }

    /// Option type (``"call"`` or ``"put"``).
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Exercise style (currently ``"european"`` for simplified constructors).
    #[getter]
    fn exercise_style(&self) -> &'static str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
        }
    }

    /// Settlement type (cash vs. physical).
    #[getter]
    fn settlement(&self) -> &'static str {
        match self.inner.settlement {
            SettlementType::Cash => "cash",
            SettlementType::Physical => "physical",
        }
    }

    /// Domestic discount curve identifier.
    #[getter]
    fn domestic_curve(&self) -> String {
        self.inner.domestic_disc_id.as_str().to_string()
    }

    /// Foreign discount curve identifier.
    #[getter]
    fn foreign_curve(&self) -> String {
        self.inner.foreign_disc_id.as_str().to_string()
    }

    /// Volatility surface identifier used for pricing.
    #[getter]
    fn vol_surface(&self) -> &'static str {
        self.inner.vol_id
    }

    /// Instrument type enum (``InstrumentType.FX_OPTION``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxOption)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "FxOption(id='{}', type='{}', strike={:.4})",
            self.inner.id,
            self.option_type(),
            self.inner.strike
        ))
    }
}

impl fmt::Display for PyFxOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FxOption({}, type={}, strike={:.4})",
            self.inner.id,
            self.option_type(),
            self.inner.strike
        )
    }
}

/// FX swap exchanging notionals on near and far legs.
#[pyclass(module = "finstack.valuations.instruments", name = "FxSwap", frozen)]
#[derive(Clone, Debug)]
pub struct PyFxSwap {
    pub(crate) inner: FxSwap,
}

impl PyFxSwap {
    pub(crate) fn new(inner: FxSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxSwap {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, base_currency, quote_currency, notional, near_date, far_date, domestic_curve, foreign_curve, /, *, near_rate=None, far_rate=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an FX swap specifying near/far legs and associated curves.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        base_currency: Bound<'_, PyAny>,
        quote_currency: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        near_date: Bound<'_, PyAny>,
        far_date: Bound<'_, PyAny>,
        domestic_curve: Bound<'_, PyAny>,
        foreign_curve: Bound<'_, PyAny>,
        near_rate: Option<f64>,
        far_rate: Option<f64>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let CurrencyArg(base) = base_currency.extract()?;
        let CurrencyArg(quote) = quote_currency.extract()?;
        let base_notional = extract_money(&notional)?;
        let near = py_to_date(&near_date)?;
        let far = py_to_date(&far_date)?;
        let domestic = extract_curve_id(&domestic_curve)?;
        let foreign = extract_curve_id(&foreign_curve)?;

        let mut builder = FxSwap::builder();
        builder = builder.id(id);
        builder = builder.base_currency(base);
        builder = builder.quote_currency(quote);
        builder = builder.near_date(near);
        builder = builder.far_date(far);
        builder = builder.base_notional(base_notional);
        builder = builder.domestic_disc_id(domestic);
        builder = builder.foreign_disc_id(foreign);
        if let Some(rate) = near_rate {
            builder = builder.near_rate(rate);
        }
        if let Some(rate) = far_rate {
            builder = builder.far_rate(rate);
        }

        let swap = builder.build().map_err(core_to_py)?;
        Ok(Self::new(swap))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Base currency exchanged on the swap.
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency exchanged on the swap.
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Base notional in the base currency.
    #[getter]
    fn base_notional(&self) -> PyMoney {
        PyMoney::new(self.inner.base_notional)
    }

    /// Near leg settlement date.
    #[getter]
    fn near_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.near_date)
    }

    /// Far leg settlement date.
    #[getter]
    fn far_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.far_date)
    }

    /// Optional contractual near FX rate.
    #[getter]
    fn near_rate(&self) -> Option<f64> {
        self.inner.near_rate
    }

    /// Optional contractual far FX rate.
    #[getter]
    fn far_rate(&self) -> Option<f64> {
        self.inner.far_rate
    }

    /// Domestic discount curve identifier.
    #[getter]
    fn domestic_curve(&self) -> String {
        self.inner.domestic_disc_id.as_str().to_string()
    }

    /// Foreign discount curve identifier.
    #[getter]
    fn foreign_curve(&self) -> String {
        self.inner.foreign_disc_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.FX_SWAP``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxSwap)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "FxSwap(id='{}', near='{}', far='{}')",
            self.inner.id, self.inner.near_date, self.inner.far_date
        ))
    }
}

impl fmt::Display for PyFxSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FxSwap({}, near={}, far={})",
            self.inner.id, self.inner.near_date, self.inner.far_date
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyFxSpot>()?;
    module.add_class::<PyFxOption>()?;
    module.add_class::<PyFxSwap>()?;
    Ok(vec!["FxSpot", "FxOption", "FxSwap"])
}
