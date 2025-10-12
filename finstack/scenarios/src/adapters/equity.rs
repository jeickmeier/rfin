//! Equity price shock adapter.

use crate::error::{Error, Result};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::MarketContext;

/// Apply percent shock to an equity price.
///
/// Retrieves the price from MarketContext, applies multiplicative shock,
/// and re-inserts the updated price.
pub fn apply_equity_shock(market: &mut MarketContext, id: &str, pct: f64) -> Result<()> {
    // Retrieve existing price
    let existing = market
        .price(id)
        .map_err(|_| Error::MarketDataNotFound { id: id.to_string() })?;

    let new_scalar = match existing {
        MarketScalar::Price(money) => {
            let factor = 1.0 + (pct / 100.0);
            let new_amount = money.amount() * factor;
            MarketScalar::Price(finstack_core::money::Money::new(new_amount, money.currency()))
        }
        MarketScalar::Unitless(val) => {
            // If stored as unitless, apply shock
            let factor = 1.0 + (pct / 100.0);
            MarketScalar::Unitless(val * factor)
        }
    };

    market.insert_price_mut(id, new_scalar);
    Ok(())
}

