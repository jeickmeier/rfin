//! Python bindings for portfolio valuation.

use crate::core::config::PyFinstackConfig;
use crate::core::market_data::context::PyMarketContext;
use crate::core::money::PyMoney;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::portfolio::extract_portfolio;
use finstack_portfolio::valuation::{
    value_portfolio, value_portfolio_with_options, PortfolioValuation, PortfolioValuationOptions,
    PositionValue,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;

/// Options controlling portfolio valuation behaviour.
#[pyclass(module = "finstack.portfolio", name = "PortfolioValuationOptions")]
#[derive(Clone)]
pub struct PyPortfolioValuationOptions {
    pub(crate) inner: PortfolioValuationOptions,
}

impl PyPortfolioValuationOptions {
    pub(crate) fn new(inner: PortfolioValuationOptions) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioValuationOptions {
    #[new]
    #[pyo3(signature = (*, strict_risk=false))]
    /// Create valuation options.
    ///
    /// Args:
    ///     strict_risk: When true, fail if risk metrics cannot be computed.
    fn new_py(strict_risk: bool) -> Self {
        Self::new(PortfolioValuationOptions {
            strict_risk,
            additional_metrics: None,
            replace_standard_metrics: false,
        })
    }

    #[getter]
    fn strict_risk(&self) -> bool {
        self.inner.strict_risk
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioValuationOptions(strict_risk={})",
            self.inner.strict_risk
        )
    }
}

/// Result of valuing a single position.
///
/// Holds both native-currency and base-currency valuations.
///
/// Examples:
///     >>> position_value.position_id
///     'POS_1'
///     >>> position_value.value_native
///     Money(USD, 1000000.0)
#[pyclass(module = "finstack.portfolio", name = "PositionValue")]
#[derive(Clone)]
pub struct PyPositionValue {
    pub(crate) inner: PositionValue,
}

impl PyPositionValue {
    pub(crate) fn new(inner: PositionValue) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionValue {
    #[getter]
    /// Get the position identifier.
    fn position_id(&self) -> String {
        self.inner.position_id.to_string()
    }

    #[getter]
    /// Get the entity identifier.
    fn entity_id(&self) -> String {
        self.inner.entity_id.to_string()
    }

    #[getter]
    /// Get the value in the instrument's native currency.
    fn value_native(&self) -> PyMoney {
        PyMoney::new(self.inner.value_native)
    }

    #[getter]
    /// Get the value converted to portfolio base currency.
    fn value_base(&self) -> PyMoney {
        PyMoney::new(self.inner.value_base)
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionValue(position='{}', entity='{}', native={}, base={})",
            self.inner.position_id,
            self.inner.entity_id,
            self.inner.value_native,
            self.inner.value_base
        )
    }

    fn __str__(&self) -> String {
        format!("{}: {}", self.inner.position_id, self.inner.value_base)
    }
}

/// Complete portfolio valuation results.
///
/// Provides per-position valuations, totals by entity, and the grand total.
///
/// Examples:
///     >>> valuation = value_portfolio(portfolio, market_context, config)
///     >>> valuation.total_base_ccy
///     Money(USD, 10000000.0)
///     >>> valuation.by_entity["ENTITY_A"]
///     Money(USD, 5000000.0)
#[pyclass(module = "finstack.portfolio", name = "PortfolioValuation")]
#[derive(Clone)]
pub struct PyPortfolioValuation {
    pub(crate) inner: PortfolioValuation,
}

impl PyPortfolioValuation {
    pub(crate) fn new(inner: PortfolioValuation) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioValuation {
    #[pyo3(text_signature = "($self, position_id)")]
    /// Get the value for a specific position.
    ///
    /// Args:
    ///     position_id: Identifier to query.
    ///
    /// Returns:
    ///     PositionValue or None: The position value if found.
    ///
    /// Examples:
    ///     >>> position_value = valuation.get_position_value("POS_1")
    fn get_position_value(&self, position_id: &str) -> Option<PyPositionValue> {
        self.inner
            .get_position_value(position_id)
            .map(|pv| PyPositionValue::new(pv.clone()))
    }

    #[pyo3(text_signature = "($self, entity_id)")]
    /// Get the total value for a specific entity.
    ///
    /// Args:
    ///     entity_id: Entity identifier to query.
    ///
    /// Returns:
    ///     Money or None: The entity's total value if found.
    ///
    /// Examples:
    ///     >>> entity_value = valuation.get_entity_value("ENTITY_A")
    fn get_entity_value(&self, entity_id: &str) -> Option<PyMoney> {
        let entity_id_string = entity_id.to_string();
        self.inner
            .get_entity_value(&entity_id_string)
            .map(|m| PyMoney::new(*m))
    }

    #[getter]
    /// Get values for each position.
    fn position_values(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (id, pv) in &self.inner.position_values {
            dict.set_item(id.as_str(), PyPositionValue::new(pv.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get the total portfolio value in base currency.
    fn total_base_ccy(&self) -> PyMoney {
        PyMoney::new(self.inner.total_base_ccy)
    }

    #[getter]
    /// Get aggregated values by entity.
    fn by_entity(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (id, money) in &self.inner.by_entity {
            dict.set_item(id.as_str(), PyMoney::new(*money))?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioValuation(total={}, positions={}, entities={})",
            self.inner.total_base_ccy,
            self.inner.position_values.len(),
            self.inner.by_entity.len()
        )
    }

    fn __str__(&self) -> String {
        format!("Portfolio Total: {}", self.inner.total_base_ccy)
    }
}

/// Value a complete portfolio.
///
/// Args:
///     portfolio: Portfolio to value.
///     market_context: Market data context.
///     config: Finstack configuration (optional, uses default if not provided).
///
/// Returns:
///     PortfolioValuation: Complete valuation results.
///
/// Raises:
///     RuntimeError: If valuation fails.
///
/// Examples:
///     >>> from finstack.portfolio import value_portfolio
///     >>> from finstack.core import FinstackConfig
///     >>> valuation = value_portfolio(portfolio, market_context, FinstackConfig())
#[pyfunction]
#[pyo3(signature = (portfolio, market_context, config=None))]
fn py_value_portfolio(
    portfolio: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPortfolioValuation> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let cfg = if let Some(config_obj) = config {
        config_obj
            .extract::<PyRef<PyFinstackConfig>>()?
            .inner
            .clone()
    } else {
        finstack_core::config::FinstackConfig::default()
    };

    let valuation =
        value_portfolio(&portfolio_inner, &market_ctx.inner, &cfg).map_err(portfolio_to_py)?;

    Ok(PyPortfolioValuation::new(valuation))
}

/// Value a portfolio with explicit valuation options.
#[pyfunction]
#[pyo3(signature = (portfolio, market_context, options, config=None))]
fn py_value_portfolio_with_options(
    portfolio: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    options: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPortfolioValuation> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;
    let opts = options
        .extract::<PyRef<PyPortfolioValuationOptions>>()?
        .inner
        .clone();

    let cfg = if let Some(config_obj) = config {
        config_obj
            .extract::<PyRef<PyFinstackConfig>>()?
            .inner
            .clone()
    } else {
        finstack_core::config::FinstackConfig::default()
    };

    let valuation = value_portfolio_with_options(&portfolio_inner, &market_ctx.inner, &cfg, &opts)
        .map_err(portfolio_to_py)?;

    Ok(PyPortfolioValuation::new(valuation))
}

/// Extract a PortfolioValuation from Python object.
pub(crate) fn extract_portfolio_valuation(
    value: &Bound<'_, PyAny>,
) -> PyResult<PortfolioValuation> {
    if let Ok(py_val) = value.extract::<PyRef<PyPortfolioValuation>>() {
        Ok(py_val.inner.clone())
    } else {
        Err(PyTypeError::new_err("Expected PortfolioValuation"))
    }
}

/// Register valuation module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyPortfolioValuationOptions>()?;
    parent.add_class::<PyPositionValue>()?;
    parent.add_class::<PyPortfolioValuation>()?;

    let wrapped_fn = wrap_pyfunction!(py_value_portfolio, parent)?;
    parent.add("value_portfolio", wrapped_fn)?;

    let wrapped_with_opts = wrap_pyfunction!(py_value_portfolio_with_options, parent)?;
    parent.add("value_portfolio_with_options", wrapped_with_opts)?;

    Ok(vec![
        "PortfolioValuationOptions".to_string(),
        "PositionValue".to_string(),
        "PortfolioValuation".to_string(),
        "value_portfolio".to_string(),
        "value_portfolio_with_options".to_string(),
    ])
}
