use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::fx::PyFxConversionPolicy;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::PyContext;
use crate::valuations::common::PyInstrumentType;
use finstack_core::money::fx::FxConversionPolicy;
use finstack_valuations::instruments::exotics::basket::{
    AssetType, Basket, BasketCalculator, BasketConstituent, BasketPricingConfig,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use std::fmt;
use std::sync::Arc;

fn parse_json(value: &Bound<'_, PyAny>) -> PyResult<Basket> {
    if let Ok(json_str) = value.extract::<&str>() {
        return serde_json::from_str(json_str)
            .map_err(|err| PyValueError::new_err(err.to_string()));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let py = dict.py();
        let json = pyo3::types::PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()?;
        return serde_json::from_str(&json).map_err(|err| PyValueError::new_err(err.to_string()));
    }
    Err(PyTypeError::new_err(
        "Expected JSON string or dict convertible to JSON",
    ))
}

/// Asset type classification for basket constituents.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AssetType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAssetType {
    pub(crate) inner: AssetType,
}

impl PyAssetType {
    pub(crate) fn new(inner: AssetType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            AssetType::Equity => "equity",
            AssetType::Bond => "bond",
            AssetType::ETF => "etf",
            AssetType::Cash => "cash",
            AssetType::Commodity => "commodity",
            AssetType::Derivative => "derivative",
        }
    }
}

#[pymethods]
impl PyAssetType {
    #[classattr]
    #[allow(non_snake_case)]
    fn EQUITY() -> Self {
        Self::new(AssetType::Equity)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn BOND() -> Self {
        Self::new(AssetType::Bond)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn ETF() -> Self {
        Self::new(AssetType::ETF)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn CASH() -> Self {
        Self::new(AssetType::Cash)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn COMMODITY() -> Self {
        Self::new(AssetType::Commodity)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn DERIVATIVE() -> Self {
        Self::new(AssetType::Derivative)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse an asset type from a string label.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match name.to_ascii_lowercase().as_str() {
            "equity" => Ok(Self::new(AssetType::Equity)),
            "bond" => Ok(Self::new(AssetType::Bond)),
            "etf" => Ok(Self::new(AssetType::ETF)),
            "cash" => Ok(Self::new(AssetType::Cash)),
            "commodity" => Ok(Self::new(AssetType::Commodity)),
            "derivative" => Ok(Self::new(AssetType::Derivative)),
            other => Err(PyValueError::new_err(format!(
                "Unknown asset type: {other}"
            ))),
        }
    }

    #[getter]
    /// Snake-case label for this asset type.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("AssetType('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Pricing configuration for basket instruments.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasketPricingConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBasketPricingConfig {
    pub(crate) inner: BasketPricingConfig,
}

impl PyBasketPricingConfig {
    pub(crate) fn new(inner: BasketPricingConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBasketPricingConfig {
    #[new]
    #[pyo3(text_signature = "(days_in_year=365.25, fx_policy='cashflow_date')")]
    /// Create a basket pricing configuration.
    ///
    /// Args:
    ///     days_in_year: Day basis for fee accrual (default: 365.25).
    ///     fx_policy: FX conversion policy label or instance (default: ``"cashflow_date"``).
    ///
    /// Returns:
    ///     BasketPricingConfig: Pricing configuration instance.
    fn ctor(days_in_year: Option<f64>, fx_policy: Option<Bound<'_, PyAny>>) -> PyResult<Self> {
        let days = days_in_year.unwrap_or(365.25);
        let policy = match fx_policy {
            Some(obj) => {
                if let Ok(p) = obj.extract::<PyFxConversionPolicy>() {
                    p.inner
                } else if let Ok(s) = obj.extract::<&str>() {
                    match s.to_ascii_lowercase().as_str() {
                        "cashflow_date" | "cashflow" => FxConversionPolicy::CashflowDate,
                        "period_end" | "end" => FxConversionPolicy::PeriodEnd,
                        "period_average" | "average" => FxConversionPolicy::PeriodAverage,
                        "custom" => FxConversionPolicy::Custom,
                        other => {
                            return Err(PyValueError::new_err(format!(
                                "Unknown FX conversion policy: {other}"
                            )))
                        }
                    }
                } else {
                    return Err(PyTypeError::new_err(
                        "fx_policy must be a string or FxConversionPolicy",
                    ));
                }
            }
            None => FxConversionPolicy::CashflowDate,
        };
        Ok(Self::new(BasketPricingConfig {
            days_in_year: days,
            fx_policy: policy,
        }))
    }

    /// Day basis used for fee accrual.
    ///
    /// Returns:
    ///     float: Days-in-year basis (e.g. 365.25).
    #[getter]
    fn days_in_year(&self) -> f64 {
        self.inner.days_in_year
    }

    /// FX conversion policy.
    ///
    /// Returns:
    ///     FxConversionPolicy: Policy governing FX conversion timing.
    #[getter]
    fn fx_policy(&self) -> PyFxConversionPolicy {
        PyFxConversionPolicy::new(self.inner.fx_policy)
    }

    fn __repr__(&self) -> String {
        let policy_label = match self.inner.fx_policy {
            FxConversionPolicy::CashflowDate => "cashflow_date",
            FxConversionPolicy::PeriodEnd => "period_end",
            FxConversionPolicy::PeriodAverage => "period_average",
            FxConversionPolicy::Custom => "custom",
            _ => "custom",
        };
        format!(
            "BasketPricingConfig(days_in_year={}, fx_policy='{}')",
            self.inner.days_in_year, policy_label
        )
    }
}

/// Read-only view of a basket constituent.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasketConstituent",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyBasketConstituent {
    inner: BasketConstituent,
}

impl PyBasketConstituent {
    fn new(inner: BasketConstituent) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBasketConstituent {
    /// Constituent identifier.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Weight in the basket (as a fraction, e.g. 0.05 = 5%).
    #[getter]
    fn weight(&self) -> f64 {
        self.inner.weight
    }

    /// Number of units for physical replication (if any).
    #[getter]
    fn units(&self) -> Option<f64> {
        self.inner.units
    }

    /// Optional ticker symbol.
    #[getter]
    fn ticker(&self) -> Option<&str> {
        self.inner.ticker.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "BasketConstituent(id='{}', weight={}, ticker={:?})",
            self.inner.id, self.inner.weight, self.inner.ticker
        )
    }
}

/// Basket calculation engine for NAV and value computations.
///
/// Stateless calculator that can be reused across multiple basket valuations.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasketCalculator",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBasketCalculator {
    inner: BasketCalculator,
}

#[pymethods]
impl PyBasketCalculator {
    #[new]
    #[pyo3(text_signature = "(config=None)")]
    /// Create a basket calculator.
    ///
    /// Args:
    ///     config: Optional :class:`BasketPricingConfig`. Uses defaults if omitted.
    ///
    /// Returns:
    ///     BasketCalculator: Calculator instance for basket valuations.
    fn ctor(config: Option<&PyBasketPricingConfig>) -> Self {
        let calc = match config {
            Some(c) => BasketCalculator::new(c.inner.clone()),
            None => BasketCalculator::with_defaults(),
        };
        Self { inner: calc }
    }

    #[pyo3(text_signature = "(self, basket, market_context, as_of, shares_outstanding)")]
    /// Calculate Net Asset Value per share.
    ///
    /// Args:
    ///     basket: The :class:`Basket` instrument to value.
    ///     market_context: :class:`MarketContext` with pricing data.
    ///     as_of: Valuation date.
    ///     shares_outstanding: Total shares outstanding.
    ///
    /// Returns:
    ///     Money: NAV per share.
    fn nav(
        &self,
        basket: &PyBasket,
        market_context: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        shares_outstanding: f64,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of).context("as_of")?;
        self.inner
            .nav(
                &basket.inner,
                &market_context.inner,
                date,
                shares_outstanding,
            )
            .map(PyMoney::new)
            .map_err(crate::errors::core_to_py)
    }

    #[pyo3(text_signature = "(self, basket, market_context, as_of, shares_outstanding=None)")]
    /// Calculate total basket value.
    ///
    /// Args:
    ///     basket: The :class:`Basket` instrument to value.
    ///     market_context: :class:`MarketContext` with pricing data.
    ///     as_of: Valuation date.
    ///     shares_outstanding: Optional total shares outstanding.
    ///
    /// Returns:
    ///     Money: Total basket value.
    fn basket_value(
        &self,
        basket: &PyBasket,
        market_context: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        shares_outstanding: Option<f64>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of).context("as_of")?;
        self.inner
            .basket_value(
                &basket.inner,
                &market_context.inner,
                date,
                shares_outstanding,
            )
            .map(PyMoney::new)
            .map_err(crate::errors::core_to_py)
    }

    #[pyo3(text_signature = "(self, basket, market_context, as_of, aum, shares_outstanding)")]
    /// Calculate NAV per share using explicit assets under management.
    ///
    /// Args:
    ///     basket: The :class:`Basket` instrument to value.
    ///     market_context: :class:`MarketContext` with pricing data.
    ///     as_of: Valuation date.
    ///     aum: Assets under management as :class:`Money`.
    ///     shares_outstanding: Total shares outstanding.
    ///
    /// Returns:
    ///     Money: NAV per share computed from AUM.
    fn nav_with_aum(
        &self,
        basket: &PyBasket,
        market_context: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        aum: Bound<'_, PyAny>,
        shares_outstanding: f64,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of).context("as_of")?;
        let aum_money = extract_money(&aum).context("aum")?;
        self.inner
            .nav_with_aum(
                &basket.inner,
                &market_context.inner,
                date,
                aum_money,
                shares_outstanding,
            )
            .map(PyMoney::new)
            .map_err(crate::errors::core_to_py)
    }

    #[pyo3(text_signature = "(self, basket, market_context, as_of, aum)")]
    /// Calculate total basket value using explicit AUM for weight-based constituents.
    ///
    /// Args:
    ///     basket: The :class:`Basket` instrument to value.
    ///     market_context: :class:`MarketContext` with pricing data.
    ///     as_of: Valuation date.
    ///     aum: Assets under management in basket currency as :class:`Money`.
    ///
    /// Returns:
    ///     Money: Total basket value computed from AUM.
    fn basket_value_with_aum(
        &self,
        basket: &PyBasket,
        market_context: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        aum: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of).context("as_of")?;
        let aum_money = extract_money(&aum).context("aum")?;
        self.inner
            .basket_value_with_aum(&basket.inner, &market_context.inner, date, aum_money)
            .map(PyMoney::new)
            .map_err(crate::errors::core_to_py)
    }

    fn __repr__(&self) -> &'static str {
        "BasketCalculator()"
    }
}

/// Basket instrument wrapper parsed from JSON definitions.
///
/// Examples:
///     >>> basket = Basket.from_json(json.dumps({...}))
///     >>> basket.instrument_type.name
///     'basket'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Basket",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBasket {
    pub(crate) inner: Arc<Basket>,
}

impl PyBasket {
    pub(crate) fn new(inner: Basket) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyBasket {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Parse a basket definition from a JSON string or dictionary.
    ///
    /// Args:
    ///     data: JSON string or dict describing the basket constituents.
    ///
    /// Returns:
    ///     Basket: Parsed basket instrument.
    ///
    /// Raises:
    ///     ValueError: If parsing fails or the basket ID is missing.
    ///     TypeError: If ``data`` is neither a string nor dict-like object.
    fn from_json(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        let basket = parse_json(&data)?;
        if basket.id.as_str().is_empty() {
            return Err(PyValueError::new_err(
                "Basket JSON must include a non-empty 'id' field",
            ));
        }
        Ok(Self::new(basket))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the basket.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.Basket``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Basket)
    }

    /// Base currency of the basket.
    ///
    /// Returns:
    ///     Currency: The basket's denomination currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Position notional used to scale basket NAV.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Total expense ratio as a decimal (e.g. 0.0025 = 0.25%).
    ///
    /// Returns:
    ///     float: Annual expense ratio.
    #[getter]
    fn expense_ratio(&self) -> f64 {
        self.inner.expense_ratio
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve identifier for present value calculations.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Number of constituents in the basket.
    ///
    /// Returns:
    ///     int: Total count of basket constituents.
    #[getter]
    fn constituent_count(&self) -> usize {
        self.inner.constituent_count()
    }

    /// List of basket constituents.
    ///
    /// Returns:
    ///     list[BasketConstituent]: Read-only constituent views.
    #[getter]
    fn constituents(&self) -> Vec<PyBasketConstituent> {
        self.inner
            .constituents
            .iter()
            .map(|c| PyBasketConstituent::new(c.clone()))
            .collect()
    }

    /// Pricing configuration for this basket.
    ///
    /// Returns:
    ///     BasketPricingConfig: The pricing configuration.
    #[getter]
    fn pricing_config(&self) -> PyBasketPricingConfig {
        PyBasketPricingConfig::new(self.inner.pricing_config.clone())
    }

    /// Get a configured calculator for this basket.
    ///
    /// Returns:
    ///     BasketCalculator: Calculator using this basket's pricing configuration.
    fn calculator(&self) -> PyBasketCalculator {
        PyBasketCalculator {
            inner: self.inner.calculator(),
        }
    }

    /// Validate basket consistency (weights sum to ~1.0, currency consistency).
    ///
    /// Raises:
    ///     ValueError: If weights do not sum to approximately 1.0 (10bp tolerance)
    ///         or if notional currency does not match basket currency.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(crate::errors::core_to_py)
    }

    #[pyo3(text_signature = "(self)")]
    /// Serialize the basket definition to a JSON string.
    ///
    /// Returns:
    ///     str: Pretty-printed JSON representation.
    ///
    /// Raises:
    ///     ValueError: If serialization fails.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }
}

impl fmt::Display for PyBasket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Basket({}, constituents={})",
            self.inner.id,
            self.inner.constituents.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAssetType>()?;
    module.add_class::<PyBasketPricingConfig>()?;
    module.add_class::<PyBasketConstituent>()?;
    module.add_class::<PyBasketCalculator>()?;
    module.add_class::<PyBasket>()?;
    Ok(vec![
        "AssetType",
        "BasketPricingConfig",
        "BasketConstituent",
        "BasketCalculator",
        "Basket",
    ])
}
