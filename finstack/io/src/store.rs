//! Backend-agnostic async persistence API.
//!
//! `finstack-io` provides a small, typed repository interface via [`Store`].
//! Storage backends (SQLite, Postgres, Turso, etc.) implement this trait.
//!
//! All operations are async to support efficient I/O across different backends.

use crate::governance::{
    ActorKind, ResourceChange, ResourceChangeInsert, ResourceEntity, ResourceShare, UserRole,
    WorkflowBinding, WorkflowState, WorkflowTransition,
};
use crate::{Error, Result};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_portfolio::{Portfolio, PortfolioSpec};
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use std::collections::HashMap;
use time::OffsetDateTime;

/// Maximum number of items in a single batch query.
///
/// Larger batches are automatically chunked to avoid:
/// - Query plan cache pollution (Postgres creates a new plan per distinct param count)
/// - Excessive query string length
/// - Memory pressure from large IN clauses
///
/// The default value of 500 balances efficiency (fewer round-trips) with reasonable
/// query complexity.
pub const MAX_BATCH_SIZE: usize = 500;

/// Async typed persistence interface for Finstack domain objects.
///
/// Backends should treat `put_*` operations as **idempotent** (upsert) whenever
/// the underlying store supports it.
///
/// # Cloneability
///
/// All built-in backends (`SqliteStore`, `PostgresStore`, `TursoStore`) implement
/// `Clone` cheaply via internal `Arc`. Users should feel free to clone store handles
/// for use across tasks or threads. The underlying connection pool or connection
/// wrapper is shared, not duplicated.
///
/// Custom backend implementations should follow this pattern to maintain consistency
/// with the built-in backends.
///
/// # Transaction Isolation
///
/// Individual `put_*` and `get_*` calls are atomic, but compound operations like
/// [`load_portfolio`](Store::load_portfolio) (which reads the portfolio spec and
/// then resolves instruments) are **not** transactionally isolated. If instruments
/// are updated concurrently, the hydrated portfolio could contain inconsistent data.
///
/// For production use cases requiring strict consistency, consider:
/// - Using the bulk methods (`put_instruments_batch`, `put_market_contexts_batch`)
///   which execute within a single transaction
/// - Implementing application-level locking
/// - Using portfolio specs with inline `instrument_spec` to avoid the lookup
///
/// # Metadata Handling
///
/// All `put_*` methods accept an optional `meta` parameter for storing provenance
/// information (source, version, tags, etc.). This metadata is persisted alongside
/// the payload for auditing and debugging purposes, but is **not returned** by
/// `get_*` methods. If you need to retrieve metadata, access the store directly
/// or implement custom queries.
#[async_trait]
pub trait Store: Send + Sync {
    /// Store a market context snapshot for a given `as_of` date.
    async fn put_market_context(
        &self,
        market_id: &str,
        as_of: Date,
        context: &MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a market context snapshot for a given `as_of` date.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned by this method.
    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContext>>;

    /// Store an instrument definition.
    async fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load an instrument definition.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned by this method.
    async fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>>;

    /// Load multiple instruments by ID.
    ///
    /// Returns a map of instrument_id -> InstrumentJson for all found instruments.
    /// Missing instruments are silently omitted from the result (no error).
    ///
    /// # Batching
    ///
    /// Large requests are automatically chunked into batches of [`MAX_BATCH_SIZE`]
    /// to avoid query plan cache pollution and excessive query complexity. Results
    /// are merged from all chunks.
    async fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<HashMap<String, InstrumentJson>>;

    /// List all stored instrument IDs.
    async fn list_instruments(&self) -> Result<Vec<String>>;

    /// Store a portfolio snapshot for a given `as_of` date.
    async fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a portfolio snapshot for a given `as_of` date.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned by this method.
    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSpec>>;

    /// Store a scenario specification.
    async fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a scenario specification.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned by this method.
    async fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>>;

    /// List all stored scenario IDs.
    async fn list_scenarios(&self) -> Result<Vec<String>>;

    /// Store a statements model specification.
    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a statements model specification.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned by this method.
    async fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>>;

    /// List all stored statement model IDs.
    async fn list_statement_models(&self) -> Result<Vec<String>>;

    /// Store a metric registry by namespace.
    ///
    /// Metric registries define reusable financial metrics (ratios, KPIs) that can be
    /// shared across multiple statement models. The namespace (e.g., "fin", "custom")
    /// serves as the primary key.
    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a metric registry by namespace.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned by this method.
    async fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>>;

    /// List all stored metric registry namespaces.
    async fn list_metric_registries(&self) -> Result<Vec<String>>;

    /// Delete a metric registry by namespace.
    ///
    /// Returns `true` if a registry was deleted, `false` if no registry existed.
    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool>;

    /// Load a market context snapshot, returning a not-found error if missing.
    async fn load_market_context(&self, market_id: &str, as_of: Date) -> Result<MarketContext> {
        self.get_market_context(market_id, as_of)
            .await?
            .ok_or_else(|| Error::not_found("market_context", format!("{market_id}@{as_of}")))
    }

    /// Load a portfolio spec snapshot, returning a not-found error if missing.
    async fn load_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<PortfolioSpec> {
        self.get_portfolio_spec(portfolio_id, as_of)
            .await?
            .ok_or_else(|| Error::not_found("portfolio", format!("{portfolio_id}@{as_of}")))
    }

    /// Load a metric registry, returning a not-found error if missing.
    async fn load_metric_registry(&self, namespace: &str) -> Result<MetricRegistry> {
        self.get_metric_registry(namespace)
            .await?
            .ok_or_else(|| Error::not_found("metric_registry", namespace))
    }

    /// Load and hydrate a portfolio for valuation/aggregation.
    ///
    /// Hydration rule:
    /// - If a position's `instrument_spec` is `None`, resolve it from the
    ///   instruments registry using `instrument_id`.
    ///
    /// # Transaction Isolation Warning
    ///
    /// This method performs multiple database reads (portfolio spec + instruments batch)
    /// **without transaction isolation**. If instruments are modified concurrently between
    /// these reads, the hydrated portfolio could contain:
    /// - Stale instrument definitions (if an instrument was updated)
    /// - A `NotFound` error (if an instrument was deleted)
    ///
    /// For strict consistency requirements, consider:
    /// - Using portfolio specs with inline `instrument_spec` to avoid the lookup
    /// - Implementing application-level locking around portfolio operations
    /// - Using bulk write operations (`put_instruments_batch`) which are transactional
    async fn load_portfolio(&self, portfolio_id: &str, as_of: Date) -> Result<Portfolio> {
        let mut spec = self.load_portfolio_spec(portfolio_id, as_of).await?;

        // Collect unique instrument IDs that need resolution, preserving first-seen order.
        let mut seen = std::collections::HashSet::<&str>::new();
        let mut missing_ids = Vec::new();
        for pos in &spec.positions {
            if pos.instrument_spec.is_some() {
                continue;
            }
            if seen.insert(pos.instrument_id.as_str()) {
                missing_ids.push(pos.instrument_id.clone());
            }
        }

        // Batch-fetch all missing instruments
        let instruments = self.get_instruments_batch(&missing_ids).await?;

        // Resolve missing instrument specs from the fetched batch
        for pos in &mut spec.positions {
            if pos.instrument_spec.is_some() {
                continue;
            }

            let instrument = instruments
                .get(&pos.instrument_id)
                .ok_or_else(|| Error::not_found("instrument", pos.instrument_id.clone()))?;

            pos.instrument_spec = Some(instrument.clone());
        }

        Ok(Portfolio::from_spec(spec)?)
    }

    /// Convenience helper: load a portfolio and matching market context for the same `as_of`.
    async fn load_portfolio_with_market(
        &self,
        portfolio_id: &str,
        market_id: &str,
        as_of: Date,
    ) -> Result<(Portfolio, MarketContext)> {
        let portfolio = self.load_portfolio(portfolio_id, as_of).await?;
        let market = self.load_market_context(market_id, as_of).await?;
        Ok((portfolio, market))
    }
}

/// Internal governance store interface for authorization and workflow operations.
#[async_trait]
pub(crate) trait GovernanceStore: Send + Sync {
    /// Load a resource entity row.
    async fn get_resource_entity(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<ResourceEntity>>;

    /// Upsert a resource entity row.
    async fn upsert_resource_entity(&self, entity: &ResourceEntity) -> Result<()>;

    /// List resource entities for a given type.
    async fn list_resource_entities(&self, resource_type: &str) -> Result<Vec<ResourceEntity>>;

    /// List explicit shares for a resource.
    async fn list_resource_shares(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<ResourceShare>>;

    /// List all shares for a given resource type, keyed by resource id.
    ///
    /// This avoids N+1 queries when filtering a list of resources by
    /// authorization. The default implementation falls back to per-resource
    /// queries but providers should override with a single batch query.
    async fn list_all_resource_shares(
        &self,
        resource_type: &str,
    ) -> Result<HashMap<String, Vec<ResourceShare>>> {
        let _ = resource_type;
        Ok(HashMap::new())
    }

    /// List roles for a user.
    async fn list_user_roles(&self, user_id: &str) -> Result<Vec<UserRole>>;

    /// List groups for a user.
    async fn list_user_groups(&self, user_id: &str) -> Result<Vec<String>>;

    /// List workflow bindings for a resource type.
    async fn list_workflow_bindings(&self, resource_type: &str) -> Result<Vec<WorkflowBinding>>;

    /// Load a workflow transition definition.
    async fn get_workflow_transition(
        &self,
        policy_id: &str,
        from_state: &str,
        to_state: &str,
    ) -> Result<Option<WorkflowTransition>>;

    /// Load a workflow state definition.
    async fn get_workflow_state(
        &self,
        policy_id: &str,
        state_key: &str,
    ) -> Result<Option<WorkflowState>>;

    /// Insert a workflow event row.
    #[allow(clippy::too_many_arguments)]
    async fn insert_workflow_event(
        &self,
        event_id: &str,
        change_id: &str,
        resource_type: &str,
        resource_id: &str,
        resource_key2: &str,
        from_state: &str,
        to_state: &str,
        actor_kind: ActorKind,
        actor_id: &str,
        note: Option<&str>,
    ) -> Result<()>;

    /// Get the last workflow event actor id for a change.
    async fn last_workflow_event_actor(&self, change_id: &str) -> Result<Option<String>>;

    /// Insert a resource change proposal.
    async fn insert_resource_change(&self, change: &ResourceChangeInsert) -> Result<()>;

    /// Update workflow state for a change proposal.
    async fn update_resource_change_state(
        &self,
        change_id: &str,
        workflow_state: &str,
        workflow_policy_id: Option<&str>,
        submitted_at: Option<&str>,
        applied_at: Option<&str>,
    ) -> Result<()>;

    /// Load a resource change proposal by id.
    async fn get_resource_change(&self, change_id: &str) -> Result<Option<ResourceChange>>;

    /// List resource changes for an owner.
    async fn list_resource_changes_for_owner(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<ResourceChange>>;

    /// Get the latest final workflow state for a resource.
    async fn latest_verified_state(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<String>>;

    /// Apply a resource change to the verified tables.
    ///
    /// # Payload Format Contract
    ///
    /// The `change.payload` **must** use the same serialization format as the
    /// corresponding `put_*` method for the resource type. Specifically:
    ///
    /// | Resource type     | Expected payload type               |
    /// |-------------------|-------------------------------------|
    /// | `instrument`      | `InstrumentJson`                    |
    /// | `market_context`  | `MarketContextState` (not `MarketContext`) |
    /// | `portfolio`       | `PortfolioSpec`                     |
    /// | `scenario`        | `ScenarioSpec`                      |
    /// | `statement_model` | `FinancialModelSpec`                |
    /// | `metric_registry` | `MetricRegistry`                    |
    /// | `series_meta`     | Arbitrary JSON metadata             |
    ///
    /// Passing a payload in the wrong format (e.g., a raw `MarketContext`
    /// instead of `MarketContextState`) will succeed at write time but cause
    /// deserialization failures on subsequent reads from the verified table.
    ///
    /// In debug builds, `GovernedHandle` validates the payload format before
    /// calling this method.
    async fn apply_change_to_verified(&self, change: &ResourceChange) -> Result<()>;
}

/// Extension trait for bulk operations.
///
/// Bulk methods execute within a single transaction for atomicity and better performance
/// when inserting many records.
#[async_trait]
pub trait BulkStore: Store {
    /// Store multiple instruments in a single transaction.
    ///
    /// This is more efficient than calling `put_instrument` repeatedly and provides
    /// atomicity (all-or-nothing).
    async fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()>;

    /// Store multiple market contexts in a single transaction.
    ///
    /// This is more efficient than calling `put_market_context` repeatedly and provides
    /// atomicity (all-or-nothing).
    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()>;

    /// Store multiple portfolio specs in a single transaction.
    async fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()>;
}

/// A time-indexed market context snapshot returned from a lookback query.
#[derive(Clone)]
pub struct MarketContextSnapshot {
    /// As-of date for this snapshot.
    pub as_of: Date,
    /// Market context snapshot.
    pub context: MarketContext,
}

impl std::fmt::Debug for MarketContextSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarketContextSnapshot")
            .field("as_of", &self.as_of)
            .field("context", &"<MarketContext>")
            .finish()
    }
}

/// A time-indexed portfolio snapshot returned from a lookback query.
#[derive(Debug, Clone)]
pub struct PortfolioSnapshot {
    /// As-of date for this snapshot.
    pub as_of: Date,
    /// Portfolio specification snapshot.
    pub spec: PortfolioSpec,
}

/// Optional extension trait for backends that support range queries / lookbacks.
#[async_trait]
pub trait LookbackStore: Send + Sync {
    /// List market contexts for a given id in `[start, end]`, ordered by `as_of`.
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>>;

    /// Get the latest market context with `as_of <= as_of`, if any.
    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>>;

    /// List portfolio specs for a given id in `[start, end]`, ordered by `as_of`.
    async fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>>;

    /// Get the latest portfolio with `as_of <= as_of`, if any.
    async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>>;
}

/// Kind of time-series data stored in the IO backend.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum SeriesKind {
    /// Market quotes and prices (bid/ask/last, etc.).
    Quote,
    /// Computed metrics (e.g., risk measures, KPIs).
    Metric,
    /// Result series (e.g., valuation outputs, scenario results).
    Result,
    /// Profit and loss series.
    Pnl,
    /// Risk series (e.g., VaR, stress metrics).
    Risk,
}

impl SeriesKind {
    /// Render the kind as a stable, lowercase identifier.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            SeriesKind::Quote => "quote",
            SeriesKind::Metric => "metric",
            SeriesKind::Result => "result",
            SeriesKind::Pnl => "pnl",
            SeriesKind::Risk => "risk",
        }
    }

    /// Parse a kind from a lowercase identifier.
    ///
    /// Returns an error if the value is not one of the known kinds.
    pub fn parse(value: &str) -> Result<Self> {
        Self::try_parse(value).ok_or_else(|| Error::invalid_series_kind(value))
    }

    /// Try to parse a kind from a lowercase identifier.
    ///
    /// Returns `None` if the value is not recognized.
    #[must_use]
    pub fn try_parse(value: &str) -> Option<Self> {
        match value {
            "quote" => Some(SeriesKind::Quote),
            "metric" => Some(SeriesKind::Metric),
            "result" => Some(SeriesKind::Result),
            "pnl" => Some(SeriesKind::Pnl),
            "risk" => Some(SeriesKind::Risk),
            _ => None,
        }
    }
}

/// Unique identifier for a time-series.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SeriesKey {
    /// Logical namespace (e.g., "market", "portfolio", "risk").
    pub namespace: String,
    /// Series identifier within the namespace.
    pub series_id: String,
    /// Series kind.
    pub kind: SeriesKind,
}

impl SeriesKey {
    /// Create a new series key.
    #[must_use]
    pub fn new(
        namespace: impl Into<String>,
        series_id: impl Into<String>,
        kind: SeriesKind,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            series_id: series_id.into(),
            kind,
        }
    }
}

/// A single time-series point.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeSeriesPoint {
    /// Timestamp for this point.
    pub ts: OffsetDateTime,
    /// Optional numeric value for quick analytics.
    pub value: Option<f64>,
    /// Optional structured payload (e.g., bid/ask/last).
    pub payload: Option<serde_json::Value>,
    /// Optional metadata (provenance, tags).
    pub meta: Option<serde_json::Value>,
}

/// Typed async persistence interface for time-series data.
#[async_trait]
pub trait TimeSeriesStore: Send + Sync {
    /// Store metadata for a time-series key.
    async fn put_series_meta(
        &self,
        key: &SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load metadata for a time-series key.
    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>>;

    /// List series ids for a namespace and kind.
    async fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>>;

    /// Store multiple points in a single transaction.
    async fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()>;

    /// Load points in a time range, ordered by timestamp.
    async fn get_points_range(
        &self,
        key: &SeriesKey,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>>;

    /// Get the latest point on or before a given timestamp.
    async fn latest_point_on_or_before(
        &self,
        key: &SeriesKey,
        ts: OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>>;
}
