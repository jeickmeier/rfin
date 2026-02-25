//! Backend-agnostic async persistence API.
//!
//! This module defines the core persistence traits that all backends implement.
//! The trait hierarchy is:
//!
//! - [`Store`] — Basic CRUD for instruments, portfolios, market contexts,
//!   scenarios, statement models, and metric registries.
//! - [`BulkStore`] — Batch insert operations that execute within a single
//!   transaction for atomicity and performance.
//! - [`LookbackStore`] — Range queries over time-indexed snapshots (market
//!   contexts and portfolios keyed by `as_of` date).
//! - [`TimeSeriesStore`] — Storage and retrieval of time-series data points
//!   (quotes, metrics, results, PnL, risk).
//!
//! All built-in backends (`SqliteStore`, `PostgresStore`, `TursoStore`) implement
//! all four traits. The [`StoreHandle`](crate::StoreHandle) enum dispatches to
//! whichever backend was selected at runtime.
//!
//! # Example
//!
//! ```rust,no_run
//! use finstack_io::{SqliteStore, Store};
//!
//! # async fn example() -> finstack_io::Result<()> {
//! let store = SqliteStore::open("finstack.db").await?;
//!
//! // Store an instrument definition
//! # let instrument = todo!();
//! store.put_instrument("DEPO-001", &instrument, None).await?;
//!
//! // Retrieve it (returns None if not found)
//! let loaded = store.get_instrument("DEPO-001").await?;
//! assert!(loaded.is_some());
//!
//! // List all stored instrument IDs
//! let ids = store.list_instruments().await?;
//! # Ok(())
//! # }
//! ```

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
    ///
    /// This is an **upsert** operation — if a snapshot for the same
    /// `(market_id, as_of)` already exists, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `market_id` - Logical identifier for the market dataset (e.g., `"USD-CURVES"`).
    /// * `as_of` - Observation date for the snapshot.
    /// * `context` - The market data to store (curves, surfaces, FX rates, etc.).
    /// * `meta` - Optional provenance metadata (source, version, tags).
    ///   Stored alongside the payload for auditing but **not** returned by
    ///   [`get_market_context`](Store::get_market_context).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the backend encounters a
    /// connection / write error.
    async fn put_market_context(
        &self,
        market_id: &str,
        as_of: Date,
        context: &MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a market context snapshot for a given `as_of` date.
    ///
    /// Returns `None` if no snapshot exists for the exact `(market_id, as_of)` pair.
    /// Use [`load_market_context`](Store::load_market_context) if you want a
    /// `NotFound` error instead of `None`.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned
    /// by this method.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a
    /// connection / read error.
    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContext>>;

    /// Store an instrument definition.
    ///
    /// This is an **upsert** — if an instrument with the same `instrument_id`
    /// already exists, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - Unique identifier (e.g., `"DEPO-001"`, `"SWAP-USD-5Y"`).
    /// * `instrument` - The instrument definition to store.
    /// * `meta` - Optional provenance metadata. Stored for auditing but **not**
    ///   returned by [`get_instrument`](Store::get_instrument).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the backend encounters a write error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{SqliteStore, Store};
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = SqliteStore::open("finstack.db").await?;
    /// # let instrument = todo!();
    ///
    /// // Store without metadata
    /// store.put_instrument("DEPO-001", &instrument, None).await?;
    ///
    /// // Store with provenance metadata
    /// let meta = serde_json::json!({ "source": "bloomberg", "version": 2 });
    /// store.put_instrument("DEPO-001", &instrument, Some(&meta)).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load an instrument definition by ID.
    ///
    /// Returns `None` if no instrument with the given ID exists.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned
    /// by this method.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a read error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{SqliteStore, Store};
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = SqliteStore::open("finstack.db").await?;
    ///
    /// match store.get_instrument("DEPO-001").await? {
    ///     Some(instrument) => println!("Found instrument"),
    ///     None => println!("Instrument not found"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>>;

    /// Load multiple instruments by ID in a single query.
    ///
    /// Returns a map of `instrument_id -> InstrumentJson` for all found instruments.
    /// Missing instruments are silently omitted from the result (no error is
    /// raised for IDs that don't exist).
    ///
    /// # Batching
    ///
    /// Large requests are automatically chunked into batches of [`MAX_BATCH_SIZE`]
    /// to avoid query plan cache pollution and excessive query complexity. Results
    /// are merged from all chunks.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a read error.
    async fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<HashMap<String, InstrumentJson>>;

    /// List all stored instrument IDs.
    ///
    /// Returns an empty vector if no instruments have been stored.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_instruments(&self) -> Result<Vec<String>>;

    /// Store a portfolio snapshot for a given `as_of` date.
    ///
    /// This is an **upsert** — if a snapshot for the same `(portfolio_id, as_of)`
    /// already exists, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `portfolio_id` - Logical identifier for the portfolio (e.g., `"equity-book"`).
    /// * `as_of` - Observation date for the snapshot.
    /// * `spec` - The portfolio specification to store.
    /// * `meta` - Optional provenance metadata. Stored for auditing but **not**
    ///   returned by [`get_portfolio_spec`](Store::get_portfolio_spec).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the backend encounters a write error.
    async fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a portfolio snapshot for a given `as_of` date.
    ///
    /// Returns `None` if no snapshot exists for the exact `(portfolio_id, as_of)`.
    /// Use [`load_portfolio_spec`](Store::load_portfolio_spec) if you want a
    /// `NotFound` error instead.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned
    /// by this method.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a read error.
    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSpec>>;

    /// Store a scenario specification.
    ///
    /// This is an **upsert** — if a scenario with the same `scenario_id`
    /// already exists, it is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the backend encounters a write error.
    async fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a scenario specification.
    ///
    /// Returns `None` if no scenario with the given ID exists.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned
    /// by this method.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a read error.
    async fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>>;

    /// List all stored scenario IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_scenarios(&self) -> Result<Vec<String>>;

    /// Store a financial statement model specification.
    ///
    /// This is an **upsert** — if a model with the same `model_id` already
    /// exists, it is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the backend encounters a write error.
    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a financial statement model specification.
    ///
    /// Returns `None` if no model with the given ID exists.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned
    /// by this method.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a read error.
    async fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>>;

    /// List all stored statement model IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_statement_models(&self) -> Result<Vec<String>>;

    /// Store a metric registry by namespace.
    ///
    /// Metric registries define reusable financial metrics (ratios, KPIs) that can be
    /// shared across multiple statement models. The namespace (e.g., `"fin"`, `"custom"`)
    /// serves as the primary key.
    ///
    /// This is an **upsert** — if a registry with the same namespace already exists,
    /// it is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the backend encounters a write error.
    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a metric registry by namespace.
    ///
    /// Returns `None` if no registry with the given namespace exists.
    /// Use [`load_metric_registry`](Store::load_metric_registry) if you want a
    /// `NotFound` error instead.
    ///
    /// Note: The `meta` field is stored for auditing purposes but not returned
    /// by this method.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the backend encounters a read error.
    async fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>>;

    /// List all stored metric registry namespaces.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_metric_registries(&self) -> Result<Vec<String>>;

    /// Delete a metric registry by namespace.
    ///
    /// Returns `true` if a registry was deleted, `false` if no registry existed
    /// for the given namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a write error.
    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool>;

    /// Load a market context snapshot, returning a not-found error if missing.
    ///
    /// This is a convenience wrapper around [`get_market_context`](Store::get_market_context)
    /// that converts `None` into [`Error::NotFound`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if no snapshot exists
    /// for the given `(market_id, as_of)`. Also propagates backend errors.
    async fn load_market_context(&self, market_id: &str, as_of: Date) -> Result<MarketContext> {
        self.get_market_context(market_id, as_of)
            .await?
            .ok_or_else(|| Error::not_found("market_context", format!("{market_id}@{as_of}")))
    }

    /// Load a portfolio spec snapshot, returning a not-found error if missing.
    ///
    /// This is a convenience wrapper around [`get_portfolio_spec`](Store::get_portfolio_spec)
    /// that converts `None` into [`Error::NotFound`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if no snapshot exists
    /// for the given `(portfolio_id, as_of)`. Also propagates backend errors.
    async fn load_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<PortfolioSpec> {
        self.get_portfolio_spec(portfolio_id, as_of)
            .await?
            .ok_or_else(|| Error::not_found("portfolio", format!("{portfolio_id}@{as_of}")))
    }

    /// Load a metric registry, returning a not-found error if missing.
    ///
    /// This is a convenience wrapper around [`get_metric_registry`](Store::get_metric_registry)
    /// that converts `None` into [`Error::NotFound`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if no registry exists
    /// for the given namespace. Also propagates backend errors.
    async fn load_metric_registry(&self, namespace: &str) -> Result<MetricRegistry> {
        self.get_metric_registry(namespace)
            .await?
            .ok_or_else(|| Error::not_found("metric_registry", namespace))
    }

    /// Load and hydrate a portfolio for valuation/aggregation.
    ///
    /// This loads the portfolio spec and then resolves any positions whose
    /// `instrument_spec` is `None` by batch-fetching the corresponding
    /// instruments from the store. The result is a fully hydrated
    /// [`Portfolio`] ready for valuation.
    ///
    /// # Hydration Rule
    ///
    /// For each position where `instrument_spec` is `None`, the instrument
    /// is resolved from the instruments registry using `instrument_id`.
    /// Positions that already have an inline `instrument_spec` are left as-is.
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
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`] if the portfolio spec or any
    ///   referenced instrument does not exist.
    /// - Backend or deserialization errors from the underlying reads.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{SqliteStore, Store};
    /// # use time::macros::date;
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = SqliteStore::open("finstack.db").await?;
    ///
    /// // Load and hydrate a portfolio (instruments are resolved automatically)
    /// let portfolio = store.load_portfolio("equity-book", date!(2025-01-15)).await?;
    /// println!("Positions: {}", portfolio.positions.len());
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// This is equivalent to calling [`load_portfolio`](Store::load_portfolio) and
    /// [`load_market_context`](Store::load_market_context) in sequence. Both must
    /// exist or a `NotFound` error is returned.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`] if either the portfolio or
    ///   the market context does not exist.
    /// - Backend or deserialization errors from the underlying reads.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{SqliteStore, Store};
    /// # use time::macros::date;
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = SqliteStore::open("finstack.db").await?;
    /// let as_of = date!(2025-01-15);
    ///
    /// let (portfolio, market) = store
    ///     .load_portfolio_with_market("equity-book", "USD-CURVES", as_of)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
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

/// Extension trait for bulk operations.
///
/// Bulk methods execute within a **single transaction** for atomicity and better
/// performance when inserting many records. If any item in the batch fails, the
/// entire batch is rolled back (all-or-nothing).
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_io::{SqliteStore, BulkStore};
///
/// # async fn example(
/// #     store: SqliteStore,
/// #     instr_a: finstack_valuations::instruments::InstrumentJson,
/// #     instr_b: finstack_valuations::instruments::InstrumentJson,
/// # ) -> finstack_io::Result<()> {
/// // Insert multiple instruments atomically
/// store.put_instruments_batch(&[
///     ("DEPO-001", &instr_a, None),
///     ("DEPO-002", &instr_b, None),
/// ]).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait BulkStore: Store {
    /// Store multiple instruments in a single transaction.
    ///
    /// Each tuple is `(instrument_id, instrument, optional_meta)`.
    /// This is more efficient than calling [`put_instrument`](Store::put_instrument)
    /// repeatedly and provides atomicity (all-or-nothing).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization of any item fails or the backend
    /// encounters a write error. On error, no items are committed.
    async fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()>;

    /// Store multiple market contexts in a single transaction.
    ///
    /// Each tuple is `(market_id, as_of, context, optional_meta)`.
    /// This is more efficient than calling [`put_market_context`](Store::put_market_context)
    /// repeatedly and provides atomicity (all-or-nothing).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization of any item fails or the backend
    /// encounters a write error. On error, no items are committed.
    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()>;

    /// Store multiple portfolio specs in a single transaction.
    ///
    /// Each tuple is `(portfolio_id, as_of, spec, optional_meta)`.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization of any item fails or the backend
    /// encounters a write error. On error, no items are committed.
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

/// Extension trait for backends that support range queries over time-indexed snapshots.
///
/// Use these methods to query historical market data and portfolio snapshots
/// within a date range or to find the latest snapshot on or before a given date.
///
/// # Examples
///
/// ```rust,no_run
/// # use finstack_io::{SqliteStore, LookbackStore};
/// # use time::macros::date;
/// # async fn example() -> finstack_io::Result<()> {
/// let store = SqliteStore::open("finstack.db").await?;
///
/// // Get all market snapshots for Q1 2025
/// let snapshots = store
///     .list_market_contexts("USD-CURVES", date!(2025-01-01), date!(2025-03-31))
///     .await?;
///
/// // Get the latest snapshot on or before a date
/// let latest = store
///     .latest_market_context_on_or_before("USD-CURVES", date!(2025-02-15))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait LookbackStore: Send + Sync {
    /// List market contexts for a given id in the date range `[start, end]`,
    /// ordered by `as_of` ascending.
    ///
    /// Returns an empty vector if no snapshots exist in the range.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>>;

    /// Get the latest market context with `as_of <= as_of`, if any.
    ///
    /// Returns `None` if no snapshot exists on or before the given date.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>>;

    /// List portfolio specs for a given id in the date range `[start, end]`,
    /// ordered by `as_of` ascending.
    ///
    /// Returns an empty vector if no snapshots exist in the range.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>>;

    /// Get the latest portfolio with `as_of <= as_of`, if any.
    ///
    /// Returns `None` if no snapshot exists on or before the given date.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
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
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_io::{SeriesKey, SeriesKind};
    ///
    /// let key = SeriesKey::new("market", "AAPL", SeriesKind::Quote);
    /// assert_eq!(key.namespace, "market");
    /// assert_eq!(key.series_id, "AAPL");
    /// assert_eq!(key.kind, SeriesKind::Quote);
    /// ```
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
///
/// Time-series are identified by a [`SeriesKey`] (namespace + series_id + kind).
/// Each point has a timestamp, an optional numeric value for quick aggregation,
/// and an optional structured JSON payload.
///
/// # Examples
///
/// ```rust,no_run
/// # use finstack_io::{SqliteStore, TimeSeriesStore, SeriesKey, SeriesKind, TimeSeriesPoint};
/// # use time::OffsetDateTime;
/// # async fn example() -> finstack_io::Result<()> {
/// let store = SqliteStore::open("finstack.db").await?;
/// let key = SeriesKey::new("market", "AAPL", SeriesKind::Quote);
///
/// // Store a batch of points
/// let now = OffsetDateTime::now_utc();
/// store.put_points_batch(&key, &[
///     TimeSeriesPoint { ts: now, value: Some(150.25), payload: None, meta: None },
/// ]).await?;
///
/// // Query a range
/// let points = store.get_points_range(
///     &key,
///     now - time::Duration::hours(1),
///     now,
///     Some(100),
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait TimeSeriesStore: Send + Sync {
    /// Store or update metadata for a time-series key.
    ///
    /// This creates the series entry if it does not exist, or updates its
    /// metadata if it does (upsert).
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a write error.
    async fn put_series_meta(
        &self,
        key: &SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load metadata for a time-series key.
    ///
    /// Returns `None` if the series does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>>;

    /// List series IDs for a namespace and kind.
    ///
    /// Returns an empty vector if no series exist for the given criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>>;

    /// Store multiple points in a single transaction.
    ///
    /// Points are upserted — if a point with the same timestamp already
    /// exists for the series, it is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization of any point fails or the backend
    /// encounters a write error. On error, no points are committed.
    async fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()>;

    /// Load points in the time range `[start, end]`, ordered by timestamp ascending.
    ///
    /// # Arguments
    ///
    /// * `key` - The series to query.
    /// * `start` - Inclusive start of the time range.
    /// * `end` - Inclusive end of the time range.
    /// * `limit` - Optional maximum number of points to return.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn get_points_range(
        &self,
        key: &SeriesKey,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>>;

    /// Get the latest point with `ts <= ts`, if any.
    ///
    /// Returns `None` if no point exists on or before the given timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend encounters a read error.
    async fn latest_point_on_or_before(
        &self,
        key: &SeriesKey,
        ts: OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>>;
}
