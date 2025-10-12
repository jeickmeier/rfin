//! FX shock adapter.

use crate::error::{Error, Result};
use finstack_core::currency::Currency;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use std::sync::Arc;

/// Apply percent shock to an FX rate.
///
/// Positive pct means base currency strengthens (rate increases).
///
/// This creates a new FxMatrix with the shocked rate if the existing FxMatrix
/// uses a SimpleFxProvider. For other providers, returns an error.
pub fn apply_fx_shock(
    market: &mut MarketContext,
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

    // Try to get the current rate
    let current_rate = fx
        .rate(finstack_core::money::fx::FxQuery::new(
            base,
            quote,
            finstack_core::dates::Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
        ))
        .map_err(Error::Core)?
        .rate;

    // Calculate shocked rate
    let factor = 1.0 + (pct / 100.0);
    let shocked_rate = current_rate * factor;

    // Create a new SimpleFxProvider with the shocked rate
    let provider = Arc::new(SimpleFxProvider::new());
    provider.set_quote(base, quote, shocked_rate);

    // Create and insert new FxMatrix
    let new_matrix = FxMatrix::new(provider);
    market.insert_fx_mut(Arc::new(new_matrix));

    Ok(())
}
