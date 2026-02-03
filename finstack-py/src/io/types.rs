//! Python wrappers for finstack-io types.

use crate::core::dates::utils::date_to_py;
use crate::core::market_data::context::PyMarketContext;
use finstack_io::{MarketContextSnapshot, PortfolioSnapshot};
use finstack_portfolio::PortfolioSpec;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// A time-indexed market context snapshot returned from lookback queries.
///
/// This represents a market context at a specific point in time, useful for
/// historical analysis and time-series operations.
///
/// Examples:
///     >>> from finstack.io import SqliteStore
///     >>> from datetime import date
///     >>> store = SqliteStore.open("data.db")
///     >>> snapshots = store.list_market_contexts("USD", date(2024, 1, 1), date(2024, 12, 31))
///     >>> for snap in snapshots:
///     ...     print(f"{snap.as_of}: {snap.context}")
#[pyclass(module = "finstack.io", name = "MarketContextSnapshot", frozen)]
pub struct PyMarketContextSnapshot {
    pub(crate) inner: MarketContextSnapshot,
}

impl PyMarketContextSnapshot {
    pub(crate) fn new(inner: MarketContextSnapshot) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketContextSnapshot {
    /// The as-of date for this snapshot.
    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    /// The market context snapshot.
    #[getter]
    fn context(&self) -> PyMarketContext {
        PyMarketContext {
            inner: self.inner.context.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!("MarketContextSnapshot(as_of={})", self.inner.as_of)
    }
}

/// A serializable portfolio specification.
///
/// This is a JSON-serializable representation of a portfolio that can be stored
/// and retrieved from the database. Use `Portfolio.from_spec()` to hydrate it
/// into a full `Portfolio` object.
///
/// Examples:
///     >>> from finstack.io import SqliteStore, PortfolioSpec
///     >>> store = SqliteStore.open("data.db")
///     >>> spec = store.get_portfolio_spec("FUND_A", date(2024, 1, 1))
///     >>> spec.id
///     'FUND_A'
#[pyclass(module = "finstack.io", name = "PortfolioSpec", frozen)]
pub struct PyPortfolioSpec {
    pub(crate) inner: PortfolioSpec,
}

impl PyPortfolioSpec {
    pub(crate) fn new(inner: PortfolioSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioSpec {
    /// Portfolio identifier.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Human-readable name.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    /// Base currency for aggregation (as string).
    #[getter]
    fn base_ccy(&self) -> String {
        self.inner.base_ccy.to_string()
    }

    /// Valuation date.
    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    /// Number of positions.
    #[getter]
    fn position_count(&self) -> usize {
        self.inner.positions.len()
    }

    /// Number of entities.
    #[getter]
    fn entity_count(&self) -> usize {
        self.inner.entities.len()
    }

    /// Convert to JSON-compatible dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        pythonize::pythonize(py, &self.inner)
            .map(|obj| obj.unbind())
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))
    }

    /// Create from JSON-compatible dict.
    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let spec: PortfolioSpec = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize: {}", e)))?;
        Ok(Self::new(spec))
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioSpec(id='{}', positions={}, as_of={})",
            self.inner.id,
            self.inner.positions.len(),
            self.inner.as_of
        )
    }
}

/// A time-indexed portfolio snapshot returned from lookback queries.
///
/// This represents a portfolio specification at a specific point in time,
/// useful for historical analysis and time-series operations.
///
/// Examples:
///     >>> from finstack.io import SqliteStore
///     >>> from datetime import date
///     >>> store = SqliteStore.open("data.db")
///     >>> snapshots = store.list_portfolios("FUND_A", date(2024, 1, 1), date(2024, 12, 31))
///     >>> for snap in snapshots:
///     ...     print(f"{snap.as_of}: {snap.spec.position_count} positions")
#[pyclass(module = "finstack.io", name = "PortfolioSnapshot", frozen)]
pub struct PyPortfolioSnapshot {
    pub(crate) inner: PortfolioSnapshot,
}

impl PyPortfolioSnapshot {
    pub(crate) fn new(inner: PortfolioSnapshot) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioSnapshot {
    /// The as-of date for this snapshot.
    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    /// The portfolio specification snapshot.
    #[getter]
    fn spec(&self) -> PyPortfolioSpec {
        PyPortfolioSpec::new(self.inner.spec.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioSnapshot(as_of={}, positions={})",
            self.inner.as_of,
            self.inner.spec.positions.len()
        )
    }
}

/// Register types in the module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyMarketContextSnapshot>()?;
    parent.add_class::<PyPortfolioSnapshot>()?;
    parent.add_class::<PyPortfolioSpec>()?;

    Ok(vec![
        "MarketContextSnapshot".to_string(),
        "PortfolioSnapshot".to_string(),
        "PortfolioSpec".to_string(),
    ])
}
