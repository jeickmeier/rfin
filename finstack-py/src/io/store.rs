//! Python bindings for the unified Store.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::io::error::map_io_error;
use crate::io::types::{PyMarketContextSnapshot, PyPortfolioSnapshot, PyPortfolioSpec};
use crate::portfolio::positions::PyPortfolio;
use crate::scenarios::spec::PyScenarioSpec;
use crate::statements::registry::PyMetricRegistry;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_core::market_data::context::MarketContext;
use finstack_io::{
    BulkStore, LookbackStore, SeriesKey, SeriesKind, Store, StoreHandle, TimeSeriesPoint,
    TimeSeriesStore,
};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{
    PyAny, PyDate, PyDateAccess, PyDateTime, PyList, PyModule, PyTimeAccess, PyTuple,
    PyTzInfoAccess,
};
use pyo3::Bound;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use time::{Date as TimeDate, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use tokio::runtime::Runtime;

/// A unified persistence store for Finstack domain objects.
///
/// This store provides CRUD operations for market contexts, instruments, portfolios,
/// scenarios, statement models, and metric registries. All operations are atomic
/// and idempotent (upserts).
///
/// The store supports multiple backends:
/// - **SQLite**: Embedded, transactional, zero-config (default)
/// - **PostgreSQL**: Production-grade relational database (requires `postgres` feature)
/// - **Turso**: SQLite-compatible with native JSON support (requires `turso` feature)
///
/// Examples:
///     >>> from finstack.io import Store
///     >>> from datetime import date
///     >>> # Open a SQLite database
///     >>> store = Store.open_sqlite("finstack.db")
///     >>> # Or use Turso
///     >>> store = Store.open_turso("finstack.db")
///     >>> # Or connect to PostgreSQL
///     >>> store = Store.connect_postgres("postgresql://user:pass@localhost/db")
///     >>> # Or auto-detect from environment
///     >>> store = Store.from_env()
///     >>> # All backends have the same API
///     >>> store.put_market_context("USD_MKT", date(2024, 1, 1), market)
#[pyclass(module = "finstack.io", name = "Store")]
pub struct PyStore {
    inner: StoreHandle,
    backend_name: &'static str,
    runtime: PyRuntime,
}

#[derive(Clone)]
struct PyRuntime {
    inner: Arc<Runtime>,
}

impl PyRuntime {
    fn new(inner: Arc<Runtime>) -> Self {
        Self { inner }
    }

    fn block_on<T, F>(&self, fut: F) -> T
    where
        F: Future<Output = T> + Send,
        T: Send,
    {
        Python::attach(|py| py.detach(|| self.inner.block_on(fut)))
    }
}

/// Create or reuse a shared tokio runtime for async operations.
fn create_runtime() -> PyResult<PyRuntime> {
    static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
    if let Some(runtime) = RUNTIME.get() {
        return Ok(PyRuntime::new(runtime.clone()));
    }

    let runtime = Runtime::new().map(Arc::new).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create async runtime: {e}"))
    })?;
    if RUNTIME.set(runtime.clone()).is_err() {
        let shared = RUNTIME.get().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "Runtime initialization race: shared runtime missing",
            )
        })?;
        return Ok(PyRuntime::new(shared.clone()));
    }
    Ok(PyRuntime::new(runtime))
}

#[pymethods]
impl PyStore {
    /// Open or create a SQLite database at the given path.
    ///
    /// The database schema is automatically created and migrated on open.
    /// Parent directories are created if they don't exist.
    ///
    /// Args:
    ///     path: Path to the SQLite database file. Use `:memory:` for an
    ///         in-memory database.
    ///
    /// Returns:
    ///     Store: The opened store instance.
    ///
    /// Raises:
    ///     IoError: If the database cannot be opened or migrated.
    ///
    /// Examples:
    ///     >>> store = Store.open_sqlite("data/finstack.db")
    ///     >>> store = Store.open_sqlite(":memory:")  # In-memory database
    #[staticmethod]
    #[pyo3(text_signature = "(path)")]
    fn open_sqlite(path: &str) -> PyResult<Self> {
        #[cfg(feature = "sqlite")]
        {
            let runtime = create_runtime()?;
            let store = runtime
                .block_on(finstack_io::SqliteStore::open(PathBuf::from(path)))
                .map_err(map_io_error)?;
            Ok(Self {
                inner: StoreHandle::Sqlite(store),
                backend_name: "sqlite",
                runtime,
            })
        }
        #[cfg(not(feature = "sqlite"))]
        {
            let _ = path;
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "SQLite backend is not available in this build",
            ))
        }
    }

    /// Connect to a PostgreSQL database.
    ///
    /// Args:
    ///     url: PostgreSQL connection URL (e.g., "postgresql://user:pass@host/db").
    ///
    /// Returns:
    ///     Store: The connected store instance.
    ///
    /// Raises:
    ///     IoError: If the connection fails or migration fails.
    ///
    /// Examples:
    ///     >>> store = Store.connect_postgres("postgresql://localhost/finstack")
    #[staticmethod]
    #[pyo3(text_signature = "(url)")]
    fn connect_postgres(url: &str) -> PyResult<Self> {
        #[cfg(feature = "postgres")]
        {
            let runtime = create_runtime()?;
            let store = runtime
                .block_on(finstack_io::PostgresStore::connect(url))
                .map_err(map_io_error)?;
            Ok(Self {
                inner: StoreHandle::Postgres(store),
                backend_name: "postgres",
                runtime,
            })
        }
        #[cfg(not(feature = "postgres"))]
        {
            let _ = url;
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "PostgreSQL backend is not available in this build",
            ))
        }
    }

    /// Open or create a Turso database at the given path.
    ///
    /// Turso is an in-process SQL database engine compatible with SQLite,
    /// offering native JSON support and modern async I/O.
    ///
    /// Args:
    ///     path: Path to the database file. Use `:memory:` for an
    ///         in-memory database.
    ///
    /// Returns:
    ///     Store: The opened store instance.
    ///
    /// Raises:
    ///     IoError: If the database cannot be opened or migrated.
    ///
    /// Examples:
    ///     >>> store = Store.open_turso("data/finstack.db")
    #[staticmethod]
    #[pyo3(text_signature = "(path)")]
    fn open_turso(path: &str) -> PyResult<Self> {
        #[cfg(feature = "turso")]
        {
            let runtime = create_runtime()?;
            let store = runtime
                .block_on(finstack_io::TursoStore::open(PathBuf::from(path)))
                .map_err(map_io_error)?;
            Ok(Self {
                inner: StoreHandle::Turso(store),
                backend_name: "turso",
                runtime,
            })
        }
        #[cfg(not(feature = "turso"))]
        {
            let _ = path;
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Turso backend is not available in this build",
            ))
        }
    }

    /// Open a store based on environment variables.
    ///
    /// Environment Variables:
    ///     FINSTACK_IO_BACKEND: Backend to use ("sqlite", "postgres", or "turso").
    ///         Defaults to "sqlite".
    ///     FINSTACK_SQLITE_PATH: Path to SQLite database file.
    ///         Required when FINSTACK_IO_BACKEND="sqlite".
    ///     FINSTACK_POSTGRES_URL: PostgreSQL connection URL.
    ///         Required when FINSTACK_IO_BACKEND="postgres".
    ///     FINSTACK_TURSO_PATH: Path to Turso database file.
    ///         Required when FINSTACK_IO_BACKEND="turso".
    ///
    /// Returns:
    ///     Store: The opened store instance.
    ///
    /// Raises:
    ///     IoError: If the store cannot be opened.
    ///     ValueError: If required environment variables are missing.
    ///
    /// Examples:
    ///     >>> import os
    ///     >>> os.environ["FINSTACK_IO_BACKEND"] = "sqlite"
    ///     >>> os.environ["FINSTACK_SQLITE_PATH"] = "data/finstack.db"
    ///     >>> store = Store.from_env()
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn from_env() -> PyResult<Self> {
        let runtime = create_runtime()?;
        let handle = runtime
            .block_on(finstack_io::open_store_from_env())
            .map_err(map_io_error)?;
        let backend_name = store_handle_backend_name(&handle);
        Ok(Self {
            inner: handle,
            backend_name,
            runtime,
        })
    }

    /// Get the backend type name.
    ///
    /// Returns:
    ///     str: One of "sqlite", "postgres", or "turso".
    #[getter]
    fn backend(&self) -> &'static str {
        self.backend_name
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
        self.runtime
            .block_on(self.inner.put_market_context(
                market_id,
                date,
                &context.inner,
                meta_json.as_ref(),
            ))
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
            .runtime
            .block_on(self.inner.get_market_context(market_id, date))
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
            .runtime
            .block_on(self.inner.get_market_context(market_id, date))
            .map_err(map_io_error)?
            .ok_or_else(|| {
                map_io_error(finstack_io::Error::not_found(
                    "market context",
                    format!("{market_id}@{date}"),
                ))
            })?;
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
    ///     >>> instrument = {"type": "equity", "spec": {...}}
    ///     >>> store.put_instrument("EQUITY_SPY", instrument)
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
        self.runtime
            .block_on(
                self.inner
                    .put_instrument(instrument_id, &instr, meta_json.as_ref()),
            )
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
    ///     >>> instr = store.get_instrument("EQUITY_SPY")
    ///     >>> if instr:
    ///     ...     print(instr["type"])
    #[pyo3(text_signature = "($self, instrument_id)")]
    fn get_instrument(&self, py: Python<'_>, instrument_id: &str) -> PyResult<Option<Py<PyAny>>> {
        let result = self
            .runtime
            .block_on(self.inner.get_instrument(instrument_id))
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

    /// Retrieve multiple instruments by ID in a single query.
    ///
    /// Args:
    ///     instrument_ids: List of instrument identifiers.
    ///
    /// Returns:
    ///     dict[str, dict]: Map of instrument_id to instrument definition.
    ///                      Missing instruments are silently omitted.
    ///
    /// Examples:
    ///     >>> instruments = store.get_instruments_batch(["EQUITY_SPY", "EQUITY_QQQ"])
    ///     >>> for id, instr in instruments.items():
    ///     ...     print(f"{id}: {instr['type']}")
    #[pyo3(text_signature = "($self, instrument_ids)")]
    fn get_instruments_batch(
        &self,
        py: Python<'_>,
        instrument_ids: Vec<String>,
    ) -> PyResult<Py<PyAny>> {
        let result = self
            .runtime
            .block_on(self.inner.get_instruments_batch(&instrument_ids))
            .map_err(map_io_error)?;
        let py_dict = pythonize::pythonize(py, &result)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))?;
        Ok(py_dict.unbind())
    }

    /// List all stored instrument IDs.
    ///
    /// Returns:
    ///     list[str]: List of instrument IDs, sorted alphabetically.
    ///
    /// Examples:
    ///     >>> ids = store.list_instruments()
    ///     >>> print(f"Found {len(ids)} instruments")
    #[pyo3(text_signature = "($self)")]
    fn list_instruments(&self) -> PyResult<Vec<String>> {
        self.runtime
            .block_on(self.inner.list_instruments())
            .map_err(map_io_error)
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
        self.runtime
            .block_on(self.inner.put_portfolio_spec(
                portfolio_id,
                date,
                &portfolio_spec,
                meta_json.as_ref(),
            ))
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
            .runtime
            .block_on(self.inner.get_portfolio_spec(portfolio_id, date))
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
        // Use the Store trait's default implementation which properly hydrates instruments
        let portfolio = self
            .runtime
            .block_on(self.inner.load_portfolio(portfolio_id, date))
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
        // Use the Store trait's default implementation which properly hydrates instruments
        let (portfolio, market) = self
            .runtime
            .block_on(
                self.inner
                    .load_portfolio_with_market(portfolio_id, market_id, date),
            )
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
        self.runtime
            .block_on(
                self.inner
                    .put_scenario(scenario_id, &scenario_spec, meta_json.as_ref()),
            )
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
        let result = self
            .runtime
            .block_on(self.inner.get_scenario(scenario_id))
            .map_err(map_io_error)?;
        Ok(result.map(PyScenarioSpec::from_inner))
    }

    /// List all stored scenario IDs.
    ///
    /// Returns:
    ///     list[str]: List of scenario IDs, sorted alphabetically.
    ///
    /// Examples:
    ///     >>> ids = store.list_scenarios()
    ///     >>> print(f"Found {len(ids)} scenarios")
    #[pyo3(text_signature = "($self)")]
    fn list_scenarios(&self) -> PyResult<Vec<String>> {
        self.runtime
            .block_on(self.inner.list_scenarios())
            .map_err(map_io_error)
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
        self.runtime
            .block_on(
                self.inner
                    .put_statement_model(model_id, &model_spec, meta_json.as_ref()),
            )
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
            .runtime
            .block_on(self.inner.get_statement_model(model_id))
            .map_err(map_io_error)?;
        Ok(result.map(PyFinancialModelSpec::new))
    }

    /// List all stored statement model IDs.
    ///
    /// Returns:
    ///     list[str]: List of model IDs, sorted alphabetically.
    ///
    /// Examples:
    ///     >>> ids = store.list_statement_models()
    ///     >>> print(f"Found {len(ids)} models")
    #[pyo3(text_signature = "($self)")]
    fn list_statement_models(&self) -> PyResult<Vec<String>> {
        self.runtime
            .block_on(self.inner.list_statement_models())
            .map_err(map_io_error)
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
        self.runtime
            .block_on(
                self.inner
                    .put_metric_registry(namespace, &reg, meta_json.as_ref()),
            )
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
            .runtime
            .block_on(self.inner.get_metric_registry(namespace))
            .map_err(map_io_error)?;
        Ok(result.map(PyMetricRegistry::new))
    }

    /// Load a metric registry, raising an error if not found.
    #[pyo3(text_signature = "($self, namespace)")]
    fn load_metric_registry(&self, namespace: &str) -> PyResult<PyMetricRegistry> {
        let reg = self
            .runtime
            .block_on(self.inner.get_metric_registry(namespace))
            .map_err(map_io_error)?
            .ok_or_else(|| {
                map_io_error(finstack_io::Error::not_found("metric registry", namespace))
            })?;
        Ok(PyMetricRegistry::new(reg))
    }

    /// List all metric registry namespaces.
    ///
    /// Returns:
    ///     list[str]: List of namespace names.
    #[pyo3(text_signature = "($self)")]
    fn list_metric_registries(&self) -> PyResult<Vec<String>> {
        self.runtime
            .block_on(self.inner.list_metric_registries())
            .map_err(map_io_error)
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
        self.runtime
            .block_on(self.inner.delete_metric_registry(namespace))
            .map_err(map_io_error)
    }

    // =========================================================================
    // Time Series Operations
    // =========================================================================

    /// Store metadata for a time-series key.
    ///
    /// Args:
    ///     namespace: Logical namespace for the series.
    ///     kind: Series kind (quote, metric, result, pnl, risk).
    ///     series_id: Series identifier.
    ///     meta: Optional metadata dict.
    #[pyo3(signature = (namespace, kind, series_id, meta=None))]
    #[pyo3(text_signature = "($self, namespace, kind, series_id, meta=None)")]
    fn put_series_meta(
        &self,
        namespace: &str,
        kind: &str,
        series_id: &str,
        meta: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let kind = parse_series_kind(kind)?;
        let meta_json = extract_meta(meta)?;
        let key = SeriesKey::new(namespace, series_id, kind);
        self.runtime
            .block_on(self.inner.put_series_meta(&key, meta_json.as_ref()))
            .map_err(map_io_error)
    }

    /// Retrieve metadata for a time-series key.
    #[pyo3(text_signature = "($self, namespace, kind, series_id)")]
    fn get_series_meta(
        &self,
        py: Python<'_>,
        namespace: &str,
        kind: &str,
        series_id: &str,
    ) -> PyResult<Option<Py<PyAny>>> {
        let kind = parse_series_kind(kind)?;
        let key = SeriesKey::new(namespace, series_id, kind);
        let meta = self
            .runtime
            .block_on(self.inner.get_series_meta(&key))
            .map_err(map_io_error)?;
        match meta {
            Some(value) => {
                let py_obj = pythonize::pythonize(py, &value)
                    .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
                Ok(Some(py_obj.unbind()))
            }
            None => Ok(None),
        }
    }

    /// List series IDs for a namespace and kind.
    #[pyo3(text_signature = "($self, namespace, kind)")]
    fn list_series(&self, namespace: &str, kind: &str) -> PyResult<Vec<String>> {
        let kind = parse_series_kind(kind)?;
        self.runtime
            .block_on(self.inner.list_series(namespace, kind))
            .map_err(map_io_error)
    }

    /// Store multiple time-series points in a single transaction.
    #[pyo3(text_signature = "($self, namespace, kind, series_id, points)")]
    fn put_points_batch(
        &self,
        namespace: &str,
        kind: &str,
        series_id: &str,
        points: &Bound<'_, PyList>,
    ) -> PyResult<()> {
        let kind = parse_series_kind(kind)?;
        let key = SeriesKey::new(namespace, series_id, kind);
        let mut batch = Vec::with_capacity(points.len());
        for item in points.iter() {
            let tuple = item.cast::<PyTuple>()?;
            if tuple.len() < 1 {
                return Err(PyValueError::new_err(
                    "Point tuples must include a timestamp",
                ));
            }
            let ts = py_to_offset_datetime(&tuple.get_item(0)?)?;
            let value = if tuple.len() > 1 && !tuple.get_item(1)?.is_none() {
                Some(tuple.get_item(1)?.extract::<f64>()?)
            } else {
                None
            };
            let payload = if tuple.len() > 2 && !tuple.get_item(2)?.is_none() {
                Some(
                    pythonize::depythonize(&tuple.get_item(2)?)
                        .map_err(|e| PyValueError::new_err(format!("Invalid payload: {e}")))?,
                )
            } else {
                None
            };
            let meta = if tuple.len() > 3 && !tuple.get_item(3)?.is_none() {
                Some(
                    pythonize::depythonize(&tuple.get_item(3)?)
                        .map_err(|e| PyValueError::new_err(format!("Invalid meta: {e}")))?,
                )
            } else {
                None
            };
            batch.push(TimeSeriesPoint {
                ts,
                value,
                payload,
                meta,
            });
        }
        self.runtime
            .block_on(self.inner.put_points_batch(&key, &batch))
            .map_err(map_io_error)
    }

    /// Retrieve points in a time range.
    #[pyo3(signature = (namespace, kind, series_id, start, end, limit=None))]
    #[pyo3(text_signature = "($self, namespace, kind, series_id, start, end, limit=None)")]
    #[allow(clippy::too_many_arguments)]
    fn get_points_range(
        &self,
        py: Python<'_>,
        namespace: &str,
        kind: &str,
        series_id: &str,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        limit: Option<usize>,
    ) -> PyResult<Vec<Py<PyAny>>> {
        let kind = parse_series_kind(kind)?;
        let key = SeriesKey::new(namespace, series_id, kind);
        let start = py_to_offset_datetime(start)?;
        let end = py_to_offset_datetime(end)?;
        let points = self
            .runtime
            .block_on(self.inner.get_points_range(&key, start, end, limit))
            .map_err(map_io_error)?;
        points
            .iter()
            .map(|point| time_series_point_to_py(py, point))
            .collect()
    }

    /// Get the latest point on or before a timestamp.
    #[pyo3(text_signature = "($self, namespace, kind, series_id, ts)")]
    fn latest_point_on_or_before(
        &self,
        py: Python<'_>,
        namespace: &str,
        kind: &str,
        series_id: &str,
        ts: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let kind = parse_series_kind(kind)?;
        let key = SeriesKey::new(namespace, series_id, kind);
        let ts = py_to_offset_datetime(ts)?;
        let point = self
            .runtime
            .block_on(self.inner.latest_point_on_or_before(&key, ts))
            .map_err(map_io_error)?;
        match point {
            Some(value) => Ok(Some(time_series_point_to_py(py, &value)?)),
            None => Ok(None),
        }
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
    ///     ...     ("EQUITY_SPY", {"type": "equity", "spec": {...}}),
    ///     ...     ("EQUITY_QQQ", {"type": "equity", "spec": {...}}),
    ///     ... ]
    ///     >>> store.put_instruments_batch(instruments)
    #[pyo3(text_signature = "($self, instruments)")]
    fn put_instruments_batch(&self, instruments: &Bound<'_, PyList>) -> PyResult<()> {
        let mut batch: Vec<(String, InstrumentJson, Option<serde_json::Value>)> = Vec::new();

        for item in instruments.iter() {
            let tuple = item.cast::<pyo3::types::PyTuple>()?;
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

        self.runtime
            .block_on(self.inner.put_instruments_batch(&refs))
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
            let tuple = item.cast::<pyo3::types::PyTuple>()?;
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

        self.runtime
            .block_on(self.inner.put_market_contexts_batch(&refs))
            .map_err(map_io_error)
    }

    /// Store multiple portfolio specs in a single transaction.
    ///
    /// Args:
    ///     portfolios: List of (portfolio_id, as_of, spec) tuples,
    ///                 or (portfolio_id, as_of, spec, meta) tuples.
    #[pyo3(text_signature = "($self, portfolios)")]
    fn put_portfolios_batch(&self, portfolios: &Bound<'_, PyList>) -> PyResult<()> {
        let mut batch: Vec<(
            String,
            finstack_core::dates::Date,
            PortfolioSpec,
            Option<serde_json::Value>,
        )> = Vec::new();

        for item in portfolios.iter() {
            let tuple = item.cast::<pyo3::types::PyTuple>()?;
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

        self.runtime
            .block_on(self.inner.put_portfolios_batch(&refs))
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
            .runtime
            .block_on(
                self.inner
                    .list_market_contexts(market_id, start_date, end_date),
            )
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
            .runtime
            .block_on(
                self.inner
                    .latest_market_context_on_or_before(market_id, date),
            )
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
            .runtime
            .block_on(
                self.inner
                    .list_portfolios(portfolio_id, start_date, end_date),
            )
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
            .runtime
            .block_on(self.inner.latest_portfolio_on_or_before(portfolio_id, date))
            .map_err(map_io_error)?;
        Ok(result.map(PyPortfolioSnapshot::new))
    }

    fn __repr__(&self) -> String {
        format!("Store(backend='{}')", self.backend_name)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get the backend name from a StoreHandle.
fn store_handle_backend_name(handle: &StoreHandle) -> &'static str {
    match handle {
        #[cfg(feature = "sqlite")]
        StoreHandle::Sqlite(_) => "sqlite",
        #[cfg(feature = "postgres")]
        StoreHandle::Postgres(_) => "postgres",
        #[cfg(feature = "turso")]
        StoreHandle::Turso(_) => "turso",
    }
}

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

fn parse_series_kind(kind: &str) -> PyResult<SeriesKind> {
    SeriesKind::try_parse(kind).ok_or_else(|| {
        PyValueError::new_err(format!(
            "Invalid series kind '{kind}'. Expected: quote, metric, result, pnl, risk."
        ))
    })
}

fn py_to_offset_datetime(obj: &Bound<'_, PyAny>) -> PyResult<OffsetDateTime> {
    if let Ok(dt) = obj.cast::<PyDateTime>() {
        let date = TimeDate::from_calendar_date(
            dt.get_year(),
            Month::try_from(dt.get_month()).map_err(|_| PyValueError::new_err("Invalid month"))?,
            dt.get_day(),
        )
        .map_err(|e| PyValueError::new_err(format!("Invalid date: {e}")))?;
        let time = Time::from_hms_micro(
            dt.get_hour(),
            dt.get_minute(),
            dt.get_second(),
            dt.get_microsecond(),
        )
        .map_err(|e| PyValueError::new_err(format!("Invalid time: {e}")))?;
        let naive = PrimitiveDateTime::new(date, time);

        // Extract timezone offset if present, otherwise assume UTC
        let offset = if let Some(tzinfo) = dt.get_tzinfo() {
            // Try to get utcoffset() from tzinfo
            if let Ok(utc_offset) = tzinfo.call_method1("utcoffset", (dt,)) {
                if !utc_offset.is_none() {
                    // utcoffset returns a timedelta, total_seconds() returns a float
                    let total_seconds_f64: f64 =
                        utc_offset.call_method0("total_seconds")?.extract()?;
                    let total_seconds_i32 = total_seconds_f64 as i32;
                    UtcOffset::from_whole_seconds(total_seconds_i32)
                        .map_err(|e| PyValueError::new_err(format!("Invalid UTC offset: {e}")))?
                } else {
                    UtcOffset::UTC
                }
            } else {
                UtcOffset::UTC
            }
        } else {
            UtcOffset::UTC
        };

        Ok(naive.assume_offset(offset))
    } else if let Ok(d) = obj.cast::<PyDate>() {
        let date = TimeDate::from_calendar_date(
            d.get_year(),
            Month::try_from(d.get_month()).map_err(|_| PyValueError::new_err("Invalid month"))?,
            d.get_day(),
        )
        .map_err(|e| PyValueError::new_err(format!("Invalid date: {e}")))?;
        let naive = PrimitiveDateTime::new(date, Time::MIDNIGHT);
        Ok(naive.assume_offset(UtcOffset::UTC))
    } else {
        Err(PyValueError::new_err(
            "Expected datetime.datetime or datetime.date",
        ))
    }
}

fn offset_datetime_to_py(py: Python<'_>, dt: OffsetDateTime) -> PyResult<Py<PyAny>> {
    // Convert to UTC for consistent output
    let dt_utc = dt.to_offset(UtcOffset::UTC);
    let date = dt_utc.date();
    let time = dt_utc.time();

    // Import datetime.timezone.utc for timezone-aware output
    let datetime_module = py.import("datetime")?;
    let timezone_utc = datetime_module.getattr("timezone")?.getattr("utc")?;
    let timezone_utc_tzinfo = timezone_utc.cast::<pyo3::types::PyTzInfo>()?;

    let py_dt = PyDateTime::new(
        py,
        date.year(),
        date.month() as u8,
        date.day(),
        time.hour(),
        time.minute(),
        time.second(),
        time.microsecond(),
        Some(timezone_utc_tzinfo),
    )?;
    Ok(py_dt.unbind().into())
}

fn time_series_point_to_py(py: Python<'_>, point: &TimeSeriesPoint) -> PyResult<Py<PyAny>> {
    let ts = offset_datetime_to_py(py, point.ts)?;
    let value = pythonize::pythonize(py, &point.value)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize value: {e}")))?
        .unbind();
    let payload: Py<PyAny> = match &point.payload {
        Some(value) => pythonize::pythonize(py, value)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize payload: {e}")))?
            .unbind(),
        None => py.None(),
    };
    let meta: Py<PyAny> = match &point.meta {
        Some(value) => pythonize::pythonize(py, value)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize meta: {e}")))?
            .unbind(),
        None => py.None(),
    };
    let tuple = PyTuple::new(py, [ts, value, payload, meta])?;
    Ok(tuple.unbind().into())
}

/// Register the store in the module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyStore>()?;
    Ok(vec!["Store".to_string()])
}
