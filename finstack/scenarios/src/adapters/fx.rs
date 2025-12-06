//! Foreign exchange shock adapter.
//!
//! This module supports FX shocks through the `OperationSpec::MarketFxPct` variant.
//! The engine applies FX shocks via `MarketBump::FxPct` which wraps the existing
//! provider behind a [`BumpedFxProvider`](finstack_core::money::fx::providers::BumpedFxProvider)
//! so the operation remains deterministic and easy to audit.

use crate::error::Result;
use finstack_core::currency::Currency;
use finstack_core::market_data::bumps::MarketBump;

/// Apply FX percentage shocks.
///
/// Schedules a `MarketBump::FxPct` operation.
///
/// # Arguments
/// - `base`: Base currency of the pair.
/// - `quote`: Quote currency of the pair.
/// - `pct`: Percentage shock to apply.
/// - `as_of`: Valuation date for the shock.
/// - `market_bumps`: Output vector to append scheduled bumps to.
///
/// # Returns
/// Number of bumps applied (always 1).
pub fn apply_fx_shock(
    base: Currency,
    quote: Currency,
    pct: f64,
    as_of: time::Date,
    market_bumps: &mut Vec<MarketBump>,
) -> Result<usize> {
    market_bumps.push(MarketBump::FxPct {
        base,
        quote,
        pct,
        as_of,
    });
    Ok(1)
}
