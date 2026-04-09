//! Python bindings for portfolio dependency index.
//!
//! Wraps [`finstack_portfolio::dependencies`] types for Python, enabling
//! selective repricing by mapping market factor changes to affected positions.

use crate::core::common::args::CurrencyArg;
use crate::portfolio::positions::PyPortfolio;
use finstack_core::types::CurveId;
use finstack_portfolio::dependencies::{DependencyIndex, MarketFactorKey};
use finstack_valuations::instruments::RatesCurveKind;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use std::str::FromStr;

/// Parse a string into a [`RatesCurveKind`].
fn parse_curve_kind(s: &str) -> PyResult<RatesCurveKind> {
    RatesCurveKind::from_str(s).map_err(|e| PyValueError::new_err(e))
}

/// Format a [`RatesCurveKind`] as a lowercase label.
fn curve_kind_label(kind: RatesCurveKind) -> &'static str {
    match kind {
        RatesCurveKind::Discount => "discount",
        RatesCurveKind::Forward => "forward",
        RatesCurveKind::Credit => "credit",
    }
}

/// Normalized market factor key for portfolio-level dependency tracking.
///
/// Each variant captures enough information to uniquely identify one atomic
/// market data input. Use the static constructors to create instances.
///
/// Examples:
///     >>> key = MarketFactorKey.curve("USD-OIS", "discount")
///     >>> key = MarketFactorKey.spot("SPX")
///     >>> key = MarketFactorKey.fx("EUR", "USD")
#[pyclass(
    name = "MarketFactorKey",
    module = "finstack.portfolio",
    frozen,
    eq,
    hash,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyMarketFactorKey {
    pub(crate) inner: MarketFactorKey,
}

impl PyMarketFactorKey {
    pub(crate) fn new(inner: MarketFactorKey) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketFactorKey {
    #[staticmethod]
    #[pyo3(text_signature = "(id, kind)")]
    /// Create a curve market factor key.
    ///
    /// Args:
    ///     id: Curve identifier (e.g. ``"USD-OIS"``).
    ///     kind: Curve kind — ``"discount"``, ``"forward"``, or ``"credit"``.
    ///
    /// Returns:
    ///     MarketFactorKey: Curve factor key.
    fn curve(id: &str, kind: &str) -> PyResult<Self> {
        let k = parse_curve_kind(kind)?;
        Ok(Self::new(MarketFactorKey::curve(CurveId::new(id), k)))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(id)")]
    /// Create a spot market factor key.
    ///
    /// Args:
    ///     id: Spot identifier (e.g. ``"SPX"``).
    ///
    /// Returns:
    ///     MarketFactorKey: Spot factor key.
    fn spot(id: &str) -> Self {
        Self::new(MarketFactorKey::spot(id))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(id)")]
    /// Create a volatility surface market factor key.
    ///
    /// Args:
    ///     id: Vol surface identifier.
    ///
    /// Returns:
    ///     MarketFactorKey: Vol-surface factor key.
    fn vol_surface(id: &str) -> Self {
        Self::new(MarketFactorKey::vol_surface(id))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(base, quote)")]
    /// Create an FX pair market factor key.
    ///
    /// Args:
    ///     base: Base currency (Currency or string code).
    ///     quote: Quote currency (Currency or string code).
    ///
    /// Returns:
    ///     MarketFactorKey: FX factor key.
    fn fx(base: CurrencyArg, quote: CurrencyArg) -> Self {
        Self::new(MarketFactorKey::fx(base.0, quote.0))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(id)")]
    /// Create a time-series market factor key.
    ///
    /// Args:
    ///     id: Series identifier.
    ///
    /// Returns:
    ///     MarketFactorKey: Series factor key.
    fn series(id: &str) -> Self {
        Self::new(MarketFactorKey::series(id))
    }

    // -- variant inspection --

    #[getter]
    /// The variant name: ``"curve"``, ``"spot"``, ``"vol_surface"``, ``"fx"``, or ``"series"``.
    fn variant(&self) -> &'static str {
        match &self.inner {
            MarketFactorKey::Curve { .. } => "curve",
            MarketFactorKey::Spot(_) => "spot",
            MarketFactorKey::VolSurface(_) => "vol_surface",
            MarketFactorKey::Fx { .. } => "fx",
            MarketFactorKey::Series(_) => "series",
        }
    }

    #[getter]
    /// The curve identifier (only for ``Curve`` variant, else ``None``).
    fn curve_id(&self) -> Option<String> {
        match &self.inner {
            MarketFactorKey::Curve { id, .. } => Some(id.to_string()),
            _ => None,
        }
    }

    #[getter]
    /// The curve kind label (only for ``Curve`` variant, else ``None``).
    fn curve_kind(&self) -> Option<&'static str> {
        match &self.inner {
            MarketFactorKey::Curve { kind, .. } => Some(curve_kind_label(*kind)),
            _ => None,
        }
    }

    #[getter]
    /// The spot/vol-surface/series identifier (``None`` for ``Curve`` and ``Fx``).
    fn id(&self) -> Option<String> {
        match &self.inner {
            MarketFactorKey::Spot(id)
            | MarketFactorKey::VolSurface(id)
            | MarketFactorKey::Series(id) => Some(id.clone()),
            _ => None,
        }
    }

    #[getter]
    /// The base currency (only for ``Fx`` variant, else ``None``).
    fn base(&self) -> Option<String> {
        match &self.inner {
            MarketFactorKey::Fx { base, .. } => Some(base.to_string()),
            _ => None,
        }
    }

    #[getter]
    /// The quote currency (only for ``Fx`` variant, else ``None``).
    fn quote_ccy(&self) -> Option<String> {
        match &self.inner {
            MarketFactorKey::Fx { quote, .. } => Some(quote.to_string()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MarketFactorKey::Curve { id, kind } => {
                format!(
                    "MarketFactorKey.curve('{}', '{}')",
                    id,
                    curve_kind_label(*kind)
                )
            }
            MarketFactorKey::Spot(id) => format!("MarketFactorKey.spot('{}')", id),
            MarketFactorKey::VolSurface(id) => format!("MarketFactorKey.vol_surface('{}')", id),
            MarketFactorKey::Fx { base, quote } => {
                format!("MarketFactorKey.fx('{}', '{}')", base, quote)
            }
            MarketFactorKey::Series(id) => format!("MarketFactorKey.series('{}')", id),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Inverted index mapping market factor keys to affected portfolio positions.
///
/// Built from a portfolio's positions, this index enables efficient lookup
/// of which positions are affected when specific market data changes.
/// Positions whose instruments failed to report dependencies are tracked
/// as *unresolved* and conservatively included in every query.
///
/// Examples:
///     >>> index = DependencyIndex.build(portfolio)
///     >>> keys = index.factors()
///     >>> affected = index.affected_positions([MarketFactorKey.spot("SPX")])
#[pyclass(name = "DependencyIndex", module = "finstack.portfolio", frozen)]
pub struct PyDependencyIndex {
    inner: DependencyIndex,
}

impl PyDependencyIndex {
    fn new(inner: DependencyIndex) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDependencyIndex {
    #[staticmethod]
    #[pyo3(text_signature = "(portfolio)")]
    /// Build a dependency index from a portfolio.
    ///
    /// Iterates all positions, calls each instrument's ``market_dependencies()``,
    /// and builds an inverted index from normalized factor keys to position indices.
    ///
    /// Args:
    ///     portfolio: Portfolio to index.
    ///
    /// Returns:
    ///     DependencyIndex: Newly built dependency index.
    fn build(portfolio: &PyPortfolio) -> Self {
        Self::new(DependencyIndex::build(portfolio.inner.positions()))
    }

    #[pyo3(text_signature = "($self, keys)")]
    /// Get the sorted, deduplicated position indices affected by any of the given keys.
    ///
    /// Unresolved positions (whose instruments could not report dependencies)
    /// are always included.
    ///
    /// Args:
    ///     keys: List of market factor keys to query.
    ///
    /// Returns:
    ///     list[int]: Sorted position indices.
    fn affected_positions(&self, keys: Vec<PyMarketFactorKey>) -> Vec<usize> {
        let rust_keys: Vec<MarketFactorKey> = keys.into_iter().map(|k| k.inner).collect();
        self.inner.affected_positions(&rust_keys)
    }

    #[pyo3(text_signature = "($self)")]
    /// Return all tracked market factor keys.
    ///
    /// Returns:
    ///     list[MarketFactorKey]: All normalized factor keys in the index.
    fn factors(&self) -> Vec<PyMarketFactorKey> {
        self.inner
            .iter()
            .map(|(k, _)| PyMarketFactorKey::new(k.clone()))
            .collect()
    }

    #[pyo3(text_signature = "($self, key)")]
    /// Look up position indices for a single market factor key.
    ///
    /// Args:
    ///     key: Market factor key to look up.
    ///
    /// Returns:
    ///     list[int] | None: Position indices, or ``None`` if the key is absent.
    fn positions_for_factor(&self, key: &PyMarketFactorKey) -> Option<Vec<usize>> {
        let positions = self.inner.positions_for_key(&key.inner);
        if positions.is_empty() {
            None
        } else {
            Some(positions.to_vec())
        }
    }

    #[pyo3(text_signature = "($self)")]
    /// Return unresolved position indices.
    ///
    /// These are positions whose instruments failed to report dependencies
    /// and are conservatively included in every ``affected_positions`` query.
    ///
    /// Returns:
    ///     list[int]: Unresolved position indices.
    fn unresolved(&self) -> Vec<usize> {
        self.inner.unresolved().to_vec()
    }

    #[getter]
    /// Number of distinct market factor keys tracked.
    fn factor_count(&self) -> usize {
        self.inner.factor_count()
    }

    fn __len__(&self) -> usize {
        self.inner.factor_count()
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!(
            "DependencyIndex(factors={}, unresolved={})",
            self.inner.factor_count(),
            self.inner.unresolved().len()
        )
    }
}

/// Register dependencies module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyMarketFactorKey>()?;
    parent.add_class::<PyDependencyIndex>()?;

    Ok(vec![
        "MarketFactorKey".to_string(),
        "DependencyIndex".to_string(),
    ])
}
