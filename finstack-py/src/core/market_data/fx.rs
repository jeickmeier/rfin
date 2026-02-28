//! FX bindings: conversion policies, configs, rate lookups, and matrices.
//!
//! Provides a Python-friendly API for foreign-exchange operations:
//! - `FxConversionPolicy`: timing policy for cashflow conversions
//! - `FxConfig`: matrix configuration (pivot currency, triangulation, cache)
//! - `FxMatrix`: mutable quote store with date/policy-aware rate evaluation
//! - `FxRateResult`: result envelope with rate and triangulation flag
//!
//! Typical flow: configure an `FxMatrix`, load direct quotes, then call
//! `rate(from, to, on, policy)` to evaluate an FX rate for a given date.
use crate::core::currency::PyCurrency;
// use crate::core::common::args::{ExtrapolationPolicyArg, CurrencyArg};
use crate::core::dates::utils::py_to_date;
use crate::errors::{core_to_py, PyContext};
use finstack_core::money::fx::SimpleFxProvider;
use finstack_core::money::fx::{FxConfig, FxConversionPolicy, FxMatrix, FxQuery, FxRateResult};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;
use std::sync::Arc;

/// Parse a snake/lower-case label into an FX conversion policy.
///
/// Accepts labels like "cashflow_date", "period_end", "period_average", or "custom".
/// Returns a `ValueError` if the policy name is unrecognized.
fn parse_policy_from_str(value: &str) -> PyResult<FxConversionPolicy> {
    match value.to_ascii_lowercase().as_str() {
        "cashflow_date" | "cashflow" | "spot" => Ok(FxConversionPolicy::CashflowDate),
        "period_end" | "end" => Ok(FxConversionPolicy::PeriodEnd),
        "period_average" | "average" => Ok(FxConversionPolicy::PeriodAverage),
        "custom" => Ok(FxConversionPolicy::Custom),
        other => Err(PyValueError::new_err(format!(
            "Unknown FX conversion policy: {other}"
        ))),
    }
}

/// Policy describing how FX conversion is performed for projected cashflows.
///
/// Parameters
/// ----------
/// None
///     Use class attributes (e.g. :attr:`FxConversionPolicy.CASHFLOW_DATE`) or :py:meth:`FxConversionPolicy.from_name`.
///
/// Returns
/// -------
/// FxConversionPolicy
///     Enum-like value governing FX conversion timing.
#[pyclass(
    module = "finstack.core.market_data.fx",
    name = "FxConversionPolicy",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyFxConversionPolicy {
    pub(crate) inner: FxConversionPolicy,
}

impl PyFxConversionPolicy {
    pub(crate) const fn new(inner: FxConversionPolicy) -> Self {
        Self { inner }
    }

    fn label(self) -> &'static str {
        match self.inner {
            FxConversionPolicy::CashflowDate => "cashflow_date",
            FxConversionPolicy::PeriodEnd => "period_end",
            FxConversionPolicy::PeriodAverage => "period_average",
            FxConversionPolicy::Custom => "custom",
            _ => "custom",
        }
    }
}

#[pymethods]
impl PyFxConversionPolicy {
    #[classattr]
    const CASHFLOW_DATE: Self = Self {
        inner: FxConversionPolicy::CashflowDate,
    };

    #[classattr]
    const PERIOD_END: Self = Self {
        inner: FxConversionPolicy::PeriodEnd,
    };

    #[classattr]
    const PERIOD_AVERAGE: Self = Self {
        inner: FxConversionPolicy::PeriodAverage,
    };

    #[classattr]
    const CUSTOM: Self = Self {
        inner: FxConversionPolicy::Custom,
    };

    /// Parse a policy from a lowercase or snake-case string.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Policy label such as ``"cashflow_date"`` or ``"period_average"``.
    ///
    /// Returns
    /// -------
    /// FxConversionPolicy
    ///     Conversion policy corresponding to ``name``.
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_policy_from_str(name).map(Self::new)
    }

    /// Canonical string label for the conversion policy.
    ///
    /// Returns
    /// -------
    /// str
    ///     Snake-case label describing the policy.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("FxConversionPolicy('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Configuration flags controlling FX matrix evaluation.
///
/// Parameters
/// ----------
/// pivot_currency : Currency, optional
///     Currency used when triangulating cross rates.
/// enable_triangulation : bool, optional
///     Enable indirect rate discovery via the pivot currency.
/// cache_capacity : int, optional
///     Maximum number of quote pairs cached.
///
/// Returns
/// -------
/// FxConfig
///     Configuration object consumed by :class:`FxMatrix`.
#[pyclass(
    module = "finstack.core.market_data.fx",
    name = "FxConfig",
    unsendable,
    from_py_object
)]
#[derive(Clone)]
pub struct PyFxConfig {
    pub(crate) inner: FxConfig,
}

#[pymethods]
impl PyFxConfig {
    /// Create a configuration with optional pivot currency, triangulation and cache size.
    ///
    /// Parameters
    /// ----------
    /// pivot_currency : Currency, optional
    ///     Pivot currency used for triangulation when provided.
    /// enable_triangulation : bool, optional
    ///     Allow the matrix to triangulate when direct quotes are unavailable.
    /// cache_capacity : int, optional
    ///     Maximum number of direct quotes cached.
    ///
    /// Returns
    /// -------
    /// FxConfig
    ///     FX configuration object.
    #[pyo3(signature = (*, pivot_currency=None, enable_triangulation=false, cache_capacity=256))]
    #[new]
    #[pyo3(
        text_signature = "(*, pivot_currency=None, enable_triangulation=False, cache_capacity=256)"
    )]
    fn new(
        pivot_currency: Option<Bound<'_, PyAny>>,
        enable_triangulation: Option<bool>,
        cache_capacity: Option<usize>,
    ) -> Self {
        let mut config = FxConfig::default();
        if let Some(ccy_any) = pivot_currency {
            if let Ok(py_ccy) = ccy_any.extract::<PyRef<PyCurrency>>() {
                config.pivot_currency = py_ccy.inner;
            } else if let Ok(code) = ccy_any.extract::<&str>() {
                if let Ok(parsed) = finstack_core::currency::Currency::try_from(code) {
                    config.pivot_currency = parsed;
                }
            }
        }
        if let Some(flag) = enable_triangulation {
            config.enable_triangulation = flag;
        }
        if let Some(cap) = cache_capacity {
            config.cache_capacity = cap;
        }
        Self { inner: config }
    }

    /// Preferred pivot currency used when triangulating FX paths.
    ///
    /// Returns
    /// -------
    /// Currency
    ///     Pivot currency for the configuration.
    #[getter]
    fn pivot_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.pivot_currency)
    }

    /// Whether the matrix is allowed to triangulate via intermediate currencies.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` when triangulation is enabled.
    #[getter]
    fn enable_triangulation(&self) -> bool {
        self.inner.enable_triangulation
    }

    /// Maximum number of cached FX rates.
    ///
    /// Returns
    /// -------
    /// int
    ///     Capacity of the FX matrix cache.
    #[getter]
    fn cache_capacity(&self) -> usize {
        self.inner.cache_capacity
    }
}

/// Result of an FX lookup including whether triangulation was used.
///
/// Parameters
/// ----------
/// rate : float
///     Effective FX rate between the requested currencies.
/// triangulated : bool
///     Flag indicating whether intermediate currencies were used.
///
/// Returns
/// -------
/// FxRateResult
///     Data class capturing the outcome of :py:meth:`FxMatrix.rate`.
#[pyclass(
    module = "finstack.core.market_data.fx",
    name = "FxRateResult",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxRateResult {
    #[pyo3(get)]
    pub rate: f64,
    #[pyo3(get)]
    pub triangulated: bool,
}

impl From<FxRateResult> for PyFxRateResult {
    fn from(value: FxRateResult) -> Self {
        Self {
            rate: value.rate,
            triangulated: value.triangulated,
        }
    }
}

// SimpleFxProvider is now provided by finstack-core

/// Coerce an optional Python object into an `FxConversionPolicy`.
///
/// Accepts `None` (defaults to cashflow date), an `FxConversionPolicy` instance,
/// or a string label understood by `parse_policy_from_str`.
pub(crate) fn parse_policy(
    _py: Python<'_>,
    policy: Option<Bound<'_, PyAny>>,
) -> PyResult<FxConversionPolicy> {
    match policy {
        None => Ok(FxConversionPolicy::CashflowDate),
        Some(value) => {
            if let Ok(enum_value) = value.extract::<PyFxConversionPolicy>() {
                return Ok(enum_value.inner);
            }
            if let Ok(label) = value.extract::<&str>() {
                return parse_policy_from_str(label);
            }
            Err(PyTypeError::new_err(
                "policy must be FxConversionPolicy or string",
            ))
        }
    }
}

/// Mutable FX quote container with date-aware rate queries.
///
/// Parameters
/// ----------
/// config : FxConfig, optional
///     Configuration overrides such as triangulation and cache capacity.
///
/// Returns
/// -------
/// FxMatrix
///     FX quote matrix capable of storing direct quotes and evaluating rates.
#[pyclass(
    module = "finstack.core.market_data.fx",
    name = "FxMatrix",
    unsendable,
    from_py_object
)]
#[derive(Clone)]
pub struct PyFxMatrix {
    provider: Arc<SimpleFxProvider>,
    pub(crate) inner: Arc<FxMatrix>,
}

impl PyFxMatrix {
    fn new_with(provider: Arc<SimpleFxProvider>, config: FxConfig) -> Self {
        let matrix = FxMatrix::with_config(provider.clone(), config);
        Self {
            provider,
            inner: Arc::new(matrix),
        }
    }
}

#[pymethods]
impl PyFxMatrix {
    /// Create an FX matrix optionally seeded with configuration overrides.
    ///
    /// Parameters
    /// ----------
    /// config : FxConfig, optional
    ///     Configuration controlling triangulation and cache behaviour.
    ///
    /// Returns
    /// -------
    /// FxMatrix
    ///     Mutable FX matrix ready to accept quotes.
    #[new]
    #[pyo3(signature = (*, config=None))]
    #[pyo3(text_signature = "(*, config=None)")]
    fn ctor(config: Option<&PyFxConfig>) -> Self {
        let provider = Arc::new(SimpleFxProvider::new());
        let cfg = config.map(|c| c.inner).unwrap_or_default();
        Self::new_with(provider, cfg)
    }

    /// Register a direct FX quote from one currency into another.
    ///
    /// Parameters
    /// ----------
    /// from_currency : Currency
    ///     Base currency.
    /// to_currency : Currency
    ///     Quote currency.
    /// rate : float
    ///     Direct FX rate ``from/to``.
    ///
    /// Returns
    /// -------
    /// None
    ///
    /// Examples
    /// --------
    /// >>> fx = FxMatrix()
    /// >>> fx.set_quote(Currency("EUR"), Currency("USD"), 1.1)
    #[pyo3(text_signature = "(self, from_currency, to_currency, rate)")]
    fn set_quote(
        &self,
        from_currency: &PyCurrency,
        to_currency: &PyCurrency,
        rate: f64,
    ) -> PyResult<()> {
        if !rate.is_finite() || rate <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "FxMatrix.set_quote requires finite, positive rate (got {}->{}={rate})",
                from_currency.inner, to_currency.inner
            )));
        }
        self.provider
            .set_quote(from_currency.inner, to_currency.inner, rate);
        self.inner
            .set_quote(from_currency.inner, to_currency.inner, rate);
        Ok(())
    }

    /// Bulk-load direct quotes from an iterable of ``(from, to, rate)`` tuples.
    ///
    /// Parameters
    /// ----------
    /// quotes : Iterable[tuple[Currency, Currency, float]]
    ///     Direct quote entries.
    ///
    /// Returns
    /// -------
    /// None
    #[pyo3(text_signature = "(self, quotes)")]
    fn set_quotes(&self, quotes: Vec<(PyCurrency, PyCurrency, f64)>) -> PyResult<()> {
        let mut converted = Vec::with_capacity(quotes.len());
        for (from, to, rate) in &quotes {
            if !rate.is_finite() || *rate <= 0.0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "FxMatrix.set_quotes requires finite, positive rates (got {}->{}={})",
                    from.inner, to.inner, rate
                )));
            }
            converted.push((from.inner, to.inner, *rate));
        }
        self.provider.set_quotes(&converted);
        for (from, to, rate) in converted {
            self.inner.set_quote(from, to, rate);
        }
        Ok(())
    }

    /// Evaluate an FX rate for ``from_currency`` into ``to_currency`` on a given date.
    ///
    /// Parameters
    /// ----------
    /// from_currency : Currency
    ///     Base currency.
    /// to_currency : Currency
    ///     Quote currency.
    /// on : datetime.date
    ///     Valuation date.
    /// policy : FxConversionPolicy or str, optional
    ///     Conversion timing policy (defaults to cashflow date).
    ///
    /// Returns
    /// -------
    /// FxRateResult
    ///     Result containing the rate and triangulation metadata.
    ///
    /// Examples
    /// --------
    /// >>> fx = FxMatrix()
    /// >>> fx.set_quote(Currency("EUR"), Currency("USD"), 1.1)
    /// >>> fx.rate(Currency("EUR"), Currency("USD"), date(2024, 1, 2))
    #[pyo3(signature = (from_currency, to_currency, on, policy=None), text_signature = "(self, from_currency, to_currency, on, policy=None)")]
    fn rate(
        &self,
        py: Python<'_>,
        from_currency: &PyCurrency,
        to_currency: &PyCurrency,
        on: Bound<'_, PyAny>,
        policy: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyFxRateResult> {
        let date = py_to_date(&on).context("on")?;
        let parsed_policy = parse_policy(py, policy)?;
        let query =
            FxQuery::with_policy(from_currency.inner, to_currency.inner, date, parsed_policy);
        let result = self.inner.rate(query).map_err(core_to_py)?;
        Ok(PyFxRateResult::from(result))
    }

    /// Inspect internal cache usage.
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of cached quotes.
    #[pyo3(text_signature = "(self)")]
    fn cache_stats(&self) -> usize {
        self.inner.cache_stats()
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "fx")?;
    module.setattr(
        "__doc__",
        "Foreign-exchange helpers: conversion policies, configs, matrices, and lookup results.",
    )?;
    module.add_class::<PyFxConversionPolicy>()?;
    module.add_class::<PyFxConfig>()?;
    module.add_class::<PyFxRateResult>()?;
    module.add_class::<PyFxMatrix>()?;
    let exports = ["FxConversionPolicy", "FxConfig", "FxRateResult", "FxMatrix"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
