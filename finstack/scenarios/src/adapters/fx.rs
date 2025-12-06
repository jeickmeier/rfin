//! Foreign exchange shock adapter.
//!
//! This module supports FX shocks through the `OperationSpec::MarketFxPct` variant.
//! The engine applies FX shocks via `MarketBump::FxPct` which wraps the existing
//! provider behind a [`BumpedFxProvider`](finstack_core::money::fx::providers::BumpedFxProvider)
//! so the operation remains deterministic and easy to audit.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_core::market_data::bumps::MarketBump;

/// Adapter for FX operations.
pub struct FxAdapter;

impl ScenarioAdapter for FxAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::MarketFxPct { base, quote, pct } => {
                let bump = MarketBump::FxPct {
                    base: *base,
                    quote: *quote,
                    pct: *pct,
                    as_of: ctx.as_of,
                };
                Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
            }
            _ => Ok(None),
        }
    }
}
