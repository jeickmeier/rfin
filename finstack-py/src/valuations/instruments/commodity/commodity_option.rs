//! Python bindings for CommodityOption instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_option::CommodityOption;
use finstack_valuations::instruments::common::parameters::CommodityConvention;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use finstack_valuations::instruments::{
    Attributes, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Commodity option (call or put on commodity forward/spot).
///
/// Represents an option to buy (call) or sell (put) a commodity at a specified
/// strike price on or before expiry. Supports European and American exercise.
///
/// Pricing uses Black-76 for European exercise and binomial tree for American.
///
/// Examples:
///     >>> option = (
///     ...     CommodityOption.builder("WTI-CALL-75-2025M06")
///     ...     .commodity_type("Energy")
///     ...     .ticker("CL")
///     ...     .strike(75.0)
///     ...     .option_type("call")
///     ...     .exercise_style("european")
///     ...     .expiry(Date(2025, 6, 15))
///     ...     .quantity(1000.0)
///     ...     .unit("BBL")
///     ...     .currency("USD")
///     ...     .forward_curve_id("WTI-FORWARD")
///     ...     .discount_curve_id("USD-OIS")
///     ...     .vol_surface_id("WTI-VOL")
///     ...     .build()
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommodityOption {
    pub(crate) inner: Arc<CommodityOption>,
}

impl PyCommodityOption {
    pub(crate) fn new(inner: CommodityOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityOptionBuilder"
)]
pub struct PyCommodityOptionBuilder {
    instrument_id: InstrumentId,
    commodity_type: Option<String>,
    ticker: Option<String>,
    strike: Option<f64>,
    option_type: OptionType,
    exercise_style: ExerciseStyle,
    expiry: Option<time::Date>,
    quantity: Option<f64>,
    unit: Option<String>,
    currency: Option<finstack_core::currency::Currency>,
    forward_curve_id: Option<CurveId>,
    discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    multiplier: f64,
    settlement: SettlementType,
    day_count: DayCount,
    spot_id: Option<String>,
    quoted_forward: Option<f64>,
    implied_volatility: Option<f64>,
    tree_steps: Option<usize>,
    convention: Option<CommodityConvention>,
    premium_settlement_days: Option<u32>,
}

impl PyCommodityOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            commodity_type: None,
            ticker: None,
            strike: None,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            expiry: None,
            quantity: None,
            unit: None,
            currency: None,
            forward_curve_id: None,
            discount_curve_id: None,
            vol_surface_id: None,
            multiplier: 1.0,
            settlement: SettlementType::Cash,
            day_count: DayCount::Act365F,
            spot_id: None,
            quoted_forward: None,
            implied_volatility: None,
            tree_steps: None,
            convention: None,
            premium_settlement_days: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.commodity_type.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("commodity_type() is required."));
        }
        if self.ticker.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("ticker() is required."));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.quantity.is_none() {
            return Err(PyValueError::new_err("quantity() is required."));
        }
        if self.unit.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("unit() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
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

    fn parse_day_count(dc: Bound<'_, PyAny>) -> PyResult<DayCount> {
        if let Ok(py_dc) = dc.extract::<pyo3::PyRef<PyDayCount>>() {
            return Ok(py_dc.inner);
        }
        if let Ok(name) = dc.extract::<&str>() {
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
        Err(PyTypeError::new_err("day_count expects DayCount or str"))
    }
}

#[pymethods]
impl PyCommodityOptionBuilder {
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

    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type =
            OptionType::from_str(&option_type).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, exercise_style)")]
    fn exercise_style(
        mut slf: PyRefMut<'_, Self>,
        exercise_style: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.exercise_style = ExerciseStyle::from_str(&exercise_style)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
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

    #[pyo3(text_signature = "($self, quantity)")]
    fn quantity(mut slf: PyRefMut<'_, Self>, quantity: f64) -> PyResult<PyRefMut<'_, Self>> {
        if quantity <= 0.0 {
            return Err(PyValueError::new_err("quantity must be positive"));
        }
        slf.quantity = Some(quantity);
        Ok(slf)
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

    #[pyo3(text_signature = "($self, multiplier)")]
    fn multiplier(mut slf: PyRefMut<'_, Self>, multiplier: f64) -> PyRefMut<'_, Self> {
        slf.multiplier = multiplier;
        slf
    }

    #[pyo3(text_signature = "($self, settlement_type)")]
    fn settlement_type(
        mut slf: PyRefMut<'_, Self>,
        settlement_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.settlement = SettlementType::from_str(&settlement_type)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.day_count = Self::parse_day_count(day_count)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, spot_id=None)", signature = (spot_id=None))]
    fn spot_id(mut slf: PyRefMut<'_, Self>, spot_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.spot_id = spot_id;
        slf
    }

    #[pyo3(text_signature = "($self, quoted_forward=None)", signature = (quoted_forward=None))]
    fn quoted_forward(
        mut slf: PyRefMut<'_, Self>,
        quoted_forward: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.quoted_forward = quoted_forward;
        slf
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

    #[pyo3(text_signature = "($self, tree_steps=None)", signature = (tree_steps=None))]
    fn tree_steps(mut slf: PyRefMut<'_, Self>, tree_steps: Option<usize>) -> PyRefMut<'_, Self> {
        slf.tree_steps = tree_steps;
        slf
    }

    #[pyo3(text_signature = "($self, convention=None)", signature = (convention=None))]
    fn convention(
        mut slf: PyRefMut<'_, Self>,
        convention: Option<String>,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.convention = match convention {
            Some(s) => Some(
                CommodityConvention::from_str(&s)
                    .map_err(|e| PyValueError::new_err(format!("Invalid convention: {e}")))?,
            ),
            None => None,
        };
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, days=None)", signature = (days=None))]
    fn premium_settlement_days(
        mut slf: PyRefMut<'_, Self>,
        days: Option<u32>,
    ) -> PyRefMut<'_, Self> {
        slf.premium_settlement_days = days;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyCommodityOption> {
        slf.ensure_ready()?;
        let commodity_type = slf.commodity_type.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing commodity_type after validation",
            )
        })?;
        let ticker = slf.ticker.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing ticker after validation",
            )
        })?;
        let strike = slf.strike.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing expiry after validation",
            )
        })?;
        let quantity = slf.quantity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing quantity after validation",
            )
        })?;
        let unit = slf.unit.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing unit after validation",
            )
        })?;
        let currency = slf.currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing currency after validation",
            )
        })?;
        let forward_curve_id = slf.forward_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing forward_curve_id after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing discount_curve_id after validation",
            )
        })?;
        let vol_surface_id = slf.vol_surface_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "CommodityOptionBuilder internal error: missing vol_surface_id after validation",
            )
        })?;

        let mut pricing_overrides = PricingOverrides::default();
        if let Some(vol) = slf.implied_volatility {
            pricing_overrides.market_quotes.implied_volatility = Some(vol);
        }
        if let Some(steps) = slf.tree_steps {
            pricing_overrides.model_config.tree_steps = Some(steps);
        }

        let mut builder = CommodityOption::builder()
            .id(slf.instrument_id.clone())
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency,
            ))
            .strike(strike)
            .option_type(slf.option_type)
            .exercise_style(slf.exercise_style)
            .expiry(expiry)
            .quantity(quantity)
            .multiplier(slf.multiplier)
            .settlement(slf.settlement)
            .forward_curve_id(forward_curve_id)
            .discount_curve_id(discount_curve_id)
            .vol_surface_id(vol_surface_id)
            .day_count(slf.day_count)
            .pricing_overrides(pricing_overrides)
            .attributes(Attributes::new());

        if let Some(sp_id) = slf.spot_id.clone() {
            builder = builder.spot_id_opt(Some(sp_id));
        }
        if let Some(qf) = slf.quoted_forward {
            builder = builder.quoted_forward_opt(Some(qf));
        }
        if let Some(conv) = slf.convention {
            builder = builder.convention_opt(Some(conv));
        }
        if let Some(days) = slf.premium_settlement_days {
            builder = builder.premium_settlement_days_opt(Some(days));
        }

        let option = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{e}")))?;

        Ok(PyCommodityOption::new(option))
    }

    fn __repr__(&self) -> String {
        "CommodityOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyCommodityOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyCommodityOptionBuilder>> {
        let py = cls.py();
        let builder = PyCommodityOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Commodity type (e.g., "Energy", "Metal").
    #[getter]
    fn commodity_type(&self) -> &str {
        &self.inner.underlying.commodity_type
    }

    /// Ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying.ticker
    }

    /// Strike price.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Option type (call or put).
    #[getter]
    fn option_type(&self) -> &str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Exercise style (european or american).
    #[getter]
    fn exercise_style(&self) -> &str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
        }
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Contract quantity.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.underlying.unit
    }

    /// Contract multiplier.
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Settlement type (physical or cash).
    #[getter]
    fn settlement_type(&self) -> &str {
        match self.inner.settlement {
            SettlementType::Physical => "physical",
            SettlementType::Cash => "cash",
        }
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.underlying.currency)
    }

    /// Forward curve ID.
    #[getter]
    fn forward_curve_id(&self) -> &str {
        self.inner.forward_curve_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Volatility surface ID.
    #[getter]
    fn vol_surface_id(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    /// Optional spot price ID.
    #[getter]
    fn spot_id(&self) -> Option<&str> {
        self.inner.spot_id.as_deref()
    }

    /// Optional quoted forward price.
    #[getter]
    fn quoted_forward(&self) -> Option<f64> {
        self.inner.quoted_forward
    }

    /// Day count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Commodity convention (e.g., "wti", "brent", "gold"). None if not set.
    #[getter]
    fn convention(&self) -> Option<&str> {
        self.inner.convention.map(|c| match c {
            CommodityConvention::WTICrude => "wti",
            CommodityConvention::BrentCrude => "brent",
            CommodityConvention::NaturalGas => "naturalgas",
            CommodityConvention::Gold => "gold",
            CommodityConvention::Silver => "silver",
            CommodityConvention::Copper => "copper",
            CommodityConvention::Agricultural => "agricultural",
            CommodityConvention::Power => "power",
            _ => "unknown",
        })
    }

    /// Premium settlement lag in business days, if explicitly set.
    #[getter]
    fn premium_settlement_days(&self) -> Option<u32> {
        self.inner.premium_settlement_days
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    /// Effective premium settlement days (resolves convention defaults).
    fn effective_premium_settlement_days(&self) -> u32 {
        self.inner.effective_premium_settlement_days()
    }

    fn __repr__(&self) -> String {
        format!(
            "CommodityOption(id='{}', ticker='{}', strike={}, type='{}', exercise='{}', expiry='{}')",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.strike,
            self.option_type(),
            self.exercise_style(),
            self.inner.expiry
        )
    }
}

impl fmt::Display for PyCommodityOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommodityOption({}, {}, {} {} @ {})",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.option_type(),
            self.exercise_style(),
            self.inner.strike
        )
    }
}

/// Export module items for registration.
pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommodityOption>()?;
    parent.add_class::<PyCommodityOptionBuilder>()?;
    Ok(())
}
