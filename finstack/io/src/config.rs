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

use crate::governance::{ActorContext, GovernedHandle};
use crate::store::GovernanceStore;
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

impl StoreHandle {
    /// Create a governed handle for the given actor.
    ///
    /// This wraps the store with governance checks for the specified actor.
    /// Governance configuration is read from the environment on the first
    /// call and cached for the process lifetime.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{ActorContext, StoreHandle};
    /// # fn example(store: StoreHandle) {
    /// let governed = store.as_actor(ActorContext::user("alice"));
    /// // `governed` now checks permissions on every read
    /// # }
    /// ```
    #[must_use]
    pub fn as_actor(&self, actor: ActorContext) -> GovernedHandle {
        GovernedHandle::new(self.clone(), actor)
    }
}

#[async_trait]
impl GovernanceStore for StoreHandle {
    async fn get_resource_entity(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<crate::ResourceEntity>> {
        dispatch_store!(self, get_resource_entity, resource_type, resource_id)
    }

    async fn list_resource_entities(
        &self,
        resource_type: &str,
    ) -> Result<Vec<crate::ResourceEntity>> {
        dispatch_store!(self, list_resource_entities, resource_type)
    }

    async fn upsert_resource_entity(&self, entity: &crate::ResourceEntity) -> Result<()> {
        dispatch_store!(self, upsert_resource_entity, entity)
    }

    async fn list_resource_shares(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<crate::ResourceShare>> {
        dispatch_store!(self, list_resource_shares, resource_type, resource_id)
    }

    async fn list_all_resource_shares(
        &self,
        resource_type: &str,
    ) -> Result<std::collections::HashMap<String, Vec<crate::ResourceShare>>> {
        dispatch_store!(self, list_all_resource_shares, resource_type)
    }

    async fn list_user_roles(&self, user_id: &str) -> Result<Vec<crate::UserRole>> {
        dispatch_store!(self, list_user_roles, user_id)
    }

    async fn list_user_groups(&self, user_id: &str) -> Result<Vec<String>> {
        dispatch_store!(self, list_user_groups, user_id)
    }

    async fn list_workflow_bindings(
        &self,
        resource_type: &str,
    ) -> Result<Vec<crate::governance::WorkflowBinding>> {
        dispatch_store!(self, list_workflow_bindings, resource_type)
    }

    async fn get_workflow_transition(
        &self,
        policy_id: &str,
        from_state: &str,
        to_state: &str,
    ) -> Result<Option<crate::governance::WorkflowTransition>> {
        dispatch_store!(
            self,
            get_workflow_transition,
            policy_id,
            from_state,
            to_state
        )
    }

    async fn get_workflow_state(
        &self,
        policy_id: &str,
        state_key: &str,
    ) -> Result<Option<crate::governance::WorkflowState>> {
        dispatch_store!(self, get_workflow_state, policy_id, state_key)
    }

    async fn insert_workflow_event(
        &self,
        event_id: &str,
        change_id: &str,
        resource_type: &str,
        resource_id: &str,
        resource_key2: &str,
        from_state: &str,
        to_state: &str,
        actor_kind: crate::ActorKind,
        actor_id: &str,
        note: Option<&str>,
    ) -> Result<()> {
        dispatch_store!(
            self,
            insert_workflow_event,
            event_id,
            change_id,
            resource_type,
            resource_id,
            resource_key2,
            from_state,
            to_state,
            actor_kind,
            actor_id,
            note
        )
    }

    async fn last_workflow_event_actor(&self, change_id: &str) -> Result<Option<String>> {
        dispatch_store!(self, last_workflow_event_actor, change_id)
    }

    async fn insert_resource_change(&self, change: &crate::ResourceChangeInsert) -> Result<()> {
        dispatch_store!(self, insert_resource_change, change)
    }

    async fn update_resource_change_state(
        &self,
        change_id: &str,
        workflow_state: &str,
        workflow_policy_id: Option<&str>,
        submitted_at: Option<&str>,
        applied_at: Option<&str>,
    ) -> Result<()> {
        dispatch_store!(
            self,
            update_resource_change_state,
            change_id,
            workflow_state,
            workflow_policy_id,
            submitted_at,
            applied_at
        )
    }

    async fn get_resource_change(&self, change_id: &str) -> Result<Option<crate::ResourceChange>> {
        dispatch_store!(self, get_resource_change, change_id)
    }

    async fn list_resource_changes_for_owner(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<crate::ResourceChange>> {
        dispatch_store!(self, list_resource_changes_for_owner, owner_user_id)
    }

    async fn latest_verified_state(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<String>> {
        dispatch_store!(self, latest_verified_state, resource_type, resource_id)
    }

    async fn apply_change_to_verified(&self, change: &crate::ResourceChange) -> Result<()> {
        dispatch_store!(self, apply_change_to_verified, change)
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

#[async_trait]
impl Store for StoreHandle {
    async fn put_market_context(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
        context: &finstack_core::market_data::context::MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_market_context, market_id, as_of, context, meta)
    }

    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<finstack_core::market_data::context::MarketContext>> {
        dispatch_store!(self, get_market_context, market_id, as_of)
    }

    async fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &finstack_valuations::instruments::InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_instrument, instrument_id, instrument, meta)
    }

    async fn get_instrument(
        &self,
        instrument_id: &str,
    ) -> Result<Option<finstack_valuations::instruments::InstrumentJson>> {
        dispatch_store!(self, get_instrument, instrument_id)
    }

    async fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<std::collections::HashMap<String, finstack_valuations::instruments::InstrumentJson>>
    {
        dispatch_store!(self, get_instruments_batch, instrument_ids)
    }

    async fn list_instruments(&self) -> Result<Vec<String>> {
        dispatch_store!(self, list_instruments)
    }

    async fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
        spec: &finstack_portfolio::PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_portfolio_spec, portfolio_id, as_of, spec, meta)
    }

    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<finstack_portfolio::PortfolioSpec>> {
        dispatch_store!(self, get_portfolio_spec, portfolio_id, as_of)
    }

    async fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &finstack_scenarios::ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_scenario, scenario_id, spec, meta)
    }

    async fn get_scenario(
        &self,
        scenario_id: &str,
    ) -> Result<Option<finstack_scenarios::ScenarioSpec>> {
        dispatch_store!(self, get_scenario, scenario_id)
    }

    async fn list_scenarios(&self) -> Result<Vec<String>> {
        dispatch_store!(self, list_scenarios)
    }

    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &finstack_statements::FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_statement_model, model_id, spec, meta)
    }

    async fn get_statement_model(
        &self,
        model_id: &str,
    ) -> Result<Option<finstack_statements::FinancialModelSpec>> {
        dispatch_store!(self, get_statement_model, model_id)
    }

    async fn list_statement_models(&self) -> Result<Vec<String>> {
        dispatch_store!(self, list_statement_models)
    }

    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &finstack_statements::registry::MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_metric_registry, namespace, registry, meta)
    }

    async fn get_metric_registry(
        &self,
        namespace: &str,
    ) -> Result<Option<finstack_statements::registry::MetricRegistry>> {
        dispatch_store!(self, get_metric_registry, namespace)
    }

    async fn list_metric_registries(&self) -> Result<Vec<String>> {
        dispatch_store!(self, list_metric_registries)
    }

    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        dispatch_store!(self, delete_metric_registry, namespace)
    }
}

#[async_trait]
impl BulkStore for StoreHandle {
    async fn put_instruments_batch(
        &self,
        instruments: &[(
            &str,
            &finstack_valuations::instruments::InstrumentJson,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()> {
        dispatch_store!(self, put_instruments_batch, instruments)
    }

    async fn put_market_contexts_batch(
        &self,
        contexts: &[(
            &str,
            finstack_core::dates::Date,
            &finstack_core::market_data::context::MarketContext,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()> {
        dispatch_store!(self, put_market_contexts_batch, contexts)
    }

    async fn put_portfolios_batch(
        &self,
        portfolios: &[(
            &str,
            finstack_core::dates::Date,
            &finstack_portfolio::PortfolioSpec,
            Option<&serde_json::Value>,
        )],
    ) -> Result<()> {
        dispatch_store!(self, put_portfolios_batch, portfolios)
    }
}

#[async_trait]
impl LookbackStore for StoreHandle {
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
    ) -> Result<Vec<crate::MarketContextSnapshot>> {
        dispatch_store!(self, list_market_contexts, market_id, start, end)
    }

    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::MarketContextSnapshot>> {
        dispatch_store!(self, latest_market_context_on_or_before, market_id, as_of)
    }

    async fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
    ) -> Result<Vec<crate::PortfolioSnapshot>> {
        dispatch_store!(self, list_portfolios, portfolio_id, start, end)
    }

    async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: finstack_core::dates::Date,
    ) -> Result<Option<crate::PortfolioSnapshot>> {
        dispatch_store!(self, latest_portfolio_on_or_before, portfolio_id, as_of)
    }
}

#[async_trait]
impl TimeSeriesStore for StoreHandle {
    async fn put_series_meta(
        &self,
        key: &crate::SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        dispatch_store!(self, put_series_meta, key, meta)
    }

    async fn get_series_meta(&self, key: &crate::SeriesKey) -> Result<Option<serde_json::Value>> {
        dispatch_store!(self, get_series_meta, key)
    }

    async fn list_series(&self, namespace: &str, kind: crate::SeriesKind) -> Result<Vec<String>> {
        dispatch_store!(self, list_series, namespace, kind)
    }

    async fn put_points_batch(
        &self,
        key: &crate::SeriesKey,
        points: &[crate::TimeSeriesPoint],
    ) -> Result<()> {
        dispatch_store!(self, put_points_batch, key, points)
    }

    async fn get_points_range(
        &self,
        key: &crate::SeriesKey,
        start: time::OffsetDateTime,
        end: time::OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<crate::TimeSeriesPoint>> {
        dispatch_store!(self, get_points_range, key, start, end, limit)
    }

    async fn latest_point_on_or_before(
        &self,
        key: &crate::SeriesKey,
        ts: time::OffsetDateTime,
    ) -> Result<Option<crate::TimeSeriesPoint>> {
        dispatch_store!(self, latest_point_on_or_before, key, ts)
    }
}

/// Open a store using the current environment configuration.
///
/// Reads `FINSTACK_IO_BACKEND` to select the provider, then reads the
/// corresponding path/URL variable. Migrations run automatically.
///
/// # Errors
///
/// - [`Error::Invariant`](crate::Error::Invariant) if the required path/URL
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
        IoBackend::Sqlite => {
            #[cfg(feature = "sqlite")]
            {
                let path = config.sqlite_path.ok_or_else(|| {
                    Error::Invariant("FINSTACK_SQLITE_PATH is required for sqlite backend".into())
                })?;
                Ok(StoreHandle::Sqlite(
                    crate::sqlite::SqliteStore::open(path).await?,
                ))
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
                    crate::postgres::PostgresStore::connect(&url).await?,
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
                Ok(StoreHandle::Turso(
                    crate::turso::TursoStore::open(path).await?,
                ))
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
