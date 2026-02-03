//! Backend-agnostic persistence API.
//!
//! `finstack-io` provides a small, typed repository interface via [`Store`].
//! Storage backends (SQLite, Postgres, filesystem, etc.) implement this trait.

use crate::{Error, Result};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_portfolio::{Portfolio, PortfolioSpec};
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use std::collections::HashMap;
use std::sync::Arc;

/// Typed persistence interface for Finstack domain objects.
///
/// Backends should treat `put_*` operations as **idempotent** (upsert) whenever
/// the underlying store supports it.
///
/// ## Transaction Isolation
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
pub trait Store {
    /// Store a market context snapshot for a given `as_of` date.
    fn put_market_context(
        &self,
        market_id: &str,
        as_of: Date,
        context: &MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a market context snapshot for a given `as_of` date.
    #[must_use = "this returns the market context, which should be used"]
    fn get_market_context(&self, market_id: &str, as_of: Date) -> Result<Option<MarketContext>>;

    /// Store an instrument definition.
    fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load an instrument definition.
    #[must_use = "this returns the instrument, which should be used"]
    fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>>;

    /// Store a portfolio snapshot for a given `as_of` date.
    fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a portfolio snapshot for a given `as_of` date.
    #[must_use = "this returns the portfolio spec, which should be used"]
    fn get_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<Option<PortfolioSpec>>;

    /// Store a scenario specification.
    fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a scenario specification.
    #[must_use = "this returns the scenario spec, which should be used"]
    fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>>;

    /// Store a statements model specification.
    fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a statements model specification.
    #[must_use = "this returns the statement model spec, which should be used"]
    fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>>;

    /// Store a metric registry by namespace.
    ///
    /// Metric registries define reusable financial metrics (ratios, KPIs) that can be
    /// shared across multiple statement models. The namespace (e.g., "fin", "custom")
    /// serves as the primary key.
    fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a metric registry by namespace.
    #[must_use = "this returns the metric registry, which should be used"]
    fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>>;

    /// List all stored metric registry namespaces.
    #[must_use = "this returns the list of namespaces, which should be used"]
    fn list_metric_registries(&self) -> Result<Vec<String>>;

    /// Delete a metric registry by namespace.
    fn delete_metric_registry(&self, namespace: &str) -> Result<bool>;

    /// Load a market context snapshot, returning a not-found error if missing.
    fn load_market_context(&self, market_id: &str, as_of: Date) -> Result<MarketContext> {
        self.get_market_context(market_id, as_of)?
            .ok_or_else(|| Error::not_found("market_context", format!("{market_id}@{as_of}")))
    }

    /// Load a portfolio spec snapshot, returning a not-found error if missing.
    fn load_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<PortfolioSpec> {
        self.get_portfolio_spec(portfolio_id, as_of)?
            .ok_or_else(|| Error::not_found("portfolio", format!("{portfolio_id}@{as_of}")))
    }

    /// Load a metric registry, returning a not-found error if missing.
    fn load_metric_registry(&self, namespace: &str) -> Result<MetricRegistry> {
        self.get_metric_registry(namespace)?
            .ok_or_else(|| Error::not_found("metric_registry", namespace))
    }

    /// Load and hydrate a portfolio for valuation/aggregation.
    ///
    /// Hydration rule:
    /// - If a position's `instrument_spec` is `None`, resolve it from the
    ///   instruments registry using `instrument_id`.
    ///
    /// Note: This method performs multiple database reads without transaction isolation.
    /// See the trait-level documentation for details on consistency guarantees.
    fn load_portfolio(&self, portfolio_id: &str, as_of: Date) -> Result<Portfolio> {
        let mut spec = self.load_portfolio_spec(portfolio_id, as_of)?;

        // Resolve missing instrument specs from the instrument registry.
        // Use Arc to avoid cloning large instrument definitions when the same
        // instrument appears in multiple positions.
        let mut cache: HashMap<String, Arc<InstrumentJson>> = HashMap::new();
        for pos in &mut spec.positions {
            if pos.instrument_spec.is_some() {
                continue;
            }

            let instrument_id = &pos.instrument_id;
            let resolved = if let Some(instr) = cache.get(instrument_id) {
                Arc::clone(instr)
            } else {
                let instr = self
                    .get_instrument(instrument_id)?
                    .ok_or_else(|| Error::not_found("instrument", instrument_id.clone()))?;
                let instr = Arc::new(instr);
                cache.insert(instrument_id.clone(), Arc::clone(&instr));
                instr
            };

            // Unwrap the Arc for assignment (positions don't share ownership)
            pos.instrument_spec =
                Some(Arc::try_unwrap(resolved).unwrap_or_else(|arc| (*arc).clone()));
        }

        Ok(Portfolio::from_spec(spec)?)
    }

    /// Convenience helper: load a portfolio and matching market context for the same `as_of`.
    fn load_portfolio_with_market(
        &self,
        portfolio_id: &str,
        market_id: &str,
        as_of: Date,
    ) -> Result<(Portfolio, MarketContext)> {
        let portfolio = self.load_portfolio(portfolio_id, as_of)?;
        let market = self.load_market_context(market_id, as_of)?;
        Ok((portfolio, market))
    }
}

/// Extension trait for bulk operations.
///
/// Bulk methods execute within a single transaction for atomicity and better performance
/// when inserting many records.
pub trait BulkStore: Store {
    /// Store multiple instruments in a single transaction.
    ///
    /// This is more efficient than calling `put_instrument` repeatedly and provides
    /// atomicity (all-or-nothing).
    fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()>;

    /// Store multiple market contexts in a single transaction.
    ///
    /// This is more efficient than calling `put_market_context` repeatedly and provides
    /// atomicity (all-or-nothing).
    fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()>;

    /// Store multiple portfolio specs in a single transaction.
    fn put_portfolio_specs_batch(
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
pub trait LookbackStore {
    /// List market contexts for a given id in `[start, end]`, ordered by `as_of`.
    fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>>;

    /// Get the latest market context with `as_of <= as_of`, if any.
    fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>>;

    /// List portfolio specs for a given id in `[start, end]`, ordered by `as_of`.
    fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>>;

    /// Get the latest portfolio with `as_of <= as_of`, if any.
    fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>>;
}
