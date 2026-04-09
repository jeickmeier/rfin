use crate::core::common::args::{BusinessDayConventionArg, CurrencyArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::intern_calendar_id_opt;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_option::{FxAtmDeltaConvention, FxOption};
use finstack_valuations::instruments::fx::fx_spot::FxSpot;
use finstack_valuations::instruments::fx::fx_swap::FxSwap;
use finstack_valuations::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// FX spot instrument exchanging base currency for quote currency.
///
/// Examples:
///     >>> spot = FxSpot.builder("eurusd_spot").base_currency("EUR").quote_currency("USD").spot_rate(1.095).build()
///     >>> spot.pair_name
///     'EURUSD'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxSpot",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxSpot {
    pub(crate) inner: Arc<FxSpot>,
}

impl PyFxSpot {
    pub(crate) fn new(inner: FxSpot) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(module = "finstack.valuations.instruments", name = "FxSpotBuilder")]
pub struct PyFxSpotBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    settlement: Option<time::Date>,
    settlement_lag_days: Option<i32>,
    spot_rate: Option<f64>,
    notional: Option<finstack_core::money::Money>,
    bdc: Option<finstack_core::dates::BusinessDayConvention>,
    base_calendar_id: Option<String>,
    quote_calendar_id: Option<String>,
}

impl PyFxSpotBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            settlement: None,
            settlement_lag_days: None,
            spot_rate: None,
            notional: None,
            bdc: None,
            base_calendar_id: None,
            quote_calendar_id: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.base_currency.is_none() {
            return Err(PyValueError::new_err("base_currency() is required."));
        }
        if self.quote_currency.is_none() {
            return Err(PyValueError::new_err("quote_currency() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyFxSpotBuilder {
    #[new]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        base_currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let CurrencyArg(base) = base_currency.extract().context("base_currency")?;
        slf.base_currency = Some(base);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        quote_currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let CurrencyArg(quote) = quote_currency.extract().context("quote_currency")?;
        slf.quote_currency = Some(quote);
        Ok(slf)
    }

    fn settlement<'py>(
        mut slf: PyRefMut<'py, Self>,
        settlement: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.settlement = Some(py_to_date(&settlement).context("settlement")?);
        Ok(slf)
    }

    fn settlement_lag_days(
        mut slf: PyRefMut<'_, Self>,
        settlement_lag_days: i32,
    ) -> PyRefMut<'_, Self> {
        slf.settlement_lag_days = Some(settlement_lag_days);
        slf
    }

    fn spot_rate(mut slf: PyRefMut<'_, Self>, spot_rate: f64) -> PyRefMut<'_, Self> {
        slf.spot_rate = Some(spot_rate);
        slf
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    fn bdc<'py>(
        mut slf: PyRefMut<'py, Self>,
        bdc: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let BusinessDayConventionArg(conv) = bdc.extract().context("bdc")?;
        slf.bdc = Some(conv);
        Ok(slf)
    }

    fn base_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.base_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn quote_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.quote_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxSpot> {
        slf.ensure_ready()?;
        let base = slf.base_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSpotBuilder internal error: missing base_currency after validation",
            )
        })?;
        let quote = slf.quote_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSpotBuilder internal error: missing quote_currency after validation",
            )
        })?;

        let mut inst = FxSpot::new(slf.instrument_id.clone(), base, quote);
        if let Some(date) = slf.settlement {
            inst = inst.with_settlement(date);
        }
        if let Some(lag) = slf.settlement_lag_days {
            inst.settlement_lag_days = Some(lag);
        }
        if let Some(rate) = slf.spot_rate {
            inst = inst.with_rate(rate).map_err(core_to_py)?;
        }
        if let Some(money) = slf.notional {
            inst = inst.with_notional(money).map_err(core_to_py)?;
        }
        if let Some(conv) = slf.bdc {
            inst = inst.with_bdc(conv);
        }
        if let Some(cal_id) = intern_calendar_id_opt(slf.base_calendar_id.as_deref()) {
            inst = inst.with_base_calendar_id(cal_id);
        }
        if let Some(cal_id) = intern_calendar_id_opt(slf.quote_calendar_id.as_deref()) {
            inst = inst.with_quote_calendar_id(cal_id);
        }
        Ok(PyFxSpot::new(inst))
    }

    fn __repr__(&self) -> String {
        "FxSpotBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyFxSpot {
    #[classmethod]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyFxSpotBuilder>> {
        let py = cls.py();
        let builder = PyFxSpotBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Base currency (FX numerator).
    ///
    /// Returns:
    ///     Currency: Base currency wrapper.
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency (FX denominator).
    ///
    /// Returns:
    ///     Currency: Quote currency wrapper.
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Notional in base currency.
    ///
    /// Returns:
    ///     Money: Base notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Explicit spot rate if provided.
    ///
    /// Returns:
    ///     float | None: Spot rate override.
    #[getter]
    fn spot_rate(&self) -> Option<f64> {
        self.inner.spot_rate
    }

    /// Settlement date if provided.
    ///
    /// Returns:
    ///     datetime.date | None: Explicit settlement date.
    #[getter]
    fn settlement(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        Ok(match self.inner.settlement {
            Some(date) => Some(date_to_py(py, date)?),
            None => None,
        })
    }

    /// Settlement lag in business days when settlement date is inferred.
    ///
    /// Returns:
    ///     int | None: Settlement lag applied if settlement date omitted.
    #[getter]
    fn settlement_lag_days(&self) -> Option<i32> {
        self.inner.settlement_lag_days
    }

    /// Business-day convention used when adjusting settlement.
    ///
    /// Returns:
    ///     str: Business-day convention label.
    #[getter]
    fn business_day_convention(&self) -> &'static str {
        match self.inner.bdc {
            finstack_core::dates::BusinessDayConvention::Following => "following",
            finstack_core::dates::BusinessDayConvention::ModifiedFollowing => "modified_following",
            finstack_core::dates::BusinessDayConvention::Preceding => "preceding",
            finstack_core::dates::BusinessDayConvention::ModifiedPreceding => "modified_preceding",
            finstack_core::dates::BusinessDayConvention::Unadjusted => "unadjusted",
            _ => "modified_following",
        }
    }

    /// Base currency calendar identifier (if set).
    ///
    /// Returns:
    ///     str | None: Base currency calendar used for settlement adjustments.
    #[getter]
    fn base_calendar(&self) -> Option<&str> {
        self.inner.base_calendar_id.as_deref()
    }

    /// Quote currency calendar identifier (if set).
    ///
    /// Returns:
    ///     str | None: Quote currency calendar used for settlement adjustments.
    #[getter]
    fn quote_calendar(&self) -> Option<&str> {
        self.inner.quote_calendar_id.as_deref()
    }

    /// FX pair mnemonic such as ``"EURUSD"``.
    ///
    /// Returns:
    ///     str: Concatenated currency pair name.
    #[getter]
    fn pair_name(&self) -> String {
        self.inner.pair_name()
    }

    /// Instrument type enum (``InstrumentType.FX_SPOT``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.FX_SPOT``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxSpot)
    }

    /// Calculate present value of the FX spot.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including FX rates
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// Money
    ///     Present value in quote currency
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

    /// Compute effective settlement date for the FX spot.
    ///
    /// Parameters
    /// ----------
    /// as_of : Date
    ///     Trade/valuation date
    ///
    /// Returns
    /// -------
    /// Date
    ///     Effective settlement date adjusted for business days
    fn effective_settlement_date(
        &self,
        py: Python<'_>,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let date = py_to_date(&as_of)?;
        let settle = self
            .inner
            .effective_settlement_date(date)
            .map_err(core_to_py)?;
        date_to_py(py, settle)
    }

    /// Check if this is a T+1 settlement pair (e.g. USD/CAD, USD/TRY).
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if the pair conventionally settles T+1
    fn is_t1_pair(&self) -> bool {
        self.inner.is_t1_pair()
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
///
/// Examples:
///     >>> option = FxOption.builder("eurusd_call").base_currency("EUR").quote_currency("USD").strike(1.1).expiry(date(2024, 12, 20)).notional(Money("EUR", 1_000_000)).build()
///     >>> option.option_type
///     'call'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxOption {
    pub(crate) inner: Arc<FxOption>,
}

impl PyFxOption {
    pub(crate) fn new(inner: FxOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(module = "finstack.valuations.instruments", name = "FxOptionBuilder")]
pub struct PyFxOptionBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    expiry: Option<time::Date>,
    notional: Option<finstack_core::money::Money>,
    domestic_curve: Option<CurveId>,
    foreign_curve: Option<CurveId>,
    vol_surface: Option<CurveId>,
    option_type: OptionType,
    exercise_style: ExerciseStyle,
    settlement: SettlementType,
    day_count: finstack_core::dates::DayCount,
}

impl PyFxOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            strike: None,
            expiry: None,
            notional: None,
            domestic_curve: None,
            foreign_curve: None,
            vol_surface: None,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            day_count: finstack_core::dates::DayCount::Act365F,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.base_currency.is_none() {
            return Err(PyValueError::new_err("base_currency() is required."));
        }
        if self.quote_currency.is_none() {
            return Err(PyValueError::new_err("quote_currency() is required."));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.domestic_curve.is_none() {
            return Err(PyValueError::new_err(
                "domestic_discount_curve() is required.",
            ));
        }
        if self.foreign_curve.is_none() {
            return Err(PyValueError::new_err(
                "foreign_discount_curve() is required.",
            ));
        }
        if self.vol_surface.is_none() {
            return Err(PyValueError::new_err("vol_surface() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyFxOptionBuilder {
    #[new]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        base_currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let CurrencyArg(base) = base_currency.extract().context("base_currency")?;
        slf.base_currency = Some(base);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        quote_currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let CurrencyArg(quote) = quote_currency.extract().context("quote_currency")?;
        slf.quote_currency = Some(quote);
        Ok(slf)
    }

    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    fn domestic_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.domestic_curve = Some(CurveId::new(curve_id));
        slf
    }

    fn foreign_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.foreign_curve = Some(CurveId::new(curve_id));
        slf
    }

    fn vol_surface<'py>(mut slf: PyRefMut<'py, Self>, surface_id: &str) -> PyRefMut<'py, Self> {
        slf.vol_surface = Some(CurveId::new(surface_id));
        slf
    }

    fn exercise_style(
        mut slf: PyRefMut<'_, Self>,
        exercise_style: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.exercise_style = ExerciseStyle::from_str(&exercise_style)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    fn day_count<'py>(mut slf: PyRefMut<'py, Self>, day_count: &PyDayCount) -> PyRefMut<'py, Self> {
        slf.day_count = day_count.inner;
        slf
    }

    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type =
            OptionType::from_str(&option_type).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    fn settlement(mut slf: PyRefMut<'_, Self>, settlement: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.settlement = SettlementType::from_str(&settlement)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxOption> {
        slf.ensure_ready()?;
        let base = slf.base_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxOptionBuilder internal error: missing base_currency after validation",
            )
        })?;
        let quote = slf.quote_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxOptionBuilder internal error: missing quote_currency after validation",
            )
        })?;
        let strike = slf.strike.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxOptionBuilder internal error: missing expiry after validation",
            )
        })?;
        let notional = slf.notional.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxOptionBuilder internal error: missing notional after validation",
            )
        })?;

        let mut builder = FxOption::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.base_currency(base);
        builder = builder.quote_currency(quote);
        builder = builder.strike(strike);
        builder = builder.option_type(slf.option_type);
        builder = builder.exercise_style(slf.exercise_style);
        builder = builder.expiry(expiry);
        builder = builder.day_count(slf.day_count);
        builder = builder.notional(notional);
        builder = builder.settlement(slf.settlement);
        builder =
            builder.domestic_discount_curve_id(slf.domestic_curve.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FxOptionBuilder internal error: missing domestic curve after validation",
                )
            })?);
        builder =
            builder.foreign_discount_curve_id(slf.foreign_curve.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FxOptionBuilder internal error: missing foreign curve after validation",
                )
            })?);
        builder = builder.vol_surface_id(slf.vol_surface.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxOptionBuilder internal error: missing vol surface after validation",
            )
        })?);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.attributes(finstack_valuations::instruments::Attributes::new());
        Ok(PyFxOption::new(builder.build().map_err(core_to_py)?))
    }

    fn __repr__(&self) -> String {
        "FxOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyFxOption {
    #[classmethod]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyFxOptionBuilder>> {
        let py = cls.py();
        let builder = PyFxOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Base currency for the option underlying.
    ///
    /// Returns:
    ///     Currency: Base currency wrapper.
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency for settlement.
    ///
    /// Returns:
    ///     Currency: Quote currency wrapper.
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Notional amount in base currency.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike rate expressed as quote per unit of base.
    ///
    /// Returns:
    ///     float: Strike rate of the option.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Expiry date.
    ///
    /// Returns:
    ///     datetime.date: Expiry converted to Python.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Option type (``"call"`` or ``"put"``).
    ///
    /// Returns:
    ///     str: Option type label.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Exercise style (currently ``"european"`` for simplified constructors).
    ///
    /// Returns:
    ///     str: Exercise style label.
    #[getter]
    fn exercise_style(&self) -> &'static str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
        }
    }

    /// Settlement type (cash vs. physical).
    ///
    /// Returns:
    ///     str: Settlement type label.
    #[getter]
    fn settlement(&self) -> &'static str {
        match self.inner.settlement {
            SettlementType::Cash => "cash",
            SettlementType::Physical => "physical",
        }
    }

    /// Domestic discount curve identifier.
    ///
    /// Returns:
    ///     str: Domestic discount curve used for discounting.
    #[getter]
    fn domestic_discount_curve(&self) -> String {
        self.inner.domestic_discount_curve_id.as_str().to_string()
    }

    /// Foreign discount curve identifier.
    ///
    /// Returns:
    ///     str: Foreign discount curve used for discounting.
    #[getter]
    fn foreign_discount_curve(&self) -> String {
        self.inner.foreign_discount_curve_id.as_str().to_string()
    }

    /// Volatility surface identifier used for pricing.
    ///
    /// Returns:
    ///     str: Volatility surface label.
    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    /// Day count convention.
    ///
    /// Returns:
    ///     DayCount: Day count convention used for time calculations.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Instrument type enum (``InstrumentType.FX_OPTION``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.FX_OPTION``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxOption)
    }

    /// Calculate present value using Garman-Kohlhagen model.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves and FX rates
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// Money
    ///     Present value in quote currency
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    /// Solve for implied volatility given a target price.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves
    /// as_of : Date
    ///     Valuation date
    /// target_price : float
    ///     Target option price to match
    /// initial_guess : float, optional
    ///     Starting volatility guess (default: 0.20)
    ///
    /// Returns
    /// -------
    /// float
    ///     Implied volatility (decimal)
    #[pyo3(signature = (market, as_of, target_price, initial_guess=None))]
    fn implied_vol(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| {
            self.inner
                .implied_vol(&market.inner, date, target_price, initial_guess)
        })
        .map_err(core_to_py)
    }

    /// Calculate the at-the-money forward (ATMF) strike.
    ///
    /// Parameters
    /// ----------
    /// spot : float
    ///     Current spot FX rate
    /// df_domestic : float
    ///     Domestic discount factor to expiry
    /// df_foreign : float
    ///     Foreign discount factor to expiry
    ///
    /// Returns
    /// -------
    /// float
    ///     ATMF strike
    #[staticmethod]
    fn atm_forward_strike(spot: f64, df_domestic: f64, df_foreign: f64) -> f64 {
        FxOption::atm_forward_strike(spot, df_domestic, df_foreign)
    }

    /// Calculate the Delta-Neutral Straddle (DNS) strike.
    ///
    /// Parameters
    /// ----------
    /// forward : float
    ///     Forward FX rate
    /// vol : float
    ///     ATM volatility (decimal)
    /// time_to_expiry : float
    ///     Time to expiry in years
    /// use_forward_delta : bool
    ///     If True, use forward delta convention (interbank standard)
    ///
    /// Returns
    /// -------
    /// float
    ///     DNS strike
    #[staticmethod]
    fn atm_dns_strike(forward: f64, vol: f64, time_to_expiry: f64, use_forward_delta: bool) -> f64 {
        let convention = if use_forward_delta {
            FxAtmDeltaConvention::Forward
        } else {
            FxAtmDeltaConvention::Spot
        };
        FxOption::atm_dns_strike_for_convention(forward, vol, time_to_expiry, convention)
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
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxSwap",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxSwap {
    pub(crate) inner: Arc<FxSwap>,
}

impl PyFxSwap {
    pub(crate) fn new(inner: FxSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(module = "finstack.valuations.instruments", name = "FxSwapBuilder")]
pub struct PyFxSwapBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    notional: Option<finstack_core::money::Money>,
    near_date: Option<time::Date>,
    far_date: Option<time::Date>,
    domestic_curve: Option<CurveId>,
    foreign_curve: Option<CurveId>,
    near_rate: Option<f64>,
    far_rate: Option<f64>,
    base_calendar_id: Option<String>,
    quote_calendar_id: Option<String>,
}

impl PyFxSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            notional: None,
            near_date: None,
            far_date: None,
            domestic_curve: None,
            foreign_curve: None,
            near_rate: None,
            far_rate: None,
            base_calendar_id: None,
            quote_calendar_id: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.base_currency.is_none() {
            return Err(PyValueError::new_err("base_currency() is required."));
        }
        if self.quote_currency.is_none() {
            return Err(PyValueError::new_err("quote_currency() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.near_date.is_none() {
            return Err(PyValueError::new_err("near_date() is required."));
        }
        if self.far_date.is_none() {
            return Err(PyValueError::new_err("far_date() is required."));
        }
        if self.domestic_curve.is_none() {
            return Err(PyValueError::new_err(
                "domestic_discount_curve() is required.",
            ));
        }
        if self.foreign_curve.is_none() {
            return Err(PyValueError::new_err(
                "foreign_discount_curve() is required.",
            ));
        }
        Ok(())
    }
}

#[pymethods]
impl PyFxSwapBuilder {
    #[new]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        base_currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let CurrencyArg(base) = base_currency.extract().context("base_currency")?;
        slf.base_currency = Some(base);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        quote_currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let CurrencyArg(quote) = quote_currency.extract().context("quote_currency")?;
        slf.quote_currency = Some(quote);
        Ok(slf)
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    fn near_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        near_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.near_date = Some(py_to_date(&near_date).context("near_date")?);
        Ok(slf)
    }

    fn far_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        far_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.far_date = Some(py_to_date(&far_date).context("far_date")?);
        Ok(slf)
    }

    fn domestic_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.domestic_curve = Some(CurveId::new(curve_id));
        slf
    }

    fn foreign_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.foreign_curve = Some(CurveId::new(curve_id));
        slf
    }

    fn base_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.base_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn quote_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.quote_calendar_id = Some(calendar_id.to_string());
        slf
    }

    #[pyo3(text_signature = "($self, near_rate=None)", signature = (near_rate=None))]
    fn near_rate(mut slf: PyRefMut<'_, Self>, near_rate: Option<f64>) -> PyRefMut<'_, Self> {
        slf.near_rate = near_rate;
        slf
    }

    #[pyo3(text_signature = "($self, far_rate=None)", signature = (far_rate=None))]
    fn far_rate(mut slf: PyRefMut<'_, Self>, far_rate: Option<f64>) -> PyRefMut<'_, Self> {
        slf.far_rate = far_rate;
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxSwap> {
        slf.ensure_ready()?;
        let base = slf.base_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSwapBuilder internal error: missing base_currency after validation",
            )
        })?;
        let quote = slf.quote_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSwapBuilder internal error: missing quote_currency after validation",
            )
        })?;
        let base_notional = slf.notional.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSwapBuilder internal error: missing notional after validation",
            )
        })?;
        let near = slf.near_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSwapBuilder internal error: missing near_date after validation",
            )
        })?;
        let far = slf.far_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "FxSwapBuilder internal error: missing far_date after validation",
            )
        })?;

        let mut builder = FxSwap::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.base_currency(base);
        builder = builder.quote_currency(quote);
        builder = builder.near_date(near);
        builder = builder.far_date(far);
        builder = builder.base_notional(base_notional);
        builder =
            builder.domestic_discount_curve_id(slf.domestic_curve.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FxSwapBuilder internal error: missing domestic curve after validation",
                )
            })?);
        builder =
            builder.foreign_discount_curve_id(slf.foreign_curve.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "FxSwapBuilder internal error: missing foreign curve after validation",
                )
            })?);
        if let Some(rate) = slf.near_rate {
            builder = builder.near_rate(rate);
        }
        if let Some(rate) = slf.far_rate {
            builder = builder.far_rate(rate);
        }
        if let Some(ref cal) = slf.base_calendar_id {
            builder = builder.base_calendar_id_opt(Some(cal.clone()));
        }
        if let Some(ref cal) = slf.quote_calendar_id {
            builder = builder.quote_calendar_id_opt(Some(cal.clone()));
        }

        let swap = builder.build().map_err(core_to_py)?;
        Ok(PyFxSwap::new(swap))
    }

    fn __repr__(&self) -> String {
        "FxSwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyFxSwap {
    #[classmethod]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyFxSwapBuilder>> {
        let py = cls.py();
        let builder = PyFxSwapBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Base currency exchanged on the swap.
    ///
    /// Returns:
    ///     Any: Base currency exchanged on the swap.
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency exchanged on the swap.
    ///
    /// Returns:
    ///     Any: Quote currency exchanged on the swap.
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Base notional in the base currency.
    ///
    /// Returns:
    ///     Any: Base notional in the base currency.
    #[getter]
    fn base_notional(&self) -> PyMoney {
        PyMoney::new(self.inner.base_notional)
    }

    /// Near leg settlement date.
    ///
    /// Returns:
    ///     Any: Near leg settlement date.
    #[getter]
    fn near_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.near_date)
    }

    /// Far leg settlement date.
    ///
    /// Returns:
    ///     Any: Far leg settlement date.
    #[getter]
    fn far_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.far_date)
    }

    /// Optional contractual near FX rate.
    ///
    /// Returns:
    ///     Any: Optional contractual near FX rate.
    #[getter]
    fn near_rate(&self) -> Option<f64> {
        self.inner.near_rate
    }

    /// Optional contractual far FX rate.
    ///
    /// Returns:
    ///     Any: Optional contractual far FX rate.
    #[getter]
    fn far_rate(&self) -> Option<f64> {
        self.inner.far_rate
    }

    /// Domestic discount curve identifier.
    ///
    /// Returns:
    ///     str: Domestic discount curve identifier.
    #[getter]
    fn domestic_discount_curve(&self) -> String {
        self.inner.domestic_discount_curve_id.as_str().to_string()
    }

    /// Foreign discount curve identifier.
    ///
    /// Returns:
    ///     str: Foreign discount curve identifier.
    #[getter]
    fn foreign_discount_curve(&self) -> String {
        self.inner.foreign_discount_curve_id.as_str().to_string()
    }

    /// Base currency calendar identifier (if set).
    ///
    /// Returns:
    ///     str | None: Base currency calendar identifier.
    #[getter]
    fn base_calendar(&self) -> Option<String> {
        self.inner.base_calendar_id.clone()
    }

    /// Quote currency calendar identifier (if set).
    ///
    /// Returns:
    ///     str | None: Quote currency calendar identifier.
    #[getter]
    fn quote_calendar(&self) -> Option<String> {
        self.inner.quote_calendar_id.clone()
    }

    /// Instrument type enum (``InstrumentType.FX_SWAP``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.FX_SWAP``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxSwap)
    }

    /// Calculate present value of the FX swap.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves and FX rates
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// Money
    ///     Present value in quote currency
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
    module.add_class::<PyFxSpotBuilder>()?;
    module.add_class::<PyFxOptionBuilder>()?;
    module.add_class::<PyFxSwapBuilder>()?;
    Ok(vec![
        "FxSpot",
        "FxOption",
        "FxSwap",
        "FxSpotBuilder",
        "FxOptionBuilder",
        "FxSwapBuilder",
    ])
}
