//! Environment-based configuration helpers for `finstack-io`.
//!
//! This module provides [`FinstackIoConfig`] for loading backend selection from
//! environment variables, [`StoreHandle`] as a concrete enum dispatching to the
//! selected backend, and [`open_store_from_env`] as a one-liner to go from
//! environment variables to a ready-to-use store.
//!
//! # Environment Variables
//!
//! | Variable | Default | Description |
//! |----------|---------|-------------|
//! | `FINSTACK_IO_BACKEND` | `sqlite` | Backend to use: `sqlite`, `postgres`, or `turso` |
//! | `FINSTACK_SQLITE_PATH` | — | Path to SQLite database file |
//! | `FINSTACK_POSTGRES_URL` | — | Postgres connection URL |
//! | `FINSTACK_TURSO_PATH` | — | Path to Turso database file |

use crate::{BulkStore, LookbackStore, Store, TimeSeriesStore};
use crate::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Internal macro to dispatch `StoreHandle` methods to the underlying backend.
///
/// This reduces repetitive match arms across trait implementations.
/// Usage: `dispatch_store!(self, method_name, arg1, arg2, ...)`
macro_rules! dispatch_store {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            #[cfg(feature = "sqlite")]
            StoreHandle::Sqlite(store) => store.$method($($arg),*).await,
            #[cfg(feature = "postgres")]
            StoreHandle::Postgres(store) => store.$method($($arg),*).await,
            #[cfg(feature = "turso")]
            StoreHandle::Turso(store) => store.$method($($arg),*).await,
        }
    };
}

macro_rules! impl_store_forwarding {
    ($trait_name:ident {
        $(
            async fn $method:ident(
                &self
                $(, $arg:ident : $arg_ty:ty)*
                $(,)?
            ) -> $ret:ty;
        )*
    }) => {
        #[async_trait]
        impl $trait_name for StoreHandle {
            $(
                async fn $method(
                    &self,
                    $($arg : $arg_ty),*
                ) -> $ret {
                    dispatch_store!(self, $method $(, $arg)*)
                }
            )*
        }
    };
}

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
    /// - `FINSTACK_IO_BACKEND` — `"sqlite"` (default), `"postgres"`, or `"turso"`.
    /// - `FINSTACK_SQLITE_PATH` — Path for SQLite database file.
    /// - `FINSTACK_POSTGRES_URL` — Postgres connection URL.
    /// - `FINSTACK_TURSO_PATH` — Path for Turso database file.
    ///
    /// Missing path/URL variables are not errors here — they are checked
    /// lazily when [`open_store_from_env`] attempts to open the backend.
    /// An invalid `FINSTACK_IO_BACKEND` value is rejected immediately.
    pub fn from_env() -> Result<Self> {
        let backend = match std::env::var("FINSTACK_IO_BACKEND") {
            Ok(value) => IoBackend::parse(&value).ok_or_else(|| {
                Error::Invariant(format!(
                    "Invalid FINSTACK_IO_BACKEND: {value}. Supported values: sqlite, postgres, postgresql, turso"
                ))
            })?,
            Err(_) => IoBackend::Sqlite,
        };

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

impl_store_forwarding!(Store {
    async fn put_market_context(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
        context: &finstack_core::market_data::context::MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<finstack_core::market_data::context::MarketContext>>;
    async fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &finstack_valuations::instruments::InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_instrument(
        &self,
        instrument_id: &str,
    ) -> Result<Option<finstack_valuations::instruments::InstrumentJson>>;
    async fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<std::collections::HashMap<String, finstack_valuations::instruments::InstrumentJson>>;
    async fn list_instruments(&self) -> Result<Vec<String>>;
    async fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
        spec: &finstack_portfolio::PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<finstack_portfolio::PortfolioSpec>>;
    async fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &finstack_scenarios::ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_scenario(
        &self,
        scenario_id: &str,
    ) -> Result<Option<finstack_scenarios::ScenarioSpec>>;
    async fn list_scenarios(&self) -> Result<Vec<String>>;
    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &finstack_statements::FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_statement_model(
        &self,
        model_id: &str,
    ) -> Result<Option<finstack_statements::FinancialModelSpec>>;
    async fn list_statement_models(&self) -> Result<Vec<String>>;
    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &finstack_statements::registry::MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_metric_registry(
        &self,
        namespace: &str,
    ) -> Result<Option<finstack_statements::registry::MetricRegistry>>;
    async fn list_metric_registries(&self) -> Result<Vec<String>>;
    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool>;
});

impl_store_forwarding!(BulkStore {
    async fn put_instruments_batch(
        &self,
        instruments: &[(
            &str,
            &finstack_valuations::instruments::InstrumentJson,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()>;
    async fn put_market_contexts_batch(
        &self,
        contexts: &[(
            &str,
            finstack_core::dates::Date,
            &finstack_core::market_data::context::MarketContext,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()>;
    async fn put_portfolios_batch(
        &self,
        portfolios: &[(
            &str,
            finstack_core::dates::Date,
            &finstack_portfolio::PortfolioSpec,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()>;
});

impl_store_forwarding!(LookbackStore {
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
    ) -> Result<Vec<crate::MarketContextSnapshot>>;
    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::MarketContextSnapshot>>;
    async fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
    ) -> Result<Vec<crate::PortfolioSnapshot>>;
    async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::PortfolioSnapshot>>;
});

impl_store_forwarding!(TimeSeriesStore {
    async fn put_series_meta(
        &self,
        key: &crate::SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;
    async fn get_series_meta(
        &self,
        key: &crate::SeriesKey,
    ) -> Result<Option<serde_json::Value>>;
    async fn list_series(&self, namespace: &str, kind: crate::SeriesKind) -> Result<Vec<String>>;
    async fn put_points_batch(
        &self,
        key: &crate::SeriesKey,
        points: &[crate::TimeSeriesPoint],
    ) -> Result<()>;
    async fn get_points_range(
        &self,
        key: &crate::SeriesKey,
        start: time::OffsetDateTime,
        end: time::OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<crate::TimeSeriesPoint>>;
    async fn latest_point_on_or_before(
        &self,
        key: &crate::SeriesKey,
        ts: time::OffsetDateTime,
    ) -> Result<Option<crate::TimeSeriesPoint>>;
});

fn require_backend_config<T>(backend: &str, env_var: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| Error::Invariant(format!("{env_var} is required for {backend} backend")))
}

#[cfg(feature = "sqlite")]
async fn open_sqlite_store(path: Option<PathBuf>) -> Result<StoreHandle> {
    let path = require_backend_config("sqlite", "FINSTACK_SQLITE_PATH", path)?;
    Ok(StoreHandle::Sqlite(
        crate::sqlite::SqliteStore::open(path).await?,
    ))
}

#[cfg(not(feature = "sqlite"))]
async fn open_sqlite_store(_path: Option<PathBuf>) -> Result<StoreHandle> {
    Err(Error::Invariant(
        "sqlite backend requested but feature is disabled".into(),
    ))
}

#[cfg(feature = "postgres")]
async fn open_postgres_store(url: Option<String>) -> Result<StoreHandle> {
    let url = require_backend_config("postgres", "FINSTACK_POSTGRES_URL", url)?;
    Ok(StoreHandle::Postgres(
        crate::postgres::PostgresStore::connect(&url).await?,
    ))
}

#[cfg(not(feature = "postgres"))]
async fn open_postgres_store(_url: Option<String>) -> Result<StoreHandle> {
    Err(Error::Invariant(
        "postgres backend requested but feature is disabled".into(),
    ))
}

#[cfg(feature = "turso")]
async fn open_turso_store(path: Option<PathBuf>) -> Result<StoreHandle> {
    let path = require_backend_config("turso", "FINSTACK_TURSO_PATH", path)?;
    Ok(StoreHandle::Turso(
        crate::turso::TursoStore::open(path).await?,
    ))
}

#[cfg(not(feature = "turso"))]
async fn open_turso_store(_path: Option<PathBuf>) -> Result<StoreHandle> {
    Err(Error::Invariant(
        "turso backend requested but feature is disabled".into(),
    ))
}

/// Open a store using the current environment configuration.
///
/// Reads `FINSTACK_IO_BACKEND` to select the provider, then reads the
/// corresponding path/URL variable. Migrations run automatically.
///
/// # Errors
///
/// - [`Error::Invariant`] if the required path/URL
///   variable is not set for the selected backend, or if the selected feature
///   is not compiled in.
/// - Backend-specific errors from opening the connection.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_io::{open_store_from_env, Store, StoreHandle};
///
/// # async fn example() -> finstack_io::Result<()> {
/// // Assumes FINSTACK_IO_BACKEND and the corresponding path/URL are set
/// let store: StoreHandle = open_store_from_env().await?;
///
/// let ids = store.list_instruments().await?;
/// # Ok(())
/// # }
/// ```
pub async fn open_store_from_env() -> Result<StoreHandle> {
    let config = FinstackIoConfig::from_env()?;
    match config.backend {
        IoBackend::Sqlite => open_sqlite_store(config.sqlite_path).await,
        IoBackend::Postgres => open_postgres_store(config.postgres_url).await,
        IoBackend::Turso => open_turso_store(config.turso_path).await,
    }
}
