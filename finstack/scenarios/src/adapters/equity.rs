//! Equity price shock adapter.
//!
//! This module supports equity price shocks through `OperationSpec::EquityPricePct`.
//! The engine applies equity shocks via `MarketBump::Curve` with `BumpUnits::Percent`,
//! which modifies the price stored in market data scalars.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::types::CurveId;

/// Adapter for equity operations.
pub struct EquityAdapter;

impl ScenarioAdapter for EquityAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::EquityPricePct { ids, pct } => {
                let mut effects = Vec::new();
                for id in ids {
                    if ctx.market.price(id).is_ok() {
                        let bump = MarketBump::Curve {
                            id: CurveId::from(id.as_str()),
                            spec: BumpSpec {
                                mode: BumpMode::Additive,
                                units: BumpUnits::Percent,
                                value: *pct,
                                bump_type: BumpType::Parallel,
                            },
                        };
                        effects.push(ScenarioEffect::MarketBump(bump));
                    } else {
                        effects.push(ScenarioEffect::Warning(format!(
                            "Equity {}: not found in market data",
                            id
                        )));
                    }
                }
                Ok(Some(effects))
            }
            _ => Ok(None),
        }
    }
}
