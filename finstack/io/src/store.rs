//! Backend-agnostic persistence API.
//!
//! `finstack-io` provides a small, typed repository interface via [`Store`].
//! Storage backends (SQLite, Postgres, filesystem, etc.) implement this trait.

use crate::{Error, Result};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_portfolio::{Portfolio, PortfolioSpec};
use finstack_scenarios::ScenarioSpec;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use std::collections::HashMap;

/// Typed persistence interface for Finstack domain objects.
///
/// Backends should treat `put_*` operations as **idempotent** (upsert) whenever
/// the underlying store supports it.
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
    fn get_market_context(&self, market_id: &str, as_of: Date) -> Result<Option<MarketContext>>;

    /// Store an instrument definition.
    fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load an instrument definition.
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
    fn get_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<Option<PortfolioSpec>>;

    /// Store a scenario specification.
    fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a scenario specification.
    fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>>;

    /// Store a statements model specification.
    fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()>;

    /// Load a statements model specification.
    fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>>;

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

    /// Load and hydrate a portfolio for valuation/aggregation.
    ///
    /// Hydration rule:
    /// - If a position's `instrument_spec` is `None`, resolve it from the
    ///   instruments registry using `instrument_id`.
    fn load_portfolio(&self, portfolio_id: &str, as_of: Date) -> Result<Portfolio> {
        let mut spec = self.load_portfolio_spec(portfolio_id, as_of)?;

        // Resolve missing instrument specs from the instrument registry.
        let mut cache: HashMap<String, InstrumentJson> = HashMap::new();
        for pos in &mut spec.positions {
            if pos.instrument_spec.is_some() {
                continue;
            }

            let instrument_id = pos.instrument_id.clone();
            let resolved = if let Some(instr) = cache.get(&instrument_id) {
                instr.clone()
            } else {
                let instr = self
                    .get_instrument(&instrument_id)?
                    .ok_or_else(|| Error::not_found("instrument", instrument_id.clone()))?;
                cache.insert(instrument_id.clone(), instr.clone());
                instr
            };

            pos.instrument_spec = Some(resolved);
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

/// A time-indexed market context snapshot returned from a lookback query.
#[derive(Clone)]
pub struct MarketContextSnapshot {
    /// As-of key (ISO date string, e.g. `2024-01-01`).
    pub as_of: String,
    /// Market context snapshot.
    pub context: MarketContext,
}

/// A time-indexed portfolio snapshot returned from a lookback query.
#[derive(Debug, Clone)]
pub struct PortfolioSnapshot {
    /// As-of key (ISO date string, e.g. `2024-01-01`).
    pub as_of: String,
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
}
