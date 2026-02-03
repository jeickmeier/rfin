//! Python bindings for SqliteStore.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::io::error::map_io_error;
use crate::io::types::{PyMarketContextSnapshot, PyPortfolioSnapshot, PyPortfolioSpec};
use crate::portfolio::portfolio::PyPortfolio;
use crate::scenarios::spec::PyScenarioSpec;
use crate::statements::registry::PyMetricRegistry;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_core::market_data::context::MarketContext;
use finstack_io::{BulkStore, LookbackStore, SqliteStore, Store};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule};
use pyo3::Bound;
use std::path::PathBuf;

/// A SQLite-backed persistence store for Finstack domain objects.
///
/// This store provides CRUD operations for market contexts, instruments, portfolios,
/// scenarios, statement models, and metric registries. All operations are atomic
/// and idempotent (upserts).
///
/// Examples:
///     >>> from finstack.io import SqliteStore
///     >>> from datetime import date
///     >>> # Open or create a database
///     >>> store = SqliteStore.open("finstack.db")
///     >>> # Store a market context
///     >>> from finstack.core.market_data import MarketContext
///     >>> market = MarketContext.empty()
///     >>> store.put_market_context("USD_MKT", date(2024, 1, 1), market)
///     >>> # Retrieve it later
///     >>> retrieved = store.get_market_context("USD_MKT", date(2024, 1, 1))
#[pyclass(module = "finstack.io", name = "SqliteStore")]
pub struct PySqliteStore {
    inner: SqliteStore,
}

#[pymethods]
impl PySqliteStore {
    /// Open or create a SQLite database at the given path.
    ///
    /// The database schema is automatically created and migrated on open.
    /// Parent directories are created if they don't exist.
    ///
    /// Args:
    ///     path: Path to the SQLite database file.
    ///
    /// Returns:
    ///     SqliteStore: The opened store instance.
    ///
    /// Raises:
    ///     IoError: If the database cannot be opened or migrated.
    ///
    /// Examples:
    ///     >>> store = SqliteStore.open("data/finstack.db")
    ///     >>> store = SqliteStore.open(":memory:")  # In-memory database
    #[staticmethod]
    #[pyo3(text_signature = "(path)")]
    fn open(path: &str) -> PyResult<Self> {
        let store = SqliteStore::open(PathBuf::from(path)).map_err(map_io_error)?;
        Ok(Self { inner: store })
    }

    /// Get the database file path.
    ///
    /// Returns:
    ///     str: Path to the SQLite database file.
    #[getter]
    fn path(&self) -> String {
        self.inner.path().to_string_lossy().to_string()
    }

    // =========================================================================
    // Market Context Operations
    // =========================================================================

    /// Store a market context snapshot.
    ///
    /// If a market context with the same ID and as_of date exists, it is replaced.
    ///
    /// Args:
    ///     market_id: Unique identifier for the market context.
    ///     as_of: Valuation date for the snapshot.
    ///     context: The market context to store.
    ///     meta: Optional metadata dict for provenance tracking.
    ///
    /// Examples:
    ///     >>> store.put_market_context("USD_MKT", date(2024, 1, 1), market)
    #[pyo3(signature = (market_id, as_of, context, meta=None))]
    #[pyo3(text_signature = "($self, market_id, as_of, context, meta=None)")]
    fn put_market_context(
        &self,
        market_id: &str,
        as_of: &Bound<'_, PyAny>,
        context: &PyMarketContext,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let date = py_to_date(as_of)?;
        let meta_json = extract_meta(meta)?;
        self.inner
            .put_market_context(market_id, date, &context.inner, meta_json.as_ref())
            .map_err(map_io_error)
    }

    /// Retrieve a market context snapshot.
    ///
    /// Args:
    ///     market_id: Market context identifier.
    ///     as_of: Valuation date to retrieve.
    ///
    /// Returns:
    ///     MarketContext or None: The market context if found.
    ///
    /// Examples:
    ///     >>> market = store.get_market_context("USD_MKT", date(2024, 1, 1))
    #[pyo3(text_signature = "($self, market_id, as_of)")]
    fn get_market_context(
        &self,
        market_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyMarketContext>> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .get_market_context(market_id, date)
            .map_err(map_io_error)?;
        Ok(result.map(|inner| PyMarketContext { inner }))
    }

    /// Load a market context, raising an error if not found.
    ///
    /// Args:
    ///     market_id: Market context identifier.
    ///     as_of: Valuation date to retrieve.
    ///
    /// Returns:
    ///     MarketContext: The market context.
    ///
    /// Raises:
    ///     NotFoundError: If the market context is not found.
    #[pyo3(text_signature = "($self, market_id, as_of)")]
    fn load_market_context(
        &self,
        market_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyMarketContext> {
        let date = py_to_date(as_of)?;
        let inner = self
            .inner
            .load_market_context(market_id, date)
            .map_err(map_io_error)?;
        Ok(PyMarketContext { inner })
    }

    // =========================================================================
    // Instrument Operations
    // =========================================================================

    /// Store an instrument definition.
    ///
    /// Instruments are stored as JSON and can be any supported instrument type.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for the instrument.
    ///     instrument: Instrument definition as a dict (JSON-serializable).
    ///     meta: Optional metadata dict.
    ///
    /// Examples:
    ///     >>> instrument = {"type": "Deposit", "currency": "USD", ...}
    ///     >>> store.put_instrument("DEP_1M_USD", instrument)
    #[pyo3(signature = (instrument_id, instrument, meta=None))]
    #[pyo3(text_signature = "($self, instrument_id, instrument, meta=None)")]
    fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &Bound<'_, PyAny>,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let instr: InstrumentJson = pythonize::depythonize(instrument)
            .map_err(|e| PyValueError::new_err(format!("Invalid instrument: {}", e)))?;
        let meta_json = extract_meta(meta)?;
        self.inner
            .put_instrument(instrument_id, &instr, meta_json.as_ref())
            .map_err(map_io_error)
    }

    /// Retrieve an instrument definition.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///
    /// Returns:
    ///     dict or None: The instrument as a dict if found.
    ///
    /// Examples:
    ///     >>> instr = store.get_instrument("DEP_1M_USD")
    ///     >>> if instr:
    ///     ...     print(instr["type"])
    #[pyo3(text_signature = "($self, instrument_id)")]
    fn get_instrument(&self, py: Python<'_>, instrument_id: &str) -> PyResult<Option<Py<PyAny>>> {
        let result = self
            .inner
            .get_instrument(instrument_id)
            .map_err(map_io_error)?;
        match result {
            Some(instr) => {
                let py_obj = pythonize::pythonize(py, &instr)
                    .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))?;
                Ok(Some(py_obj.unbind()))
            }
            None => Ok(None),
        }
    }

    // =========================================================================
    // Portfolio Operations
    // =========================================================================

    /// Store a portfolio specification.
    ///
    /// Args:
    ///     portfolio_id: Unique identifier for the portfolio.
    ///     as_of: Valuation date for the snapshot.
    ///     spec: Portfolio specification (PortfolioSpec or dict).
    ///     meta: Optional metadata dict.
    ///
    /// Examples:
    ///     >>> store.put_portfolio_spec("FUND_A", date(2024, 1, 1), spec)
    #[pyo3(signature = (portfolio_id, as_of, spec, meta=None))]
    #[pyo3(text_signature = "($self, portfolio_id, as_of, spec, meta=None)")]
    fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: &Bound<'_, PyAny>,
        spec: &Bound<'_, PyAny>,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let date = py_to_date(as_of)?;
        let portfolio_spec = extract_portfolio_spec(spec)?;
        let meta_json = extract_meta(meta)?;
        self.inner
            .put_portfolio_spec(portfolio_id, date, &portfolio_spec, meta_json.as_ref())
            .map_err(map_io_error)
    }

    /// Retrieve a portfolio specification.
    ///
    /// Args:
    ///     portfolio_id: Portfolio identifier.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     PortfolioSpec or None: The portfolio spec if found.
    #[pyo3(text_signature = "($self, portfolio_id, as_of)")]
    fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyPortfolioSpec>> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .get_portfolio_spec(portfolio_id, date)
            .map_err(map_io_error)?;
        Ok(result.map(PyPortfolioSpec::new))
    }

    /// Load and hydrate a portfolio.
    ///
    /// This loads the portfolio spec and resolves any missing instrument definitions
    /// from the instrument registry.
    ///
    /// Args:
    ///     portfolio_id: Portfolio identifier.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     Portfolio: The hydrated portfolio.
    ///
    /// Raises:
    ///     NotFoundError: If the portfolio or required instruments are not found.
    #[pyo3(text_signature = "($self, portfolio_id, as_of)")]
    fn load_portfolio(
        &self,
        portfolio_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyPortfolio> {
        let date = py_to_date(as_of)?;
        let portfolio = self
            .inner
            .load_portfolio(portfolio_id, date)
            .map_err(map_io_error)?;
        Ok(PyPortfolio::new(portfolio))
    }

    /// Load a portfolio and matching market context.
    ///
    /// Convenience method to load both a portfolio and its corresponding market
    /// context for the same as_of date.
    ///
    /// Args:
    ///     portfolio_id: Portfolio identifier.
    ///     market_id: Market context identifier.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     tuple[Portfolio, MarketContext]: The portfolio and market context.
    #[pyo3(text_signature = "($self, portfolio_id, market_id, as_of)")]
    fn load_portfolio_with_market(
        &self,
        portfolio_id: &str,
        market_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<(PyPortfolio, PyMarketContext)> {
        let date = py_to_date(as_of)?;
        let (portfolio, market) = self
            .inner
            .load_portfolio_with_market(portfolio_id, market_id, date)
            .map_err(map_io_error)?;
        Ok((
            PyPortfolio::new(portfolio),
            PyMarketContext { inner: market },
        ))
    }

    // =========================================================================
    // Scenario Operations
    // =========================================================================

    /// Store a scenario specification.
    ///
    /// Args:
    ///     scenario_id: Unique identifier for the scenario.
    ///     spec: Scenario specification.
    ///     meta: Optional metadata dict.
    #[pyo3(signature = (scenario_id, spec, meta=None))]
    #[pyo3(text_signature = "($self, scenario_id, spec, meta=None)")]
    fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &Bound<'_, PyAny>,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let scenario_spec = extract_scenario_spec(spec)?;
        let meta_json = extract_meta(meta)?;
        self.inner
            .put_scenario(scenario_id, &scenario_spec, meta_json.as_ref())
            .map_err(map_io_error)
    }

    /// Retrieve a scenario specification.
    ///
    /// Args:
    ///     scenario_id: Scenario identifier.
    ///
    /// Returns:
    ///     ScenarioSpec or None: The scenario spec if found.
    #[pyo3(text_signature = "($self, scenario_id)")]
    fn get_scenario(&self, scenario_id: &str) -> PyResult<Option<PyScenarioSpec>> {
        let result = self.inner.get_scenario(scenario_id).map_err(map_io_error)?;
        Ok(result.map(PyScenarioSpec::from_inner))
    }

    // =========================================================================
    // Statement Model Operations
    // =========================================================================

    /// Store a financial statement model specification.
    ///
    /// Args:
    ///     model_id: Unique identifier for the model.
    ///     spec: Financial model specification.
    ///     meta: Optional metadata dict.
    #[pyo3(signature = (model_id, spec, meta=None))]
    #[pyo3(text_signature = "($self, model_id, spec, meta=None)")]
    fn put_statement_model(
        &self,
        model_id: &str,
        spec: &Bound<'_, PyAny>,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let model_spec = extract_statement_model(spec)?;
        let meta_json = extract_meta(meta)?;
        self.inner
            .put_statement_model(model_id, &model_spec, meta_json.as_ref())
            .map_err(map_io_error)
    }

    /// Retrieve a financial statement model specification.
    ///
    /// Args:
    ///     model_id: Model identifier.
    ///
    /// Returns:
    ///     FinancialModelSpec or None: The model spec if found.
    #[pyo3(text_signature = "($self, model_id)")]
    fn get_statement_model(&self, model_id: &str) -> PyResult<Option<PyFinancialModelSpec>> {
        let result = self
            .inner
            .get_statement_model(model_id)
            .map_err(map_io_error)?;
        Ok(result.map(PyFinancialModelSpec::new))
    }

    // =========================================================================
    // Metric Registry Operations
    // =========================================================================

    /// Store a metric registry.
    ///
    /// Args:
    ///     namespace: Registry namespace (e.g., "fin", "custom").
    ///     registry: The metric registry.
    ///     meta: Optional metadata dict.
    #[pyo3(signature = (namespace, registry, meta=None))]
    #[pyo3(text_signature = "($self, namespace, registry, meta=None)")]
    fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &Bound<'_, PyAny>,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let reg = extract_metric_registry(registry)?;
        let meta_json = extract_meta(meta)?;
        self.inner
            .put_metric_registry(namespace, &reg, meta_json.as_ref())
            .map_err(map_io_error)
    }

    /// Retrieve a metric registry.
    ///
    /// Args:
    ///     namespace: Registry namespace.
    ///
    /// Returns:
    ///     MetricRegistry or None: The registry if found.
    #[pyo3(text_signature = "($self, namespace)")]
    fn get_metric_registry(&self, namespace: &str) -> PyResult<Option<PyMetricRegistry>> {
        let result = self
            .inner
            .get_metric_registry(namespace)
            .map_err(map_io_error)?;
        Ok(result.map(PyMetricRegistry::new))
    }

    /// Load a metric registry, raising an error if not found.
    #[pyo3(text_signature = "($self, namespace)")]
    fn load_metric_registry(&self, namespace: &str) -> PyResult<PyMetricRegistry> {
        let reg = self
            .inner
            .load_metric_registry(namespace)
            .map_err(map_io_error)?;
        Ok(PyMetricRegistry::new(reg))
    }

    /// List all metric registry namespaces.
    ///
    /// Returns:
    ///     list[str]: List of namespace names.
    #[pyo3(text_signature = "($self)")]
    fn list_metric_registries(&self) -> PyResult<Vec<String>> {
        self.inner.list_metric_registries().map_err(map_io_error)
    }

    /// Delete a metric registry.
    ///
    /// Args:
    ///     namespace: Registry namespace to delete.
    ///
    /// Returns:
    ///     bool: True if the registry was deleted, False if not found.
    #[pyo3(text_signature = "($self, namespace)")]
    fn delete_metric_registry(&self, namespace: &str) -> PyResult<bool> {
        self.inner
            .delete_metric_registry(namespace)
            .map_err(map_io_error)
    }

    // =========================================================================
    // Bulk Operations
    // =========================================================================

    /// Store multiple instruments in a single transaction.
    ///
    /// This is more efficient than calling put_instrument repeatedly.
    ///
    /// Args:
    ///     instruments: List of (instrument_id, instrument_dict) tuples,
    ///                  or (instrument_id, instrument_dict, meta_dict) tuples.
    ///
    /// Examples:
    ///     >>> instruments = [
    ///     ...     ("DEP_1M", {"type": "Deposit", ...}),
    ///     ...     ("DEP_3M", {"type": "Deposit", ...}),
    ///     ... ]
    ///     >>> store.put_instruments_batch(instruments)
    #[pyo3(text_signature = "($self, instruments)")]
    fn put_instruments_batch(&self, instruments: &Bound<'_, PyList>) -> PyResult<()> {
        let mut batch: Vec<(String, InstrumentJson, Option<serde_json::Value>)> = Vec::new();

        for item in instruments.iter() {
            let tuple = item.downcast::<pyo3::types::PyTuple>()?;
            let id: String = tuple.get_item(0)?.extract()?;
            let instr: InstrumentJson = pythonize::depythonize(&tuple.get_item(1)?)
                .map_err(|e| PyValueError::new_err(format!("Invalid instrument: {}", e)))?;
            let meta = if tuple.len() > 2 {
                let meta_item = tuple.get_item(2)?;
                if meta_item.is_none() {
                    None
                } else {
                    Some(
                        pythonize::depythonize(&meta_item)
                            .map_err(|e| PyValueError::new_err(format!("Invalid meta: {}", e)))?,
                    )
                }
            } else {
                None
            };
            batch.push((id, instr, meta));
        }

        // Convert to the format expected by the Rust API
        let refs: Vec<(&str, &InstrumentJson, Option<&serde_json::Value>)> = batch
            .iter()
            .map(|(id, instr, meta)| (id.as_str(), instr, meta.as_ref()))
            .collect();

        self.inner
            .put_instruments_batch(&refs)
            .map_err(map_io_error)
    }

    /// Store multiple market contexts in a single transaction.
    ///
    /// Args:
    ///     contexts: List of (market_id, as_of, context) tuples,
    ///               or (market_id, as_of, context, meta) tuples.
    #[pyo3(text_signature = "($self, contexts)")]
    fn put_market_contexts_batch(&self, contexts: &Bound<'_, PyList>) -> PyResult<()> {
        let mut batch: Vec<(
            String,
            finstack_core::dates::Date,
            MarketContext,
            Option<serde_json::Value>,
        )> = Vec::new();

        for item in contexts.iter() {
            let tuple = item.downcast::<pyo3::types::PyTuple>()?;
            let id: String = tuple.get_item(0)?.extract()?;
            let date = py_to_date(&tuple.get_item(1)?)?;
            let ctx: PyRef<PyMarketContext> = tuple.get_item(2)?.extract()?;
            let meta = if tuple.len() > 3 {
                let meta_item = tuple.get_item(3)?;
                if meta_item.is_none() {
                    None
                } else {
                    Some(
                        pythonize::depythonize(&meta_item)
                            .map_err(|e| PyValueError::new_err(format!("Invalid meta: {}", e)))?,
                    )
                }
            } else {
                None
            };
            batch.push((id, date, ctx.inner.clone(), meta));
        }

        let refs: Vec<(
            &str,
            finstack_core::dates::Date,
            &MarketContext,
            Option<&serde_json::Value>,
        )> = batch
            .iter()
            .map(|(id, date, ctx, meta)| (id.as_str(), *date, ctx, meta.as_ref()))
            .collect();

        self.inner
            .put_market_contexts_batch(&refs)
            .map_err(map_io_error)
    }

    /// Store multiple portfolio specs in a single transaction.
    ///
    /// Args:
    ///     portfolios: List of (portfolio_id, as_of, spec) tuples,
    ///                 or (portfolio_id, as_of, spec, meta) tuples.
    #[pyo3(text_signature = "($self, portfolios)")]
    fn put_portfolio_specs_batch(&self, portfolios: &Bound<'_, PyList>) -> PyResult<()> {
        let mut batch: Vec<(
            String,
            finstack_core::dates::Date,
            PortfolioSpec,
            Option<serde_json::Value>,
        )> = Vec::new();

        for item in portfolios.iter() {
            let tuple = item.downcast::<pyo3::types::PyTuple>()?;
            let id: String = tuple.get_item(0)?.extract()?;
            let date = py_to_date(&tuple.get_item(1)?)?;
            let spec = extract_portfolio_spec(&tuple.get_item(2)?)?;
            let meta = if tuple.len() > 3 {
                let meta_item = tuple.get_item(3)?;
                if meta_item.is_none() {
                    None
                } else {
                    Some(
                        pythonize::depythonize(&meta_item)
                            .map_err(|e| PyValueError::new_err(format!("Invalid meta: {}", e)))?,
                    )
                }
            } else {
                None
            };
            batch.push((id, date, spec, meta));
        }

        let refs: Vec<(
            &str,
            finstack_core::dates::Date,
            &PortfolioSpec,
            Option<&serde_json::Value>,
        )> = batch
            .iter()
            .map(|(id, date, spec, meta)| (id.as_str(), *date, spec, meta.as_ref()))
            .collect();

        self.inner
            .put_portfolio_specs_batch(&refs)
            .map_err(map_io_error)
    }

    // =========================================================================
    // Lookback Operations
    // =========================================================================

    /// List market context snapshots in a date range.
    ///
    /// Args:
    ///     market_id: Market context identifier.
    ///     start: Start date (inclusive).
    ///     end: End date (inclusive).
    ///
    /// Returns:
    ///     list[MarketContextSnapshot]: Snapshots ordered by as_of date.
    #[pyo3(text_signature = "($self, market_id, start, end)")]
    fn list_market_contexts(
        &self,
        market_id: &str,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
    ) -> PyResult<Vec<PyMarketContextSnapshot>> {
        let start_date = py_to_date(start)?;
        let end_date = py_to_date(end)?;
        let snapshots = self
            .inner
            .list_market_contexts(market_id, start_date, end_date)
            .map_err(map_io_error)?;
        Ok(snapshots
            .into_iter()
            .map(PyMarketContextSnapshot::new)
            .collect())
    }

    /// Get the latest market context on or before a date.
    ///
    /// Args:
    ///     market_id: Market context identifier.
    ///     as_of: Maximum date to search.
    ///
    /// Returns:
    ///     MarketContextSnapshot or None: The latest snapshot if found.
    #[pyo3(text_signature = "($self, market_id, as_of)")]
    fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyMarketContextSnapshot>> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .latest_market_context_on_or_before(market_id, date)
            .map_err(map_io_error)?;
        Ok(result.map(PyMarketContextSnapshot::new))
    }

    /// List portfolio snapshots in a date range.
    ///
    /// Args:
    ///     portfolio_id: Portfolio identifier.
    ///     start: Start date (inclusive).
    ///     end: End date (inclusive).
    ///
    /// Returns:
    ///     list[PortfolioSnapshot]: Snapshots ordered by as_of date.
    #[pyo3(text_signature = "($self, portfolio_id, start, end)")]
    fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
    ) -> PyResult<Vec<PyPortfolioSnapshot>> {
        let start_date = py_to_date(start)?;
        let end_date = py_to_date(end)?;
        let snapshots = self
            .inner
            .list_portfolios(portfolio_id, start_date, end_date)
            .map_err(map_io_error)?;
        Ok(snapshots
            .into_iter()
            .map(PyPortfolioSnapshot::new)
            .collect())
    }

    /// Get the latest portfolio on or before a date.
    ///
    /// Args:
    ///     portfolio_id: Portfolio identifier.
    ///     as_of: Maximum date to search.
    ///
    /// Returns:
    ///     PortfolioSnapshot or None: The latest snapshot if found.
    #[pyo3(text_signature = "($self, portfolio_id, as_of)")]
    fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyPortfolioSnapshot>> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .latest_portfolio_on_or_before(portfolio_id, date)
            .map_err(map_io_error)?;
        Ok(result.map(PyPortfolioSnapshot::new))
    }

    fn __repr__(&self) -> String {
        format!("SqliteStore(path='{}')", self.inner.path().display())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract metadata from a Python object to JSON.
fn extract_meta(meta: Option<&Bound<'_, PyAny>>) -> PyResult<Option<serde_json::Value>> {
    match meta {
        Some(obj) if !obj.is_none() => {
            let value: serde_json::Value = pythonize::depythonize(obj)
                .map_err(|e| PyValueError::new_err(format!("Invalid metadata: {}", e)))?;
            Ok(Some(value))
        }
        _ => Ok(None),
    }
}

/// Extract a PortfolioSpec from a Python object.
fn extract_portfolio_spec(spec: &Bound<'_, PyAny>) -> PyResult<PortfolioSpec> {
    if let Ok(py_spec) = spec.extract::<PyRef<PyPortfolioSpec>>() {
        Ok(py_spec.inner.clone())
    } else {
        // Try to deserialize from dict
        pythonize::depythonize(spec)
            .map_err(|e| PyValueError::new_err(format!("Invalid portfolio spec: {}", e)))
    }
}

/// Extract a ScenarioSpec from a Python object.
fn extract_scenario_spec(spec: &Bound<'_, PyAny>) -> PyResult<ScenarioSpec> {
    if let Ok(py_spec) = spec.extract::<PyRef<PyScenarioSpec>>() {
        Ok(py_spec.inner.clone())
    } else {
        pythonize::depythonize(spec)
            .map_err(|e| PyValueError::new_err(format!("Invalid scenario spec: {}", e)))
    }
}

/// Extract a FinancialModelSpec from a Python object.
fn extract_statement_model(spec: &Bound<'_, PyAny>) -> PyResult<FinancialModelSpec> {
    if let Ok(py_spec) = spec.extract::<PyRef<PyFinancialModelSpec>>() {
        Ok(py_spec.inner.clone())
    } else {
        pythonize::depythonize(spec)
            .map_err(|e| PyValueError::new_err(format!("Invalid statement model: {}", e)))
    }
}

/// Extract a MetricRegistry from a Python object.
fn extract_metric_registry(registry: &Bound<'_, PyAny>) -> PyResult<MetricRegistry> {
    // Try extracting the PyMetricRegistry wrapper first
    if let Ok(py_reg) = registry.extract::<PyRef<PyMetricRegistry>>() {
        return Ok(py_reg.inner.clone());
    }
    // Fall back to deserializing from dict
    pythonize::depythonize(registry)
        .map_err(|e| PyValueError::new_err(format!("Invalid metric registry: {}", e)))
}

/// Register the store in the module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PySqliteStore>()?;
    Ok(vec!["SqliteStore".to_string()])
}
