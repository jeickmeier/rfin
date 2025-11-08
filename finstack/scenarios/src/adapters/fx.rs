//! Foreign exchange shock adapter.
//!
//! Provides helper functions used by `OperationSpec::MarketFxPct` to apply
//! multiplicative shocks to FX matrices held inside the market context. The
//! adapter wraps the existing provider behind a [`BumpedFxProvider`] so the
//! operation remains deterministic and easy to audit.

use crate::error::{Error, Result};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::providers::BumpedFxProvider;
use finstack_core::money::fx::FxMatrix;
use std::sync::Arc;

/// Apply a percentage shock to an FX rate.
///
/// Positive percentages strengthen the base currency (increase the quoted rate).
/// The function wraps the existing FX provider with a [`BumpedFxProvider`] so the
/// shocked pair is overridden while every other rate continues to delegate to the
/// original matrix configuration.
///
/// # Arguments
/// - `market`: Market context whose FX matrix will be shocked.
/// - `as_of`: Valuation date for querying the FX rate.
/// - `base`: Base currency of the rate to shock.
/// - `quote`: Quote currency of the rate to shock.
/// - `pct`: Percentage change to apply (e.g., `5.0` means +5%).
///
/// # Returns
/// [`Result`](crate::error::Result) indicating success. The Ok variant is unit
/// typed because the changes are applied directly to `market`.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   no FX matrix is present.
/// - [`Error::Core`](crate::error::Error::Core) if retrieving the existing rate
///   fails.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::fx::apply_fx_shock;
/// use finstack_core::currency::Currency;
/// use finstack_core::market_data::MarketContext;
/// use time::macros::date;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// let as_of = date!(2025 - 01 - 01);
/// // ... populate `market` with an FX matrix ...
/// apply_fx_shock(&mut market, as_of, Currency::USD, Currency::EUR, 2.5)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_fx_shock(
    market: &mut MarketContext,
    as_of: Date,
    base: Currency,
    quote: Currency,
    pct: f64,
) -> Result<()> {
    // Get the current FX matrix
    let fx = market
        .fx
        .as_ref()
        .ok_or_else(|| Error::MarketDataNotFound {
            id: "FX matrix".to_string(),
        })?;

    // Try to get the current rate using the provided as_of date
    let current_rate = fx
        .rate(finstack_core::money::fx::FxQuery::new(base, quote, as_of))
        .map_err(Error::Core)?
        .rate;

    // Calculate shocked rate
    let factor = 1.0 + (pct / 100.0);
    let shocked_rate = current_rate * factor;

    // Wrap the original provider so only the requested pair is bumped and all
    // other quotes continue to delegate to the existing source.
    let bumped_provider = Arc::new(BumpedFxProvider::new(
        fx.provider(),
        base,
        quote,
        shocked_rate,
    ));

    // Recreate the matrix with the previous configuration to preserve cache
    // sizing and triangulation behaviour.
    let new_matrix = FxMatrix::with_config(bumped_provider, fx.config());
    market.insert_fx_mut(Arc::new(new_matrix));

    Ok(())
}
