//! Equity price shock adapter.
//!
//! This module supports equity price shocks through `OperationSpec::EquityPricePct`.
//! The engine applies equity shocks via `MarketBump::Curve` with `BumpUnits::Percent`,
//! which modifies the price stored in market data scalars.

use crate::error::Result;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;

/// Apply equity price percentage shocks.
///
/// Iterates over provided equity identifiers, verifies their existence in the market context,
/// and schedules a corresponding market bump.
///
/// # Arguments
/// - `market`: The market context to query for existence.
/// - `ids`: List of equity identifiers to shock.
/// - `pct`: Percentage shock to apply (e.g. -5.0 for -5%).
/// - `market_bumps`: Output vector to append scheduled bumps to.
/// - `warnings`: Output vector to append warnings to.
///
/// # Returns
/// Number of bumps applied.
pub fn apply_equity_price_shock(
    market: &MarketContext,
    ids: &[String],
    pct: f64,
    market_bumps: &mut Vec<MarketBump>,
    warnings: &mut Vec<String>,
) -> Result<usize> {
    let mut applied = 0;
    for id in ids {
        if market.price(id).is_ok() {
            market_bumps.push(MarketBump::Curve {
                id: CurveId::from(id.as_str()),
                spec: BumpSpec {
                    mode: BumpMode::Additive,
                    units: BumpUnits::Percent,
                    value: pct,
                    bump_type: BumpType::Parallel,
                },
            });
            applied += 1;
        } else {
            warnings.push(format!("Equity {}: not found in market data", id));
        }
    }
    Ok(applied)
}
