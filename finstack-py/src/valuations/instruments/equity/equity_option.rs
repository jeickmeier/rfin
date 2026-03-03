use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::{
    Attributes, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
};
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyTuple, PyType};
use pyo3::Bound;
use std::fmt;
use std::sync::Arc;

/// Equity option priced via Black–Scholes style models.
///
/// Examples:
///     >>> option = EquityOption.builder("opt_aapl_jan").ticker("AAPL").strike(180.0).expiry(date(2024, 1, 19)).notional(Money.from_code(1.0, "USD")).option_type("call").exercise_style("european").disc_id("USD-OIS").spot_id("AAPL").vol_surface("AAPL-VOL").build()
///     >>> option.option_type
///     'call'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEquityOption {
    pub(crate) inner: Arc<EquityOption>,
}

impl PyEquityOption {
    pub(crate) fn new(inner: EquityOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

/// Greeks for an equity option (delta, gamma, vega, theta, rho).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityOptionGreeks",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEquityOptionGreeks {
    pub(crate) delta_val: f64,
    pub(crate) gamma_val: f64,
    pub(crate) vega_val: f64,
    pub(crate) theta_val: f64,
    pub(crate) rho_val: f64,
}

#[pymethods]
impl PyEquityOptionGreeks {
    #[getter]
    fn delta(&self) -> f64 {
        self.delta_val
    }

    #[getter]
    fn gamma(&self) -> f64 {
        self.gamma_val
    }

    #[getter]
    fn vega(&self) -> f64 {
        self.vega_val
    }

    #[getter]
    fn theta(&self) -> f64 {
        self.theta_val
    }

    #[getter]
    fn rho(&self) -> f64 {
        self.rho_val
    }

    fn __repr__(&self) -> String {
        format!(
            "EquityOptionGreeks(delta={:.6}, gamma={:.6}, vega={:.6}, theta={:.6}, rho={:.6})",
            self.delta_val, self.gamma_val, self.vega_val, self.theta_val, self.rho_val,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityOptionBuilder",
    unsendable
)]
pub struct PyEquityOptionBuilder {
    instrument_id: InstrumentId,
    ticker: Option<String>,
    strike: Option<f64>,
    option_type: OptionType,
    exercise_style: ExerciseStyle,
    expiry: Option<time::Date>,
    notional: Option<Money>,
    day_count: DayCount,
    settlement: SettlementType,
    discount_curve_id: Option<CurveId>,
    spot_id: Option<String>,
    vol_surface_id: Option<CurveId>,
    div_yield_id: Option<String>,
    discrete_dividends: Vec<(time::Date, f64)>,
    exercise_schedule: Option<Vec<time::Date>>,
}

impl PyEquityOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            ticker: None,
            strike: None,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            expiry: None,
            notional: None,
            day_count: DayCount::Act365F,
            settlement: SettlementType::Cash,
            discount_curve_id: None,
            spot_id: None,
            vol_surface_id: None,
            div_yield_id: None,
            discrete_dividends: Vec::new(),
            exercise_schedule: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.ticker.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("ticker() is required."));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("disc_id() is required."));
        }
        if self.spot_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("spot_id() is required."));
        }
        if self.vol_surface_id.is_none() {
            return Err(PyValueError::new_err("vol_surface() is required."));
        }
        Ok(())
    }

    fn parse_day_count(value: &Bound<'_, PyAny>) -> PyResult<DayCount> {
        if let Ok(py_dc) = value.extract::<PyRef<PyDayCount>>() {
            return Ok(py_dc.inner);
        }
        if let Ok(name) = value.extract::<&str>() {
            return match name.to_lowercase().as_str() {
                "act_360" | "act/360" => Ok(DayCount::Act360),
                "act_365f" | "act/365f" | "act365f" => Ok(DayCount::Act365F),
                "act_act" | "act/act" | "actact" => Ok(DayCount::ActAct),
                "thirty_360" | "30/360" | "30e/360" => Ok(DayCount::Thirty360),
                other => Err(PyValueError::new_err(format!(
                    "Unsupported day count '{other}'"
                ))),
            };
        }
        Err(PyTypeError::new_err("day_count() expects DayCount or str"))
    }

    fn parse_option_type(value: &str) -> PyResult<OptionType> {
        match value.to_lowercase().as_str() {
            "call" => Ok(OptionType::Call),
            "put" => Ok(OptionType::Put),
            other => Err(PyValueError::new_err(format!(
                "option_type must be 'call' or 'put' (got '{other}')"
            ))),
        }
    }

    fn parse_exercise_style(value: &str) -> PyResult<ExerciseStyle> {
        match value.to_lowercase().as_str() {
            "european" => Ok(ExerciseStyle::European),
            "american" => Ok(ExerciseStyle::American),
            "bermudan" => Ok(ExerciseStyle::Bermudan),
            other => Err(PyValueError::new_err(format!(
                "exercise_style must be 'european', 'american', or 'bermudan' (got '{other}')"
            ))),
        }
    }

    fn parse_settlement(value: &str) -> PyResult<SettlementType> {
        match value.to_lowercase().as_str() {
            "cash" => Ok(SettlementType::Cash),
            "physical" => Ok(SettlementType::Physical),
            other => Err(PyValueError::new_err(format!(
                "settlement must be 'cash' or 'physical' (got '{other}')"
            ))),
        }
    }
}

#[pymethods]
impl PyEquityOptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, ticker)")]
    fn ticker(mut slf: PyRefMut<'_, Self>, ticker: String) -> PyRefMut<'_, Self> {
        slf.ticker = Some(ticker);
        slf
    }

    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyResult<PyRefMut<'_, Self>> {
        if strike <= 0.0 {
            return Err(PyValueError::new_err("strike must be positive"));
        }
        slf.strike = Some(strike);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type = Self::parse_option_type(&option_type)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, exercise_style)")]
    fn exercise_style(
        mut slf: PyRefMut<'_, Self>,
        exercise_style: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.exercise_style = Self::parse_exercise_style(&exercise_style)?;
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

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.day_count = Self::parse_day_count(&day_count)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, settlement)")]
    fn settlement(mut slf: PyRefMut<'_, Self>, settlement: String) -> PyResult<PyRefMut<'_, Self>> {
        slf.settlement = Self::parse_settlement(&settlement)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, spot_id)")]
    fn spot_id(mut slf: PyRefMut<'_, Self>, spot_id: String) -> PyRefMut<'_, Self> {
        slf.spot_id = Some(spot_id);
        slf
    }

    #[pyo3(text_signature = "($self, vol_surface)")]
    fn vol_surface(mut slf: PyRefMut<'_, Self>, vol_surface: String) -> PyRefMut<'_, Self> {
        slf.vol_surface_id = Some(CurveId::new(vol_surface.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, div_yield_id=None)", signature = (div_yield_id=None))]
    fn div_yield_id(
        mut slf: PyRefMut<'_, Self>,
        div_yield_id: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.div_yield_id = div_yield_id;
        slf
    }

    #[pyo3(text_signature = "($self, ex_date, amount)")]
    fn add_discrete_dividend<'py>(
        mut slf: PyRefMut<'py, Self>,
        ex_date: Bound<'py, PyAny>,
        amount: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = py_to_date(&ex_date).context("ex_date")?;
        slf.discrete_dividends.push((date, amount));
        Ok(slf)
    }

    /// Set exercise schedule for Bermudan options.
    #[pyo3(text_signature = "($self, dates)")]
    fn exercise_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        dates: Vec<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let schedule: Vec<time::Date> = dates
            .iter()
            .map(|d| py_to_date(d).context("exercise_schedule date"))
            .collect::<Result<_, _>>()?;
        slf.exercise_schedule = Some(schedule);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyEquityOption> {
        slf.ensure_ready()?;

        let ticker = slf.ticker.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "EquityOptionBuilder internal error: missing ticker after validation",
            )
        })?;
        let strike = slf.strike.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "EquityOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "EquityOptionBuilder internal error: missing expiry after validation",
            )
        })?;
        let discount = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "EquityOptionBuilder internal error: missing discount curve after validation",
            )
        })?;
        let spot_id = slf.spot_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "EquityOptionBuilder internal error: missing spot_id after validation",
            )
        })?;
        let vol_surface = slf.vol_surface_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "EquityOptionBuilder internal error: missing vol surface after validation",
            )
        })?;

        EquityOption::builder()
            .id(slf.instrument_id.clone())
            .underlying_ticker(ticker)
            .strike(strike)
            .option_type(slf.option_type)
            .exercise_style(slf.exercise_style)
            .expiry(expiry)
            .notional(
                slf.notional
                    .unwrap_or(Money::new(1.0, finstack_core::currency::Currency::USD)),
            )
            .day_count(slf.day_count)
            .settlement(slf.settlement)
            .discount_curve_id(discount)
            .spot_id(spot_id.into())
            .vol_surface_id(vol_surface)
            .div_yield_id_opt(slf.div_yield_id.clone().map(CurveId::new))
            .discrete_dividends(slf.discrete_dividends.clone())
            .exercise_schedule_opt(slf.exercise_schedule.clone())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map(PyEquityOption::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "EquityOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyEquityOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyEquityOptionBuilder>> {
        let py = cls.py();
        let builder = PyEquityOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
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

    /// Underlying ticker symbol.
    ///
    /// Returns:
    ///     str: Ticker for the underlying equity.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    /// Strike price as scalar.
    ///
    /// Returns:
    ///     float: Strike price in underlying price units.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Position notional.
    ///
    /// Returns:
    ///     Money: Notional amount for the option position.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Option type label (``"call"``/``"put"``).
    ///
    /// Returns:
    ///     str: ``"call"`` or ``"put"`` depending on option direction.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Exercise style label.
    ///
    /// Returns:
    ///     str: Exercise style such as ``"european"``.
    #[getter]
    fn exercise_style(&self) -> &'static str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
        }
    }

    /// Expiry date of the option.
    ///
    /// Returns:
    ///     datetime.date: Expiry date in calendar form.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Volatility surface identifier.
    ///
    /// Returns:
    ///     str: Volatility surface identifier used for pricing.
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

    /// Settlement type (``"cash"`` or ``"physical"``).
    ///
    /// Returns:
    ///     str: Settlement method for the option contract.
    #[getter]
    fn settlement(&self) -> &'static str {
        match self.inner.settlement {
            SettlementType::Cash => "cash",
            SettlementType::Physical => "physical",
        }
    }

    /// Spot price identifier.
    ///
    /// Returns:
    ///     str: Market data key for spot price lookup.
    #[getter]
    fn spot_id(&self) -> &str {
        self.inner.spot_id.as_str()
    }

    /// Dividend yield identifier, if provided.
    ///
    /// Returns:
    ///     str | None: Market data key for dividend yield, or None.
    #[getter]
    fn div_yield_id(&self) -> Option<String> {
        self.inner
            .div_yield_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    /// Discrete dividend schedule as list of ``(date, amount)`` tuples.
    ///
    /// Returns:
    ///     list[tuple[date, float]]: Dividend ex-dates and amounts.
    #[getter]
    fn discrete_dividends(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let items: PyResult<Vec<Py<PyAny>>> = self
            .inner
            .discrete_dividends
            .iter()
            .map(|(date, amount)| {
                let py_date = date_to_py(py, *date)?;
                Ok(
                    PyTuple::new(py, [py_date, amount.into_pyobject(py)?.into_any().unbind()])?
                        .into(),
                )
            })
            .collect();
        Ok(pyo3::types::PyList::new(py, items?)?.into())
    }

    /// Instrument type enum (``InstrumentType.EQUITY_OPTION``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::EquityOption)
    }

    #[pyo3(signature = (market, as_of))]
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

    #[pyo3(signature = (market, as_of))]
    fn greeks(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyEquityOptionGreeks> {
        let date = py_to_date(&as_of)?;
        let g = py
            .detach(|| self.inner.greeks(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyEquityOptionGreeks {
            delta_val: g.delta,
            gamma_val: g.gamma,
            vega_val: g.vega,
            theta_val: g.theta,
            rho_val: g.rho,
        })
    }

    #[pyo3(signature = (market, as_of))]
    fn delta(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.delta(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn gamma(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.gamma(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn vega(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.vega(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn theta(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.theta(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn rho(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.rho(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of, market_price))]
    fn implied_vol(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        market_price: f64,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.implied_vol(&market.inner, date, market_price))
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "EquityOption(id='{}', ticker='{}', type='{}')",
            self.inner.id,
            self.inner.underlying_ticker,
            self.option_type()
        ))
    }
}

impl fmt::Display for PyEquityOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EquityOption({}, ticker={}, type={})",
            self.inner.id,
            self.inner.underlying_ticker,
            self.option_type()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyEquityOption>()?;
    module.add_class::<PyEquityOptionBuilder>()?;
    module.add_class::<PyEquityOptionGreeks>()?;
    Ok(vec![
        "EquityOption",
        "EquityOptionBuilder",
        "EquityOptionGreeks",
    ])
}
