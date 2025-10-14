//! Equity price shock adapter.
//!
//! Implements the logic behind `OperationSpec::EquityPricePct` by fetching an
//! equity price from market data, applying a percentage change, and writing the
//! shocked value back.

use crate::error::{Error, Result};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::MarketContext;

/// Apply a percentage shock to an equity price stored in market data.
///
/// # Arguments
/// - `market`: Market context that owns the price entry.
/// - `id`: Identifier of the equity price (e.g., ticker or custom key).
/// - `pct`: Percentage change to apply. Positive values increase the price.
///
/// # Returns
/// [`Result`](crate::error::Result) indicating whether the shock succeeded.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the equity price does not exist in the market context.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::equity::apply_equity_shock;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... insert price data via `market.insert_price_mut("SPY", scalar)` ...
/// apply_equity_shock(&mut market, "SPY", -10.0)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_equity_shock(market: &mut MarketContext, id: &str, pct: f64) -> Result<()> {
    // Retrieve existing price
    let existing = market
        .price(id)
        .map_err(|_| Error::MarketDataNotFound { id: id.to_string() })?;

    let new_scalar = match existing {
        MarketScalar::Price(money) => {
            let factor = 1.0 + (pct / 100.0);
            let new_amount = money.amount() * factor;
            MarketScalar::Price(finstack_core::money::Money::new(
                new_amount,
                money.currency(),
            ))
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
