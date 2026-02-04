//! Environment-based configuration helpers for `finstack-io`.

use crate::{BulkStore, LookbackStore, Store, TimeSeriesStore};
use crate::{Error, Result};
use std::path::PathBuf;

/// Available IO backends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoBackend {
    /// SQLite backend.
    Sqlite,
    /// Postgres backend.
    Postgres,
    /// Turso backend.
    Turso,
}

impl IoBackend {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "sqlite" => Some(Self::Sqlite),
            "postgres" | "postgresql" => Some(Self::Postgres),
            "turso" => Some(Self::Turso),
            _ => None,
        }
    }
}

/// Environment-based configuration for IO backends.
#[derive(Clone, Debug)]
pub struct FinstackIoConfig {
    /// Selected backend.
    pub backend: IoBackend,
    /// SQLite database path (required when backend is SQLite).
    pub sqlite_path: Option<PathBuf>,
    /// Postgres connection URL (required when backend is Postgres).
    pub postgres_url: Option<String>,
    /// Turso database path (required when backend is Turso).
    pub turso_path: Option<PathBuf>,
}

impl FinstackIoConfig {
    /// Load configuration from environment variables.
    ///
    /// Reads:
    /// - `FINSTACK_IO_BACKEND` (default: `sqlite`)
    /// - `FINSTACK_SQLITE_PATH`
    /// - `FINSTACK_POSTGRES_URL`
    /// - `FINSTACK_TURSO_PATH`
    pub fn from_env() -> Result<Self> {
        let backend = std::env::var("FINSTACK_IO_BACKEND")
            .ok()
            .and_then(|value| IoBackend::parse(&value))
            .unwrap_or(IoBackend::Sqlite);

        let sqlite_path = std::env::var("FINSTACK_SQLITE_PATH")
            .ok()
            .map(PathBuf::from);
        let postgres_url = std::env::var("FINSTACK_POSTGRES_URL").ok();
        let turso_path = std::env::var("FINSTACK_TURSO_PATH").ok().map(PathBuf::from);

        Ok(Self {
            backend,
            sqlite_path,
            postgres_url,
            turso_path,
        })
    }
}

/// A concrete store handle resolved from configuration.
#[derive(Clone, Debug)]
pub enum StoreHandle {
    /// SQLite-backed store.
    #[cfg(feature = "sqlite")]
    Sqlite(crate::sqlite::SqliteStore),
    /// Postgres-backed store.
    #[cfg(feature = "postgres")]
    Postgres(crate::postgres::PostgresStore),
    /// Turso-backed store.
    #[cfg(feature = "turso")]
    Turso(crate::turso::TursoStore),
}

impl Store for StoreHandle {
    fn put_market_context(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
        context: &finstack_core::market_data::context::MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_market_context(market_id, as_of, context, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => {
                store.put_market_context(market_id, as_of, context, meta)
            }
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_market_context(market_id, as_of, context, meta),
        }
    }

    fn get_market_context(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<finstack_core::market_data::context::MarketContext>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_market_context(market_id, as_of),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_market_context(market_id, as_of),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_market_context(market_id, as_of),
        }
    }

    fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &finstack_valuations::instruments::InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_instrument(instrument_id, instrument, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_instrument(instrument_id, instrument, meta),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_instrument(instrument_id, instrument, meta),
        }
    }

    fn get_instrument(
        &self,
        instrument_id: &str,
    ) -> Result<Option<finstack_valuations::instruments::InstrumentJson>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_instrument(instrument_id),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_instrument(instrument_id),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_instrument(instrument_id),
        }
    }

    fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<std::collections::HashMap<String, finstack_valuations::instruments::InstrumentJson>>
    {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_instruments_batch(instrument_ids),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_instruments_batch(instrument_ids),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_instruments_batch(instrument_ids),
        }
    }

    fn list_instruments(&self) -> Result<Vec<String>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_instruments(),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_instruments(),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_instruments(),
        }
    }

    fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
        spec: &finstack_portfolio::PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_portfolio_spec(portfolio_id, as_of, spec, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => {
                store.put_portfolio_spec(portfolio_id, as_of, spec, meta)
            }
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_portfolio_spec(portfolio_id, as_of, spec, meta),
        }
    }

    fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<finstack_portfolio::PortfolioSpec>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_portfolio_spec(portfolio_id, as_of),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_portfolio_spec(portfolio_id, as_of),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_portfolio_spec(portfolio_id, as_of),
        }
    }

    fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &finstack_scenarios::ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_scenario(scenario_id, spec, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_scenario(scenario_id, spec, meta),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_scenario(scenario_id, spec, meta),
        }
    }

    fn get_scenario(&self, scenario_id: &str) -> Result<Option<finstack_scenarios::ScenarioSpec>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_scenario(scenario_id),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_scenario(scenario_id),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_scenario(scenario_id),
        }
    }

    fn list_scenarios(&self) -> Result<Vec<String>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_scenarios(),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_scenarios(),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_scenarios(),
        }
    }

    fn put_statement_model(
        &self,
        model_id: &str,
        spec: &finstack_statements::FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_statement_model(model_id, spec, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_statement_model(model_id, spec, meta),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_statement_model(model_id, spec, meta),
        }
    }

    fn get_statement_model(
        &self,
        model_id: &str,
    ) -> Result<Option<finstack_statements::FinancialModelSpec>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_statement_model(model_id),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_statement_model(model_id),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_statement_model(model_id),
        }
    }

    fn list_statement_models(&self) -> Result<Vec<String>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_statement_models(),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_statement_models(),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_statement_models(),
        }
    }

    fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &finstack_statements::registry::MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_metric_registry(namespace, registry, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_metric_registry(namespace, registry, meta),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_metric_registry(namespace, registry, meta),
        }
    }

    fn get_metric_registry(
        &self,
        namespace: &str,
    ) -> Result<Option<finstack_statements::registry::MetricRegistry>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_metric_registry(namespace),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_metric_registry(namespace),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_metric_registry(namespace),
        }
    }

    fn list_metric_registries(&self) -> Result<Vec<String>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_metric_registries(),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_metric_registries(),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_metric_registries(),
        }
    }

    fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.delete_metric_registry(namespace),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.delete_metric_registry(namespace),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.delete_metric_registry(namespace),
        }
    }
}

impl BulkStore for StoreHandle {
    fn put_instruments_batch(
        &self,
        instruments: &[(
            &str,
            &finstack_valuations::instruments::InstrumentJson,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_instruments_batch(instruments),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_instruments_batch(instruments),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_instruments_batch(instruments),
        }
    }

    fn put_market_contexts_batch(
        &self,
        contexts: &[(
            &str,
            finstack_core::dates::Date,
            &finstack_core::market_data::context::MarketContext,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_market_contexts_batch(contexts),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_market_contexts_batch(contexts),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_market_contexts_batch(contexts),
        }
    }

    fn put_portfolios_batch(
        &self,
        portfolios: &[(
            &str,
            finstack_core::dates::Date,
            &finstack_portfolio::PortfolioSpec,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_portfolios_batch(portfolios),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_portfolios_batch(portfolios),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_portfolios_batch(portfolios),
        }
    }
}

impl LookbackStore for StoreHandle {
    fn list_market_contexts(
        &self,
        market_id: &str,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
    ) -> Result<Vec<crate::MarketContextSnapshot>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_market_contexts(market_id, start, end),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_market_contexts(market_id, start, end),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_market_contexts(market_id, start, end),
        }
    }

    fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::MarketContextSnapshot>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => {
                store.latest_market_context_on_or_before(market_id, as_of)
            }
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => {
                store.latest_market_context_on_or_before(market_id, as_of)
            }
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.latest_market_context_on_or_before(market_id, as_of),
        }
    }

    fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
    ) -> Result<Vec<crate::PortfolioSnapshot>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_portfolios(portfolio_id, start, end),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_portfolios(portfolio_id, start, end),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_portfolios(portfolio_id, start, end),
        }
    }

    fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::PortfolioSnapshot>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.latest_portfolio_on_or_before(portfolio_id, as_of),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => {
                store.latest_portfolio_on_or_before(portfolio_id, as_of)
            }
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.latest_portfolio_on_or_before(portfolio_id, as_of),
        }
    }
}

impl TimeSeriesStore for StoreHandle {
    fn put_series_meta(
        &self,
        key: &crate::SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_series_meta(key, meta),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_series_meta(key, meta),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_series_meta(key, meta),
        }
    }

    fn get_series_meta(&self, key: &crate::SeriesKey) -> Result<Option<serde_json::Value>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_series_meta(key),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_series_meta(key),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_series_meta(key),
        }
    }

    fn list_series(&self, namespace: &str, kind: crate::SeriesKind) -> Result<Vec<String>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.list_series(namespace, kind),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.list_series(namespace, kind),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.list_series(namespace, kind),
        }
    }

    fn put_points_batch(
        &self,
        key: &crate::SeriesKey,
        points: &[crate::TimeSeriesPoint],
    ) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.put_points_batch(key, points),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.put_points_batch(key, points),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.put_points_batch(key, points),
        }
    }

    fn get_points_range(
        &self,
        key: &crate::SeriesKey,
        start: time::OffsetDateTime,
        end: time::OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<crate::TimeSeriesPoint>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.get_points_range(key, start, end, limit),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.get_points_range(key, start, end, limit),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.get_points_range(key, start, end, limit),
        }
    }

    fn latest_point_on_or_before(
        &self,
        key: &crate::SeriesKey,
        ts: time::OffsetDateTime,
    ) -> Result<Option<crate::TimeSeriesPoint>> {
        match self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.latest_point_on_or_before(key, ts),
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.latest_point_on_or_before(key, ts),
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.latest_point_on_or_before(key, ts),
        }
    }
}

/// Open a store using the current environment configuration.
pub fn open_store_from_env() -> Result<StoreHandle> {
    let config = FinstackIoConfig::from_env()?;
    match config.backend {
        IoBackend::Sqlite => {
            #[cfg(feature = "sqlite")]
            {
                let path = config.sqlite_path.ok_or_else(|| {
                    Error::Invariant("FINSTACK_SQLITE_PATH is required for sqlite backend".into())
                })?;
                Ok(StoreHandle::Sqlite(crate::sqlite::SqliteStore::open(path)?))
            }
            #[cfg(not(feature = "sqlite"))]
            {
                Err(Error::Invariant(
                    "sqlite backend requested but feature is disabled".into(),
                ))
            }
        }
        IoBackend::Postgres => {
            #[cfg(feature = "postgres")]
            {
                let url = config.postgres_url.ok_or_else(|| {
                    Error::Invariant(
                        "FINSTACK_POSTGRES_URL is required for postgres backend".into(),
                    )
                })?;
                Ok(StoreHandle::Postgres(
                    crate::postgres::PostgresStore::connect(&url)?,
                ))
            }
            #[cfg(not(feature = "postgres"))]
            {
                Err(Error::Invariant(
                    "postgres backend requested but feature is disabled".into(),
                ))
            }
        }
        IoBackend::Turso => {
            #[cfg(feature = "turso")]
            {
                let path = config.turso_path.ok_or_else(|| {
                    Error::Invariant("FINSTACK_TURSO_PATH is required for turso backend".into())
                })?;
                Ok(StoreHandle::Turso(crate::turso::TursoStore::open(path)?))
            }
            #[cfg(not(feature = "turso"))]
            {
                Err(Error::Invariant(
                    "turso backend requested but feature is disabled".into(),
                ))
            }
        }
    }
}
