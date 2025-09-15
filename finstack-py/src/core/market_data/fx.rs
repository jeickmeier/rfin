//! Python bindings for foreign exchange functionality.

use finstack_core::currency::Currency as CoreCurrency;
use finstack_core::dates::Date;
use finstack_core::money::fx::{
    FxConversionPolicy as CorePolicy, FxMatrix as CoreMatrix, FxProvider as CoreProvider, FxQuery as CoreFxQuery, FxRate,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::core::currency::PyCurrency;
use crate::core::dates::PyDate;
use crate::core::money::PyMoney;

/// FX conversion policy for determining which rate to use.
///
/// This enum determines how FX rates are applied when converting
/// monetary amounts between currencies:
///
/// - CashflowDate: Use the spot/forward rate on the cashflow date
/// - PeriodEnd: Use the rate at the end of the period
/// - PeriodAverage: Use an average rate over the period
/// - Custom: Custom strategy defined by the provider
///
/// Examples:
///     >>> from finstack.market_data import FxConversionPolicy
///     >>> policy = FxConversionPolicy.CashflowDate
#[pyclass(name = "FxConversionPolicy", module = "finstack.market_data")]
#[derive(Clone, Copy)]
pub enum PyFxConversionPolicy {
    /// Use spot/forward on the cashflow date
    CashflowDate,
    /// Use period end date
    PeriodEnd,
    /// Use an average over the period
    PeriodAverage,
    /// Custom strategy defined by the caller/provider
    Custom,
}

impl PyFxConversionPolicy {
    pub fn to_core(self) -> CorePolicy {
        match self {
            PyFxConversionPolicy::CashflowDate => CorePolicy::CashflowDate,
            PyFxConversionPolicy::PeriodEnd => CorePolicy::PeriodEnd,
            PyFxConversionPolicy::PeriodAverage => CorePolicy::PeriodAverage,
            PyFxConversionPolicy::Custom => CorePolicy::Custom,
        }
    }
}

/// Simple FX rate provider for currency conversion.
///
/// This is a basic implementation that stores fixed FX rates
/// for currency pairs. In production, you would typically use
/// a more sophisticated provider that fetches live rates.
///
/// Examples:
///     >>> from finstack.market_data import SimpleFxProvider
///     >>> from finstack import Currency
///     >>>
///     >>> # Create provider with some rates
///     >>> provider = SimpleFxProvider()
///     >>> provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
///     >>> provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)
///     >>> provider.set_rate(Currency("USD"), Currency("GBP"), 0.73)
///     >>> provider.set_rate(Currency("GBP"), Currency("USD"), 1.37)
///     >>>
///     >>> # Get a rate
///     >>> rate = provider.get_rate(Currency("USD"), Currency("EUR"))
///     >>> print(f"USD to EUR rate: {rate}")
#[pyclass(name = "SimpleFxProvider", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PySimpleFxProvider {
    rates: HashMap<(CoreCurrency, CoreCurrency), FxRate>,
}

#[pymethods]
impl PySimpleFxProvider {
    /// Create a new empty FX provider.
    #[new]
    fn new() -> Self {
        PySimpleFxProvider {
            rates: HashMap::new(),
        }
    }

    /// Set an FX rate for a currency pair.
    ///
    /// Args:
    ///     from_currency: The source currency
    ///     to_currency: The target currency
    ///     rate: The exchange rate (how many units of to_currency per unit of from_currency)
    ///
    /// Examples:
    ///     >>> provider = SimpleFxProvider()
    ///     >>> provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
    fn set_rate(&mut self, from_currency: &PyCurrency, to_currency: &PyCurrency, rate: f64) {
        let fx_rate = rate;
        self.rates
            .insert((from_currency.inner(), to_currency.inner()), fx_rate);
    }

    /// Get an FX rate for a currency pair.
    ///
    /// Args:
    ///     from_currency: The source currency
    ///     to_currency: The target currency
    ///
    /// Returns:
    ///     The exchange rate
    ///
    /// Raises:
    ///     ValueError: If the rate is not available
    fn get_rate(&self, from_currency: &PyCurrency, to_currency: &PyCurrency) -> PyResult<f64> {
        // Check for identity
        if from_currency.inner() == to_currency.inner() {
            return Ok(1.0);
        }

        // Look up the rate
        let key = (from_currency.inner(), to_currency.inner());
        self.rates.get(&key).copied().ok_or_else(|| {
            PyErr::new::<PyValueError, _>(format!(
                "FX rate not available for {} to {}",
                from_currency.inner(),
                to_currency.inner()
            ))
        })
    }

    /// Clear all rates from the provider.
    fn clear(&mut self) {
        self.rates.clear();
    }

    /// Get the number of rates stored.
    fn __len__(&self) -> usize {
        self.rates.len()
    }
}

// Implement the CoreProvider trait so we can use it with the Rust core
impl CoreProvider for PySimpleFxProvider {
    fn rate(
        &self,
        from: CoreCurrency,
        to: CoreCurrency,
        _on: Date,
        _policy: CorePolicy,
    ) -> finstack_core::Result<FxRate> {
        // For simplicity, ignore date and policy in this basic implementation
        if from == to {
            return Ok(1.0);
        }

        let key = (from, to);
        self.rates.get(&key).copied().ok_or_else(|| {
            finstack_core::error::InputError::NotFound {
                id: "fx_rate".to_string(),
            }
            .into()
        })
    }
}

// Helper function to convert money using an FX provider
pub fn convert_money(
    money: &PyMoney,
    to_currency: &PyCurrency,
    date: &PyDate,
    provider: &PySimpleFxProvider,
    policy: &PyFxConversionPolicy,
) -> PyResult<PyMoney> {
    let converted = money
        .inner()
        .convert(
            to_currency.inner(),
            date.inner(),
            provider,
            (*policy).to_core(),
        )
        .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("FX conversion failed: {}", e)))?;

    Ok(PyMoney::from_inner(converted))
}

/// FX rate matrix with caching and triangulation support.
///
/// This class wraps an FX provider and adds caching and triangulation
/// capabilities for more efficient rate lookups.
///
/// Examples:
///     >>> from finstack.market_data import FxMatrix, SimpleFxProvider
///     >>> from finstack import Currency, Date
///     >>>
///     >>> # Create matrix with a provider
///     >>> provider = SimpleFxProvider()
///     >>> provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
///     >>> matrix = FxMatrix(provider)
///     >>>
///     >>> # Get rates (with caching)
///     >>> date = Date(2025, 1, 15)
///     >>> rate = matrix.get_rate(
///     ...     Currency("USD"),
///     ...     Currency("EUR"),
///     ...     date,
///     ...     FxConversionPolicy.CashflowDate
///     ... )
#[pyclass(name = "FxMatrix", module = "finstack.market_data")]
pub struct PyFxMatrix {
    inner: CoreMatrix,
}

#[pymethods]
impl PyFxMatrix {
    /// Create a new FX matrix with the given provider.
    ///
    /// Args:
    ///     provider: The FX provider to use for rate lookups
    #[new]
    fn new(provider: PySimpleFxProvider) -> Self {
        use std::sync::Arc;
        PyFxMatrix {
            inner: CoreMatrix::new(Arc::new(provider)),
        }
    }

    /// Get an FX rate with caching.
    ///
    /// Args:
    ///     from_currency: The source currency
    ///     to_currency: The target currency
    ///     date: The date for the rate
    ///     policy: The conversion policy to use
    ///
    /// Returns:
    ///     The exchange rate
    ///
    /// Raises:
    ///     RuntimeError: If the rate cannot be obtained
    fn get_rate(
        &self,
        from_currency: &PyCurrency,
        to_currency: &PyCurrency,
        date: &PyDate,
        policy: &PyFxConversionPolicy,
    ) -> PyResult<f64> {
        self.inner
            .rate(CoreFxQuery {
                from: from_currency.inner(),
                to: to_currency.inner(),
                on: date.inner(),
                policy: (*policy).to_core(),
                closure_check: None,
                want_meta: false,
            })
            .map(|r| r.rate)
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("Failed to get FX rate: {}", e)))
    }

    /// Clear expired entries from the cache.
    fn clear_expired(&self) {
        self.inner.clear_expired();
    }

    /// Clear all cached rates.
    fn clear_cache(&self) {
        self.inner.clear_cache();
    }
}
